use std::time::Instant;
use crate::sound::play_completion_sound;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    Focus,
    ShortBreak,
    LongBreak,
}

impl TimerMode {
    pub fn title(&self) -> &'static str {
        match self {
            TimerMode::Focus => "Focus",
            TimerMode::ShortBreak => "Short Break",
            TimerMode::LongBreak => "Long Break",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            TimerMode::Focus => "STAY IN THE ZONE",
            TimerMode::ShortBreak => "REST & RECHARGE",
            TimerMode::LongBreak => "DEEP RESTORATION",
        }
    }
}

/// A phase that ran to completion.
pub struct Finished {
    pub mode: TimerMode,
    pub secs: f64,
}

pub struct PomodoroTimer {
    pub mode: TimerMode,
    pub is_running: bool,
    pub remaining_seconds: f64,
    pub total_seconds: f64,
    pub last_tick: Option<Instant>,

    // Session streaks (0 to 4 in current cycle)
    pub completed_sessions: u32,
    pub cycle_goal: u32,
    pub total_focus_sessions_today: u32,
    pub total_focus_seconds_today: f64,

    // Settings
    pub focus_duration_mins: u32,
    pub short_break_mins: u32,
    pub long_break_mins: u32,
    pub sound_enabled: bool,
    pub auto_start_breaks: bool,
    pub auto_start_focus: bool,
}

impl Default for PomodoroTimer {
    fn default() -> Self {
        let focus_mins = 25;
        let short_mins = 5;
        let long_mins = 15;
        let total = (focus_mins * 60) as f64;

        Self {
            mode: TimerMode::Focus,
            is_running: false,
            remaining_seconds: total,
            total_seconds: total,
            last_tick: None,
            completed_sessions: 0,
            cycle_goal: 4,
            total_focus_sessions_today: 0,
            total_focus_seconds_today: 0.0,
            focus_duration_mins: focus_mins,
            short_break_mins: short_mins,
            long_break_mins: long_mins,
            sound_enabled: true,
            auto_start_breaks: false,
            auto_start_focus: false,
        }
    }
}

impl PomodoroTimer {
    pub fn duration_for_mode(&self, mode: TimerMode) -> f64 {
        match mode {
            TimerMode::Focus => (self.focus_duration_mins * 60) as f64,
            TimerMode::ShortBreak => (self.short_break_mins * 60) as f64,
            TimerMode::LongBreak => (self.long_break_mins * 60) as f64,
        }
    }

    pub fn set_mode(&mut self, new_mode: TimerMode) {
        self.mode = new_mode;
        self.is_running = false;
        self.last_tick = None;
        let dur = self.duration_for_mode(new_mode);
        self.total_seconds = dur;
        self.remaining_seconds = dur;
    }

    pub fn start(&mut self) {
        if self.remaining_seconds <= 0.0 {
            self.remaining_seconds = self.total_seconds;
        }
        self.is_running = true;
        self.last_tick = Some(Instant::now());
    }

    pub fn pause(&mut self) {
        self.is_running = false;
        self.last_tick = None;
    }

    pub fn toggle_run(&mut self) {
        if self.is_running {
            self.pause();
        } else {
            self.start();
        }
    }

    pub fn reset(&mut self) {
        self.is_running = false;
        self.last_tick = None;
        self.remaining_seconds = self.total_seconds;
    }

    pub fn add_seconds(&mut self, secs: f64) {
        self.remaining_seconds = (self.remaining_seconds + secs).clamp(0.0, 7200.0);
        if self.remaining_seconds > self.total_seconds {
            self.total_seconds = self.remaining_seconds;
        }
    }

    pub fn formatted_time(&self) -> String {
        let total_secs = self.remaining_seconds.ceil().max(0.0) as u64;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    pub fn formatted_focus_time_today(&self) -> String {
        let total_secs = self.total_focus_seconds_today as u64;
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }

    /// Updates the timer based on actual elapsed time.
    /// Returns true if a session completed this tick.
    pub fn update(&mut self) -> Option<Finished> {
        if !self.is_running {
            return None;
        }

        let now = Instant::now();
        if let Some(last) = self.last_tick {
            let elapsed = now.duration_since(last).as_secs_f64();
            self.remaining_seconds -= elapsed;
        }
        self.last_tick = Some(now);

        if self.remaining_seconds <= 0.0 {
            self.remaining_seconds = 0.0;
            self.is_running = false;
            self.last_tick = None;

            if self.sound_enabled {
                play_completion_sound();
            }

            let finished = Finished { mode: self.mode, secs: self.total_seconds };
            self.handle_session_completed();
            return Some(finished);
        }

        None
    }

    fn handle_session_completed(&mut self) {
        match self.mode {
            TimerMode::Focus => {
                self.completed_sessions += 1;
                self.total_focus_sessions_today += 1;

                if self.completed_sessions >= self.cycle_goal {
                    self.completed_sessions = 0;
                    self.set_mode(TimerMode::LongBreak);
                } else {
                    self.set_mode(TimerMode::ShortBreak);
                }

                if self.auto_start_breaks {
                    self.start();
                }
            }
            TimerMode::ShortBreak | TimerMode::LongBreak => {
                self.set_mode(TimerMode::Focus);
                if self.auto_start_focus {
                    self.start();
                }
            }
        }
    }

    /// Skip to the next phase without crediting the current one.
    pub fn skip_to_next(&mut self) {
        match self.mode {
            TimerMode::Focus => self.set_mode(TimerMode::ShortBreak),
            _ => self.set_mode(TimerMode::Focus),
        }
    }

    pub fn apply_settings(
        &mut self,
        focus_mins: u32,
        short_mins: u32,
        long_mins: u32,
        sound: bool,
        auto_breaks: bool,
        auto_focus: bool,
    ) {
        let old_focus = self.focus_duration_mins;
        let old_short = self.short_break_mins;
        let old_long = self.long_break_mins;

        self.focus_duration_mins = focus_mins.clamp(1, 120);
        self.short_break_mins = short_mins.clamp(1, 60);
        self.long_break_mins = long_mins.clamp(1, 90);
        self.sound_enabled = sound;
        self.auto_start_breaks = auto_breaks;
        self.auto_start_focus = auto_focus;

        if !self.is_running {
            let changed = match self.mode {
                TimerMode::Focus => old_focus != self.focus_duration_mins,
                TimerMode::ShortBreak => old_short != self.short_break_mins,
                TimerMode::LongBreak => old_long != self.long_break_mins,
            };
            if changed {
                let dur = self.duration_for_mode(self.mode);
                self.total_seconds = dur;
                self.remaining_seconds = dur;
            }
        }
    }
}
