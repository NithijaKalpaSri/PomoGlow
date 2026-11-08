//! Persistent user data (profile, settings, history) and small date helpers.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ---------- Avatar palettes (shared look with the web app) ----------
pub const SKINS: [[u8; 3]; 6] = [
    [255, 224, 200],
    [247, 205, 170],
    [226, 169, 128],
    [190, 128, 90],
    [140, 92, 64],
    [98, 64, 46],
];
pub const HAIRS: [[u8; 3]; 6] = [
    [120, 78, 58],
    [58, 40, 34],
    [214, 160, 80],
    [196, 90, 70],
    [120, 100, 170],
    [90, 90, 96],
];
pub const PHONES: [[u8; 3]; 6] = [
    [240, 150, 160],
    [120, 190, 170],
    [246, 190, 90],
    [130, 170, 230],
    [170, 140, 220],
    [236, 120, 90],
];
pub const BGS: [[u8; 3]; 6] = [
    [253, 226, 214],
    [214, 234, 220],
    [252, 238, 204],
    [212, 228, 244],
    [232, 220, 244],
    [244, 224, 232],
];
/// Theme accents: (sidebar tint, accent)
pub const THEMES: [(&str, [u8; 3], [u8; 3]); 4] = [
    ("Sage", [190, 220, 206], [86, 150, 128]),
    ("Peach", [250, 212, 196], [226, 112, 82]),
    ("Lavender", [220, 210, 240], [140, 110, 200]),
    ("Sky", [200, 222, 242], [76, 130, 200]),
];
pub const STYLE_NAMES: [&str; 3] = ["Bun", "Long", "Short"];

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub tagline: String,
    pub daily_goal: u32,
    pub skin: usize,
    pub hair: usize,
    pub phones: usize,
    pub bg: usize,
    pub style: usize,
    pub theme: usize,
    /// 0 = no avatar, 1 = cartoon, 2 = photo
    pub mode: u8,
    /// 0 = female, 1 = male (cartoon avatar)
    pub gender: u8,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: String::new(),
            tagline: String::new(),
            daily_goal: 6,
            skin: 1,
            hair: 0,
            phones: 0,
            bg: 0,
            style: 0,
            theme: 0,
            mode: 0,
            gender: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub focus: u32,
    pub short: u32,
    pub long: u32,
    pub sound: bool,
    pub auto_breaks: bool,
    pub auto_focus: bool,
    pub dark: bool,
    pub clock_style: u8,
    pub overlay_style: u8,
    pub overlay_size: f32,
    pub overlay_opacity: f32,
    pub click_through: bool,
    pub overlay_color: u8,
    pub overlay_pos: u8,
    pub overlay_label: bool,
    pub overlay_progress: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { focus: 25, short: 5, long: 15, sound: true, auto_breaks: false, auto_focus: false, dark: false, clock_style: 0, overlay_style: 4, overlay_size: 1.0, overlay_opacity: 0.85, click_through: true, overlay_color: 0, overlay_pos: 0, overlay_label: true, overlay_progress: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DayStat {
    pub sessions: u32,
    pub focus_mins: u32,
    /// Minutes by period: morning, afternoon, evening, night
    pub periods: [u32; 4],
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionLog {
    pub date: String,
    pub time: String,
    pub mins: u32,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SaveData {
    pub onboarded: bool,
    pub profile: Profile,
    pub settings: Settings,
    pub days: BTreeMap<String, DayStat>,
    pub log: Vec<SessionLog>,
}

fn data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
    let dir = PathBuf::from(base).join("Pomoglow");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn data_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("data.json"))
}

pub fn avatar_file() -> Option<PathBuf> {
    data_dir().map(|d| d.join("avatar.png"))
}

impl SaveData {
    pub fn load() -> Self {
        data_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let (Some(p), Ok(s)) = (data_path(), serde_json::to_string_pretty(self)) {
            let _ = std::fs::write(p, s);
        }
    }

    pub fn record_focus(&mut self, mins: u32, label: &str) {
        let now = now_local();
        let key = date_key(now.days);
        let d = self.days.entry(key.clone()).or_default();
        d.sessions += 1;
        d.focus_mins += mins;
        d.periods[period_of(now.hour)] += mins;
        self.log.push(SessionLog {
            date: key,
            time: format!("{:02}:{:02}", now.hour, now.min),
            mins,
            label: if label.trim().is_empty() { "Focus session".into() } else { label.trim().into() },
        });
        if self.log.len() > 200 {
            self.log.remove(0);
        }
    }

    pub fn day(&self, days: i64) -> DayStat {
        self.days.get(&date_key(days)).cloned().unwrap_or_default()
    }

    pub fn streak(&self) -> u32 {
        let today = now_local().days;
        let mut d = if self.day(today).sessions > 0 { today } else { today - 1 };
        let mut n = 0;
        while self.day(d).sessions > 0 {
            n += 1;
            d -= 1;
        }
        n
    }
}

pub fn period_of(hour: u32) -> usize {
    match hour {
        5..=11 => 0,
        12..=16 => 1,
        17..=21 => 2,
        _ => 3,
    }
}

// ---------- Time helpers ----------
#[derive(Clone, Copy)]
pub struct LocalTime {
    /// Days since 1970-01-01 (local)
    pub days: i64,
    pub hour: u32,
    pub min: u32,
    /// 0 = Monday .. 6 = Sunday
    pub weekday: usize,
}

pub fn now_local() -> LocalTime {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };
        let days = days_from_civil(st.wYear as i64, st.wMonth as i64, st.wDay as i64);
        return LocalTime {
            days,
            hour: st.wHour as u32,
            min: st.wMinute as u32,
            weekday: ((st.wDayOfWeek as usize) + 6) % 7,
        };
    }
    #[allow(unreachable_code)]
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        let days = secs.div_euclid(86400);
        let rem = secs.rem_euclid(86400);
        LocalTime {
            days,
            hour: (rem / 3600) as u32,
            min: ((rem % 3600) / 60) as u32,
            weekday: ((days + 3).rem_euclid(7)) as usize,
        }
    }
}

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn date_key(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

pub const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
pub const MONTHS: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub fn pretty_date(t: &LocalTime) -> String {
    let (_, m, d) = civil_from_days(t.days);
    format!("{}, {} {}", WEEKDAYS[t.weekday], MONTHS[(m - 1) as usize], d)
}

pub fn greeting(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good Morning",
        12..=16 => "Good Afternoon",
        17..=21 => "Good Evening",
        _ => "Good Night",
    }
}
