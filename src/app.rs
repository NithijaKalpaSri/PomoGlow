use crate::clock::{draw_clock, ClockData, STYLE_NAMES};

const OV_COLORS: [[u8; 3]; 6] = [[255, 255, 255], [255, 130, 100], [120, 230, 180], [130, 190, 255], [190, 160, 255], [255, 210, 90]];
const OV_POS: [&str; 7] = ["Center", "Top", "Bottom", "Top-Left", "Top-Right", "Bottom-Left", "Bottom-Right"];
use crate::kit::*;
use crate::tray::{Tray, TrayAction};
use crate::store::avatar_file;
use crate::store::*;
use crate::timer::{PomodoroTimer, TimerMode};
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Rect, Sense, Shape, Stroke, Ui, UiBuilder};
use std::f32::consts::PI;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Dashboard,
    Timer,
    Clock,
    Profile,
    Settings,
}

pub struct PomoglowApp {
    data: SaveData,
    timer: PomodoroTimer,
    page: Page,
    task: String,
    always_on_top: bool,
    dirty: bool,
    last_save: Instant,
    toast: Option<(String, Instant)>,
    confirm_reset: bool,
    last_title: String,
    applied_dark: bool,
    overlay_on: bool,
    frames: u32,
    tray: Option<Tray>,
    photo_tex: Option<egui::TextureHandle>,
}

const BAR_COLORS: [[u8; 3]; 7] = [
    [244, 176, 150],
    [250, 208, 120],
    [160, 200, 170],
    [236, 128, 96],
    [140, 176, 226],
    [120, 184, 156],
    [200, 170, 230],
];
const PERIOD_COLORS: [[u8; 3]; 4] = [[240, 160, 130], [244, 200, 110], [150, 200, 170], [140, 176, 226]];
const PERIOD_NAMES: [&str; 4] = ["Morning", "Afternoon", "Evening", "Night"];

impl PomoglowApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let data = SaveData::load();
        let mut timer = PomodoroTimer::default();
        let s = data.settings.clone();
        timer.apply_settings(s.focus, s.short, s.long, s.sound, s.auto_breaks, s.auto_focus);
        timer.set_mode(TimerMode::Focus);
        let mut app = Self {
            data,
            timer,
            page: match std::env::var("POMOGLOW_PAGE").as_deref() {
                Ok("timer") => Page::Timer,
                Ok("clock") => Page::Clock,
                Ok("profile") => Page::Profile,
                Ok("settings") => Page::Settings,
                _ => Page::Dashboard,
            },
            task: String::new(),
            always_on_top: false,
            dirty: false,
            last_save: Instant::now(),
            toast: None,
            confirm_reset: false,
            last_title: String::new(),
            applied_dark: false,
            overlay_on: false,
            frames: 0,
            tray: Tray::new(&_cc.egui_ctx),
            photo_tex: None,
        };
        if app.data.profile.mode == 2 {
            app.load_photo(&_cc.egui_ctx);
        }
        app
    }

    fn load_photo(&mut self, ctx: &egui::Context) {
        self.photo_tex = avatar_file().and_then(|f| image::open(f).ok()).map(|img| {
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            ctx.load_texture("avatar", egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), Default::default())
        });
    }

    fn pick_photo(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp"]).pick_file() else { return };
        let Ok(img) = image::open(path) else {
            self.say("Could not open that image");
            return;
        };
        let img = img.resize_to_fill(256, 256, image::imageops::FilterType::Lanczos3);
        if let Some(f) = avatar_file() {
            let _ = img.save(f);
        }
        self.load_photo(ctx);
        self.data.profile.mode = 2;
        self.dirty = true;
    }

    fn av(&self, p: &Painter, c: egui::Pos2, r: f32) {
        match (self.data.profile.mode, &self.photo_tex) {
            (2, Some(t)) => avatar_photo(p, c, r, t.id()),
            (1, _) => avatar(p, c, r, &self.data.profile),
            _ => avatar_empty(p, c, r),
        }
    }

    fn accent(&self) -> Color32 {
        match self.timer.mode {
            TimerMode::Focus => CORAL,
            TimerMode::ShortBreak => Color32::from_rgb(96, 172, 142),
            TimerMode::LongBreak => Color32::from_rgb(150, 120, 212),
        }
    }

    fn theme(&self) -> (Color32, Color32) {
        let t = THEMES[self.data.profile.theme % 4];
        (rgb(t.1), rgb(t.2))
    }

    fn today(&self) -> DayStat {
        self.data.day(now_local().days)
    }

    fn week_stats(&self) -> [DayStat; 7] {
        let now = now_local();
        let monday = now.days - now.weekday as i64;
        std::array::from_fn(|i| self.data.day(monday + i as i64))
    }

    fn say(&mut self, msg: &str) {
        self.toast = Some((msg.to_string(), Instant::now()));
    }
}

/// Makes the overlay window colour-keyed (pure black = see-through), click-through, hidden from
/// Alt-Tab, and places it on the screen according to the chosen preset.
#[cfg(windows)]
static OV_LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(windows)]
fn overlay_closed() {
    OV_LAST.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(windows)]
fn style_overlay(click_through: bool, preset: u8, alpha: u8) {
    use std::sync::atomic::Ordering;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let title: Vec<u16> = "Pomoglow Overlay\0".encode_utf16().collect();
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd.is_null() {
            return;
        }
        let mut st = GetWindowLongPtrW(hwnd, GWL_STYLE);
        st &= !((WS_CAPTION | WS_THICKFRAME | WS_BORDER | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX) as isize);
        st |= WS_POPUP as isize;
        SetWindowLongPtrW(hwnd, GWL_STYLE, st);
        let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        ex &= !((WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE | WS_EX_DLGMODALFRAME) as isize);
        ex |= (WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) as isize;
        if click_through {
            ex |= WS_EX_TRANSPARENT as isize;
        } else {
            ex &= !(WS_EX_TRANSPARENT as isize);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
        SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_COLORKEY | LWA_ALPHA);
        // no DWM border / rounded corners / shadow line around the overlay
        let none: u32 = 0xFFFF_FFFE; // DWMWA_COLOR_NONE
        let (donotround, ncoff): (i32, i32) = (1, 1);
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, 34, &none as *const _ as *const _, 4);
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, 33, &donotround as *const _ as *const _, 4);
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, 2, &ncoff as *const _ as *const _, 4);

        let mut rc: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut rc);
        let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
        let key = ((preset as u64 + 1) << 48) | ((w as u64 & 0xFFFFFF) << 24) | (h as u64 & 0xFFFFFF);
        if OV_LAST.load(Ordering::Relaxed) != key {
            let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
            let (m, mb) = (50, 90);
            let (cx, cy) = ((sw - w) / 2, (sh - h) / 2);
            let (rx, by) = (sw - w - m, sh - h - mb);
            let (x, y) = match preset {
                1 => (cx, m),
                2 => (cx, by),
                3 => (m, m),
                4 => (rx, m),
                5 => (m, by),
                6 => (rx, by),
                _ => (cx, cy),
            };
            SetWindowPos(hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
            // clip a few px off every edge so no system border/line can show around the clock
            let rgn = windows_sys::Win32::Graphics::Gdi::CreateRectRgn(3, 4, w - 3, h - 3);
            windows_sys::Win32::Graphics::Gdi::SetWindowRgn(hwnd, rgn, 1);
            OV_LAST.store(key, Ordering::Relaxed);
        }
    }
}

fn split(r: Rect, weights: &[f32], gap: f32) -> Vec<Rect> {
    let total: f32 = weights.iter().sum();
    let avail = r.width() - gap * (weights.len() as f32 - 1.0);
    let mut x = r.left();
    weights
        .iter()
        .map(|w| {
            let wd = avail * w / total;
            let out = Rect::from_min_size(pos2(x, r.top()), vec2(wd, r.height()));
            x += wd + gap;
            out
        })
        .collect()
}

fn ring(p: &Painter, c: egui::Pos2, r: f32, w: f32, ratio: f32, col: Color32, track: Color32) {
    p.circle_stroke(c, r, Stroke::new(w, track));
    if ratio > 0.002 {
        let pts = arc_points(c, r, -PI * 0.5, -PI * 0.5 + 2.0 * PI * ratio);
        p.add(Shape::line(pts.clone(), Stroke::new(w + 8.0, alpha(col, 40))));
        p.add(Shape::line(pts.clone(), Stroke::new(w, col)));
        p.circle_filled(pts[0], w * 0.5, col);
        p.circle_filled(*pts.last().unwrap(), w * 0.5, col);
    }
}

fn leaf(p: &Painter, base: egui::Pos2, ang: f32, len: f32, col: Color32) {
    let (s, c) = ang.sin_cos();
    let pts: Vec<egui::Pos2> = (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0 * 2.0 * PI;
            let (lx, ly) = (t.cos() * len * 0.5 + len * 0.5, t.sin() * len * 0.22 * (1.0 - 0.3 * t.cos()));
            base + vec2(lx * c - ly * s, lx * s + ly * c)
        })
        .collect();
    p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
}

fn toggle(ui: &mut Ui, id: &str, c: egui::Pos2, on: &mut bool, accent: Color32) -> bool {
    let rect = Rect::from_center_size(c, vec2(48.0, 26.0));
    let resp = ui.interact(rect, Id::new(id), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        *on = !*on;
    }
    let p = ui.painter();
    p.rect_filled(rect, radius(13.0), if *on { accent } else { Color32::from_rgb(214, 204, 194) });
    let kx = if *on { rect.right() - 13.0 } else { rect.left() + 13.0 };
    p.circle_filled(pos2(kx, c.y + 1.0), 10.0, alpha(Color32::BLACK, 30));
    p.circle_filled(pos2(kx, c.y), 10.0, Color32::WHITE);
    resp.clicked()
}

impl eframe::App for PomoglowApp {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        set_dark(self.data.settings.dark);
        if self.applied_dark != self.data.settings.dark {
            ctx.set_visuals(visuals());
            self.applied_dark = self.data.settings.dark;
        }

        if self.timer.is_running {
            if let Some(f) = self.timer.update() {
                if f.mode == TimerMode::Focus {
                    self.data.record_focus((f.secs / 60.0).round().max(1.0) as u32, &self.task);
                    self.dirty = true;
                    self.say("Focus session complete. Take a break!");
                } else {
                    self.say("Break is over. Ready to focus?");
                }
            }
            ctx.request_repaint_after(Duration::from_millis(100));
            let title = format!("{} - Pomoglow", self.timer.formatted_time());
            if title != self.last_title {
                if let Some(t) = &self.tray {
                    t.set_tooltip(&title);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
                self.last_title = title;
            }
        } else if self.last_title != "Pomoglow" {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("Pomoglow".into()));
            self.last_title = "Pomoglow".into();
        }
        if self.dirty {
            if self.last_save.elapsed() > Duration::from_millis(800) {
                self.data.save();
                self.dirty = false;
                self.last_save = Instant::now();
            } else {
                ctx.request_repaint_after(Duration::from_millis(900));
            }
        }

        self.frames = self.frames.saturating_add(1);
        if self.frames == 90 && std::env::var_os("POMOGLOW_OVERLAY").is_some() {
            self.overlay_on = true;
        }
        if self.frames == 90 && std::env::var_os("POMOGLOW_HIDETEST").is_some() {
            self.overlay_on = true;
            self.timer.start();
            #[cfg(windows)]
            crate::tray::win::hide();
        }
        if self.frames == 160 && std::env::var_os("POMOGLOW_HIDETEST").is_some() {
            if let Some(t) = &self.tray {
                t.test_push_show();
            }
        }
        if self.frames == 260 && std::env::var_os("POMOGLOW_HIDETEST").is_some() {
            if let Some(t) = &self.tray {
                t.test_push_quit();
            }
        }
        // close button -> hide to tray
        if ctx.input(|i| i.viewport().close_requested()) && self.tray.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            #[cfg(windows)]
            crate::tray::win::hide();
        }
        let actions = self.tray.as_ref().map(|t| t.poll()).unwrap_or_default();
        for act in actions {
            match act {
                TrayAction::Show => {
                    #[cfg(windows)]
                    crate::tray::win::show();
                }
                TrayAction::Toggle => self.timer.toggle_run(),
                TrayAction::Overlay => self.overlay_on = !self.overlay_on,
                TrayAction::Quit => {
                    self.data.save();
                    std::process::exit(0);
                }
            }
            ctx.request_repaint();
        }
        if self.frames < 120 {
            ctx.request_repaint();
        }
        #[cfg(windows)]
        if !self.overlay_on {
            overlay_closed();
        }
        if self.overlay_on {
            self.overlay(&ctx);
        }

        let full = ui.available_rect_before_wrap();
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, if is_dark() { Color32::from_rgb(13, 17, 23) } else { CREAM });
        blob(&p, full.left_top() + vec2(380.0, 60.0), 360.0, if is_dark() { Color32::from_rgb(70, 100, 170) } else { Color32::from_rgb(250, 190, 160) });
        blob(&p, full.right_top() + vec2(-120.0, 300.0), 320.0, if is_dark() { Color32::from_rgb(50, 90, 150) } else { Color32::from_rgb(190, 215, 240) });
        blob(&p, full.right_bottom() + vec2(-300.0, -60.0), 340.0, if is_dark() { Color32::from_rgb(40, 120, 125) } else { Color32::from_rgb(200, 232, 205) });
        blob(&p, full.left_bottom() + vec2(300.0, 40.0), 280.0, if is_dark() { Color32::from_rgb(90, 80, 150) } else { Color32::from_rgb(230, 210, 245) });

        if !self.data.onboarded {
            self.onboarding(ui, full);
            return;
        }

        let side = Rect::from_min_size(full.min + vec2(14.0, 14.0), vec2(230.0, full.height() - 28.0));
        self.sidebar(ui, side);

        let area = Rect::from_min_max(pos2(side.right() + 20.0, full.top() + 14.0), pos2(full.right() - 18.0, full.bottom() - 14.0));
        self.header(ui, area);
        let body = Rect::from_min_max(pos2(area.left(), area.top() + 66.0), area.max);
        let mut bui = ui.new_child(UiBuilder::new().max_rect(body));
        bui.set_clip_rect(body.expand2(vec2(14.0, 0.0)).intersect(full));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut bui, |ui| {
            ui.spacing_mut().item_spacing.y = 16.0;
            match self.page {
                Page::Dashboard => self.dashboard(ui),
                Page::Timer => self.timer_page(ui, body.height()),
                Page::Clock => self.clock_page(ui),
                Page::Profile => self.profile_page(ui, false),
                Page::Settings => self.settings_page(ui),
            }
            ui.add_space(20.0);
        });

        // toast
        if let Some((msg, t)) = &self.toast {
            if t.elapsed() < Duration::from_secs(4) {
                let r = Rect::from_center_size(pos2(full.center().x + 120.0, full.bottom() - 46.0), vec2(420.0, 44.0));
                glass(&p, r, Color32::WHITE, 22.0);
                text(&p, r.center(), Align2::CENTER_CENTER, msg, 15.0, INK, true);
                ctx.request_repaint_after(Duration::from_millis(500));
            } else {
                self.toast = None;
            }
        }
    }
}

// ---------------------------------------------------------------- clock + overlay
impl PomoglowApp {
    fn clock_data(&self) -> ClockData {
        ClockData {
            time: self.timer.formatted_time(),
            ratio: (self.timer.remaining_seconds / self.timer.total_seconds.max(1.0)) as f32,
            accent: self.accent(),
            label: self.timer.mode.title().to_uppercase(),
            tint: Color32::WHITE,
            show_label: true,
            show_progress: true,
            status: if self.timer.is_running { "Running".into() } else if self.timer.remaining_seconds < self.timer.total_seconds { "Paused".into() } else { "Ready".into() },
        }
    }

    fn overlay(&mut self, ctx: &egui::Context) {
        let st = self.data.settings.clone();
        let sz = vec2(820.0, 340.0) * st.overlay_size;
        let builder = egui::ViewportBuilder::default()
            .with_title("Pomoglow Overlay")
            .with_inner_size(sz)
            .with_decorations(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_resizable(false)
            .with_mouse_passthrough(st.click_through);
        #[cfg(windows)]
        if self.frames % 2 == 0 {
            style_overlay(st.click_through, st.overlay_pos, (st.overlay_opacity.clamp(0.05, 1.0) * 255.0) as u8);
        }
        let mut cd = self.clock_data();
        cd.tint = rgb(OV_COLORS[st.overlay_color as usize % 6]);
        cd.show_label = st.overlay_label;
        cd.show_progress = st.overlay_progress;
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("pomoglow_overlay"), builder, |ui, _class| {
            let r = ui.max_rect();
            let p = ui.painter().clone();
            draw_clock(&p, r.shrink(10.0), st.overlay_style, &cd, true, 1.0);
            if !st.click_through {
                let resp = ui.interact(r, Id::new("overlay_drag"), Sense::drag());
                if resp.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                p.rect_stroke(r.shrink(2.0), radius(18.0), Stroke::new(1.5, Color32::from_white_alpha(70)), egui::StrokeKind::Inside);
            }
        });
    }

    fn clock_page(&mut self, ui: &mut Ui) {
        let p = ui.painter().clone();
        let w = ui.available_width() - 26.0;
        let x0 = ui.min_rect().left() + 12.0;
        let (rr, _) = ui.allocate_exact_size(vec2(w + 26.0, 330.0), Sense::hover());
        let gal = Rect::from_min_size(pos2(x0, rr.top()), vec2(w, 330.0));
        glass(&p, gal, Color32::from_rgb(255, 250, 244), 30.0);
        text(&p, pos2(gal.left() + 26.0, gal.top() + 30.0), Align2::LEFT_CENTER, "Clock style", 19.0, INK, true);
        text(&p, pos2(gal.right() - 26.0, gal.top() + 30.0), Align2::RIGHT_CENTER, "Used on the Focus Timer page", 12.5, INK2, false);
        let cd = self.clock_data();
        let inner = Rect::from_min_max(gal.min + vec2(20.0, 62.0), gal.max - vec2(20.0, 20.0));
        for (i, r) in split(inner, &[1.0; 5], 12.0).iter().enumerate() {
            let on = self.data.settings.clock_style as usize == i;
            let resp = ui.interact(*r, Id::new(("clk", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            p.rect_filled(*r, radius(20.0), if on { alpha(CORAL, 38) } else { dm(Color32::from_white_alpha(120)) });
            p.rect_stroke(*r, radius(20.0), Stroke::new(if on { 2.5 } else { 1.0 }, if on { CORAL } else { alpha(INK2, 50) }), egui::StrokeKind::Inside);
            let prev = Rect::from_min_max(r.min + vec2(4.0, 10.0), pos2(r.max.x - 4.0, r.max.y - 46.0));
            draw_clock(&p.with_clip_rect(*r), prev, i as u8, &cd, false, 1.0);
            text(&p, pos2(r.center().x, r.bottom() - 24.0), Align2::CENTER_CENTER, STYLE_NAMES[i], 14.5, INK, on);
            if resp.clicked() {
                self.data.settings.clock_style = i as u8;
                self.dirty = true;
            }
        }

        let (rr, _) = ui.allocate_exact_size(vec2(w + 26.0, 660.0), Sense::hover());
        let card = Rect::from_min_size(pos2(x0, rr.top()), vec2(w, 660.0));
        glass(&p, card, Color32::from_rgb(255, 250, 244), 30.0);
        let (_, accent) = self.theme();
        let fx = card.left() + 28.0;
        let lx = fx + 170.0;
        let ctrl_w = card.width() - 170.0 - 100.0;
        text(&p, pos2(fx, card.top() + 32.0), Align2::LEFT_CENTER, "Transparent overlay", 19.0, INK, true);
        text(&p, pos2(fx, card.top() + 58.0), Align2::LEFT_CENTER, "A big see-through clock on top of every window. Click-through, so it never blocks your work.", 13.0, INK2, false);
        let mut y = card.top() + 104.0;
        let mut changed = false;

        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Show overlay", 15.0, INK, true);
        let mut on = self.overlay_on;
        toggle(ui, "ov_on", pos2(card.right() - 60.0, y), &mut on, accent);
        self.overlay_on = on;

        y += 54.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Style", 15.0, INK, true);
        let n = STYLE_NAMES.len() as f32;
        let bw = ((ctrl_w - (n - 1.0) * 8.0) / n).clamp(60.0, 96.0);
        for (i, name) in STYLE_NAMES.iter().enumerate() {
            let b = Rect::from_center_size(pos2(lx + bw / 2.0 + i as f32 * (bw + 8.0), y), vec2(bw, 34.0));
            let sel = self.data.settings.overlay_style as usize == i;
            if pill_button(ui, &format!("ovs{i}"), b, name, if sel { CORAL } else { Color32::from_white_alpha(220) }, if sel { Color32::WHITE } else { INK }, 13.0) {
                self.data.settings.overlay_style = i as u8;
                changed = true;
            }
        }

        y += 54.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Color", 15.0, INK, true);
        for (i, col) in OV_COLORS.iter().enumerate() {
            let c = pos2(lx + 18.0 + i as f32 * 44.0, y);
            let resp = ui.interact(Rect::from_center_size(c, vec2(38.0, 38.0)), Id::new(("ovc", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            p.circle_filled(c, 15.0, rgb(*col));
            p.circle_stroke(c, 15.0, Stroke::new(1.0, alpha(INK, 50)));
            if self.data.settings.overlay_color as usize == i {
                p.circle_stroke(c, 20.0, Stroke::new(2.5, INK));
            }
            if resp.clicked() {
                self.data.settings.overlay_color = i as u8;
                changed = true;
            }
        }

        y += 54.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Position", 15.0, INK, true);
        let pw = ((ctrl_w - 3.0 * 8.0) / 4.0).clamp(80.0, 116.0);
        for (i, name) in OV_POS.iter().enumerate() {
            let (row, colm) = (i / 4, i % 4);
            let b = Rect::from_center_size(pos2(lx + pw / 2.0 + colm as f32 * (pw + 8.0), y + row as f32 * 44.0 - 4.0), vec2(pw, 34.0));
            let sel = self.data.settings.overlay_pos as usize == i;
            if pill_button(ui, &format!("ovp{i}"), b, name, if sel { CORAL } else { Color32::from_white_alpha(220) }, if sel { Color32::WHITE } else { INK }, 13.0) {
                self.data.settings.overlay_pos = i as u8;
                changed = true;
            }
        }

        y += 92.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Size", 15.0, INK, true);
        changed |= line_slider(ui, "ls_size", Rect::from_min_size(pos2(lx, y - 12.0), vec2(ctrl_w - 70.0, 24.0)), &mut self.data.settings.overlay_size, 0.4, 2.5, accent);
        text(&p, pos2(lx + ctrl_w - 10.0, y), Align2::RIGHT_CENTER, &format!("{:.0}%", self.data.settings.overlay_size * 100.0), 14.0, INK, true);
        y += 52.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Transparency", 15.0, INK, true);
        let mut transp = 1.0 - self.data.settings.overlay_opacity;
        if line_slider(ui, "ls_transp", Rect::from_min_size(pos2(lx, y - 12.0), vec2(ctrl_w - 70.0, 24.0)), &mut transp, 0.0, 0.9, accent) {
            self.data.settings.overlay_opacity = 1.0 - transp;
            changed = true;
        }
        text(&p, pos2(lx + ctrl_w - 10.0, y), Align2::RIGHT_CENTER, &format!("{:.0}%", transp / 0.9 * 100.0), 14.0, INK, true);
        text(&p, pos2(lx, y + 26.0), Align2::LEFT_CENTER, "Drag the line: left = solid, right = more see-through", 12.0, INK2, false);

        y += 60.0;
        let tx = card.right() - 60.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Show mode label (Focus / Break)", 15.0, INK, false);
        changed |= toggle(ui, "ov_lbl", pos2(tx, y), &mut self.data.settings.overlay_label, accent);
        y += 46.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Show progress bar", 15.0, INK, false);
        changed |= toggle(ui, "ov_prg", pos2(tx, y), &mut self.data.settings.overlay_progress, accent);
        y += 46.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Click-through (mouse passes to windows below)", 15.0, INK, false);
        changed |= toggle(ui, "ov_ct", pos2(tx, y), &mut self.data.settings.click_through, accent);
        text(&p, pos2(fx, y + 30.0), Align2::LEFT_CENTER, "Turn click-through off to drag the overlay anywhere, then turn it back on.", 12.5, INK2, false);
        if changed {
            self.dirty = true;
        }
    }
}

// ---------------------------------------------------------------- chrome
impl PomoglowApp {
    fn sidebar(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter().clone();
        let (tint, accent) = self.theme();
        glass(&p, r, tint, 34.0);
        let cx = r.center().x;
        self.av(&p, pos2(cx, r.top() + 82.0), 52.0);
        text(&p, pos2(cx, r.top() + 158.0), Align2::CENTER_CENTER, &format!("Hi, {}!", self.data.profile.name), 22.0, INK, true);
        text(&p, pos2(cx, r.top() + 182.0), Align2::CENTER_CENTER, &self.data.profile.tagline, 12.5, INK2, false);

        let items = [(Page::Dashboard, Icon::Dashboard, "Dashboard"), (Page::Timer, Icon::Timer, "Focus Timer"), (Page::Clock, Icon::Clock, "Clock Styles"), (Page::Profile, Icon::User, "Profile"), (Page::Settings, Icon::Gear, "Settings")];
        let mut y = r.top() + 212.0;
        for (page, ic, label) in items {
            let item = Rect::from_min_size(pos2(r.left() + 14.0, y), vec2(r.width() - 28.0, 42.0));
            let resp = ui.interact(item, Id::new(("nav", label)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                self.page = page;
            }
            let active = self.page == page;
            if active {
                glass(&p, item, CREAM, 25.0);
            } else if resp.hovered() {
                p.rect_filled(item, radius(25.0), dm(Color32::from_white_alpha(70)));
            }
            icon(&p, ic, pos2(item.left() + 34.0, item.center().y), 24.0, if active { accent } else { mix(accent, INK, 0.45) });
            text(&p, pos2(item.left() + 62.0, item.center().y), Align2::LEFT_CENTER, label, 16.0, INK, active);
            y += 47.0;
        }

        // daily goal card
        let goal = self.data.profile.daily_goal.max(1);
        let done = self.today().sessions;
        let card = Rect::from_min_max(pos2(r.left() + 14.0, r.bottom() - 134.0), pos2(r.right() - 14.0, r.bottom() - 14.0));
        glass(&p, card, PEACH, 26.0);
        text(&p, pos2(card.center().x, card.top() + 26.0), Align2::CENTER_CENTER, "Daily Goal", 17.0, INK, true);
        text(&p, pos2(card.center().x, card.top() + 52.0), Align2::CENTER_CENTER, &format!("{done} of {goal} sessions"), 13.0, INK2, false);
        let bar = Rect::from_min_size(pos2(card.left() + 20.0, card.top() + 70.0), vec2(card.width() - 40.0, 12.0));
        p.rect_filled(bar, radius(6.0), dm(Color32::from_white_alpha(130)));
        let frac = (done as f32 / goal as f32).clamp(0.0, 1.0);
        if frac > 0.0 {
            p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * frac).max(12.0), 12.0)), radius(6.0), CORAL);
        }
        let btn = Rect::from_center_size(pos2(card.center().x, card.bottom() - 28.0), vec2(card.width() - 40.0, 36.0));
        let label = if self.timer.is_running { "Open Timer" } else { "Start Focus" };
        if pill_button(ui, "side_start", btn, label, CORAL, Color32::WHITE, 15.0) {
            if !self.timer.is_running {
                self.timer.set_mode(TimerMode::Focus);
                self.timer.start();
            }
            self.page = Page::Timer;
        }
    }

    fn header(&mut self, ui: &mut Ui, area: Rect) {
        let p = ui.painter().clone();
        let title = match self.page {
            Page::Dashboard => "Dashboard",
            Page::Timer => "Focus Timer",
            Page::Clock => "Clock Styles",
            Page::Profile => "My Profile",
            Page::Settings => "Settings",
        };
        text(&p, pos2(area.left() + 12.0, area.top() + 30.0), Align2::LEFT_CENTER, title, 34.0, INK, true);
        let right = area.right() - 12.0;
        // profile chip
        let chip = Rect::from_min_max(pos2(right - 190.0, area.top() + 6.0), pos2(right, area.top() + 54.0));
        glass(&p, chip, Color32::WHITE, 24.0);
        self.av(&p, pos2(chip.left() + 26.0, chip.center().y), 19.0);
        text(&p, pos2(chip.left() + 54.0, chip.center().y - 8.0), Align2::LEFT_CENTER, &self.data.profile.name, 14.5, INK, true);
        text(&p, pos2(chip.left() + 54.0, chip.center().y + 10.0), Align2::LEFT_CENTER, &format!("{} day streak", self.data.streak()), 11.5, INK2, false);
        if ui.interact(chip, Id::new("chip"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.page = Page::Profile;
        }
        // pin
        let pc = pos2(chip.left() - 34.0, chip.center().y);
        let fill = if self.always_on_top { CORAL } else { Color32::from_white_alpha(230) };
        if icon_button(ui, "pin", pc, 21.0, Icon::Pin, fill, if self.always_on_top { Color32::WHITE } else { INK2 }) {
            self.always_on_top = !self.always_on_top;
            let lvl = if self.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(lvl));
        }
        let mc = pos2(pc.x - 50.0, pc.y);
        let dark = self.data.settings.dark;
        if icon_button(ui, "darkmode", mc, 21.0, if dark { Icon::Sun } else { Icon::Moon }, Color32::from_white_alpha(230), INK2) {
            self.data.settings.dark = !dark;
            self.dirty = true;
        }
        text(&p, pos2(mc.x - 32.0, pc.y), Align2::RIGHT_CENTER, &pretty_date(&now_local()), 14.0, INK2, false);
    }
}

// ---------------------------------------------------------------- dashboard
impl PomoglowApp {
    fn dashboard(&mut self, ui: &mut Ui) {
        let p = ui.painter().clone();
        let w = ui.available_width() - 26.0;
        let x0 = ui.min_rect().left() + 12.0;
        let mut row = |ui: &mut Ui, h: f32| -> Rect {
            let (r, _) = ui.allocate_exact_size(vec2(w + 26.0, h), Sense::hover());
            Rect::from_min_size(pos2(x0, r.top()), vec2(w, h))
        };
        let now = now_local();
        let today = self.today();
        let yesterday = self.data.day(now.days - 1);
        let week = self.week_stats();
        let week_mins: u32 = week.iter().map(|d| d.focus_mins).sum();
        let goal = self.data.profile.daily_goal.max(1);

        // hero
        let hero = row(ui, 176.0);
        glass(&p, hero, PEACH, 32.0);
        self.av(&p, pos2(hero.left() + 100.0, hero.center().y), 70.0);
        let tx = hero.left() + 200.0;
        text(&p, pos2(tx, hero.top() + 48.0), Align2::LEFT_CENTER, &format!("{}, {}!", greeting(now.hour), self.data.profile.name), 30.0, INK, true);
        let sub = if today.sessions >= goal { "Daily goal reached. Amazing work!".to_string() } else { format!("{} of {} focus sessions done today. Let's go!", today.sessions, goal) };
        text(&p, pos2(tx, hero.top() + 84.0), Align2::LEFT_CENTER, &sub, 16.0, INK2, false);
        let label = if self.timer.is_running { "Open Timer" } else { "Start Focus" };
        if pill_button(ui, "hero_start", Rect::from_min_size(pos2(tx, hero.top() + 108.0), vec2(170.0, 42.0)), label, CORAL, Color32::WHITE, 16.0) {
            if !self.timer.is_running {
                self.timer.set_mode(TimerMode::Focus);
                self.timer.start();
            }
            self.page = Page::Timer;
        }
        // plant
        let pot = pos2(hero.right() - 90.0, hero.bottom() - 28.0);
        for (a, l) in [(-1.9, 70.0), (-1.4, 84.0), (-1.0, 74.0), (-2.4, 56.0), (-0.6, 56.0)] {
            leaf(&p, pot + vec2(0.0, -20.0), a, l, Color32::from_rgb(110, 160, 108));
        }
        p.rect_filled(Rect::from_center_size(pot, vec2(60.0, 46.0)), radius(14.0), Color32::from_rgb(240, 212, 180));
        p.rect_filled(Rect::from_center_size(pot + vec2(0.0, -20.0), vec2(68.0, 12.0)), radius(6.0), Color32::from_rgb(228, 196, 160));

        // stats
        let stats = row(ui, 112.0);
        let cols = split(stats, &[1.0; 4], 14.0);
        let hours = format!("{:.1}", today.focus_mins as f32 / 60.0);
        let cards: [(Color32, Color32, Icon, &str, String, String); 4] = [
            (SAGE, Color32::from_rgb(110, 170, 140), Icon::Timer, "Sessions Today", today.sessions.to_string(), format!("{} yesterday", yesterday.sessions)),
            (PEACH, CORAL, Icon::Clock, "Hours Focused", hours, format!("{:.1}h this week", week_mins as f32 / 60.0)),
            (BUTTER, Color32::from_rgb(240, 170, 60), Icon::Flame, "Current Streak", self.data.streak().to_string(), "days in a row".into()),
            (SKY, Color32::from_rgb(110, 150, 210), Icon::Heart, "Daily Goal", format!("{}%", (today.sessions * 100 / goal).min(100)), format!("{} / {} sessions", today.sessions, goal)),
        ];
        for (r, (tint, tile_c, ic, label, big, small)) in cols.iter().zip(cards) {
            glass(&p, *r, tint, 26.0);
            tile(&p, pos2(r.left() + 40.0, r.top() + 36.0), 42.0, tile_c, ic);
            text(&p, pos2(r.left() + 70.0, r.top() + 36.0), Align2::LEFT_CENTER, label, 14.0, INK, true);
            text(&p, pos2(r.left() + 20.0, r.top() + 72.0), Align2::LEFT_CENTER, &big, 30.0, INK, true);
            text(&p, pos2(r.left() + 20.0, r.top() + 98.0), Align2::LEFT_CENTER, &small, 12.5, INK2, false);
        }

        // chart + donut
        let r3 = row(ui, 270.0);
        let cols = split(r3, &[1.55, 1.0], 16.0);
        self.chart(&p, cols[0], &week, now.weekday);
        self.donut(&p, cols[1], &week);

        // recent + mini timer
        let r4 = row(ui, 250.0);
        let cols = split(r4, &[1.55, 1.0], 16.0);
        self.recent(ui, &p, cols[0]);
        self.mini_timer(ui, &p, cols[1]);
    }

    fn chart(&self, p: &Painter, r: Rect, week: &[DayStat; 7], today: usize) {
        glass(p, r, Color32::from_rgb(255, 250, 244), 28.0);
        text(p, pos2(r.left() + 24.0, r.top() + 30.0), Align2::LEFT_CENTER, "Focus Overview", 18.0, INK, true);
        text(p, pos2(r.right() - 24.0, r.top() + 30.0), Align2::RIGHT_CENTER, "This week", 13.0, INK2, false);
        let plot = Rect::from_min_max(pos2(r.left() + 58.0, r.top() + 64.0), pos2(r.right() - 24.0, r.bottom() - 38.0));
        let maxm = week.iter().map(|d| d.focus_mins).max().unwrap_or(0).max(60);
        let top_h = ((maxm as f32 / 60.0).ceil() as u32).max(1);
        let step = if top_h > 6 { 2 } else { 1 };
        let top = (top_h + step - 1) / step * step;
        let scale = top as f32 * 60.0;
        let mut h = 0;
        while h <= top {
            let y = plot.bottom() - plot.height() * (h as f32 * 60.0 / scale);
            p.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], Stroke::new(1.0, alpha(INK2, 40)));
            text(p, pos2(plot.left() - 10.0, y), Align2::RIGHT_CENTER, &format!("{h}h"), 12.0, INK2, false);
            h += step;
        }
        let slot = plot.width() / 7.0;
        for (i, d) in week.iter().enumerate() {
            let cx = plot.left() + slot * (i as f32 + 0.5);
            let bh = (plot.height() * d.focus_mins as f32 / scale).max(if d.focus_mins > 0 { 8.0 } else { 0.0 });
            let bw = (slot * 0.52).min(40.0);
            let col = rgb(BAR_COLORS[i]);
            if bh > 0.0 {
                let br = Rect::from_min_max(pos2(cx - bw / 2.0, plot.bottom() - bh), pos2(cx + bw / 2.0, plot.bottom()));
                p.rect_filled(br, radius(bw / 2.0), col);
                p.rect_filled(Rect::from_min_size(br.min + vec2(4.0, 3.0), vec2(bw * 0.28, (bh - 8.0).max(0.0))), radius(bw * 0.14), dm(Color32::from_white_alpha(70)));
                text(p, pos2(cx, br.top() - 10.0), Align2::CENTER_CENTER, &format!("{}m", d.focus_mins), 11.5, INK2, false);
            } else {
                p.circle_filled(pos2(cx, plot.bottom() - 3.0), 3.0, alpha(INK2, 70));
            }
            text(p, pos2(cx, plot.bottom() + 18.0), Align2::CENTER_CENTER, WEEKDAYS[i], 13.0, if i == today { CORAL } else { INK2 }, i == today);
        }
    }

    fn donut(&self, p: &Painter, r: Rect, week: &[DayStat; 7]) {
        glass(p, r, Color32::from_rgb(255, 250, 244), 28.0);
        text(p, pos2(r.left() + 24.0, r.top() + 30.0), Align2::LEFT_CENTER, "When You Focus", 18.0, INK, true);
        let mut per = [0u32; 4];
        for d in week {
            for k in 0..4 {
                per[k] += d.periods[k];
            }
        }
        let total: u32 = per.iter().sum();
        let c = pos2(r.left() + 84.0, r.top() + 156.0);
        let rad = 56.0;
        if total == 0 {
            p.circle_stroke(c, rad, Stroke::new(26.0, alpha(INK2, 40)));
            text(p, c, Align2::CENTER_CENTER, "No data", 13.0, INK2, false);
        } else {
            let mut a = -PI * 0.5;
            for k in 0..4 {
                if per[k] == 0 {
                    continue;
                }
                let sweep = 2.0 * PI * per[k] as f32 / total as f32;
                let gap = if per.iter().filter(|v| **v > 0).count() > 1 { 0.05 } else { 0.0 };
                p.add(Shape::line(arc_points(c, rad, a + gap, a + sweep - gap), Stroke::new(26.0, rgb(PERIOD_COLORS[k]))));
                a += sweep;
            }
            text(p, c, Align2::CENTER_CENTER, &format!("{}h", total / 60), 22.0, INK, true);
            text(p, c + vec2(0.0, 20.0), Align2::CENTER_CENTER, "total", 11.0, INK2, false);
        }
        for k in 0..4 {
            let y = r.top() + 100.0 + k as f32 * 34.0;
            let lx = r.left() + 166.0;
            p.circle_filled(pos2(lx, y), 6.0, rgb(PERIOD_COLORS[k]));
            text(p, pos2(lx + 12.0, y), Align2::LEFT_CENTER, PERIOD_NAMES[k], 12.5, INK, false);
            let pct = if total > 0 { per[k] * 100 / total } else { 0 };
            text(p, pos2(r.right() - 22.0, y), Align2::RIGHT_CENTER, &format!("{pct}%"), 12.5, INK, true);
        }
    }

    fn recent(&mut self, ui: &mut Ui, p: &Painter, r: Rect) {
        glass(p, r, Color32::from_rgb(255, 250, 244), 28.0);
        text(p, pos2(r.left() + 24.0, r.top() + 30.0), Align2::LEFT_CENTER, "Recent Sessions", 18.0, INK, true);
        if self.data.log.is_empty() {
            text(p, r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Finish a focus session to see it here.", 14.0, INK2, false);
            return;
        }
        let tints = [SAGE, PEACH, BUTTER, SKY];
        for (i, s) in self.data.log.iter().rev().take(4).enumerate() {
            let y = r.top() + 82.0 + i as f32 * 41.0;
            tile(p, pos2(r.left() + 42.0, y), 32.0, mix(tints[i % 4], INK, 0.3), Icon::Timer);
            text(p, pos2(r.left() + 68.0, y - 8.0), Align2::LEFT_CENTER, &s.label, 14.5, INK, true);
            text(p, pos2(r.left() + 68.0, y + 10.0), Align2::LEFT_CENTER, &format!("{}  {}", &s.date, s.time), 11.5, INK2, false);
            let pill = Rect::from_center_size(pos2(r.right() - 52.0, y), vec2(66.0, 28.0));
            p.rect_filled(pill, radius(14.0), tints[i % 4]);
            text(p, pill.center(), Align2::CENTER_CENTER, &format!("{}m", s.mins), 13.0, INK, true);
        }
        let _ = ui;
    }

    fn mini_timer(&mut self, ui: &mut Ui, p: &Painter, r: Rect) {
        glass(p, r, SAGE, 28.0);
        let acc = self.accent();
        text(p, pos2(r.left() + 24.0, r.top() + 30.0), Align2::LEFT_CENTER, "Quick Timer", 18.0, INK, true);
        let c = pos2(r.center().x, r.top() + 128.0);
        let ratio = (self.timer.remaining_seconds / self.timer.total_seconds.max(1.0)) as f32;
        ring(p, c, 56.0, 10.0, ratio, acc, dm(Color32::from_white_alpha(120)));
        text(p, c, Align2::CENTER_CENTER, &self.timer.formatted_time(), 26.0, INK, true);
        text(p, pos2(r.center().x, r.top() + 204.0), Align2::CENTER_CENTER, self.timer.mode.title(), 13.0, INK2, false);
        let kind = if self.timer.is_running { Icon::Pause } else { Icon::Play };
        if icon_button(ui, "mini_play", pos2(r.right() - 44.0, r.bottom() - 40.0), 22.0, kind, acc, Color32::WHITE) {
            self.timer.toggle_run();
        }
        if pill_button(ui, "mini_open", Rect::from_center_size(pos2(r.left() + 66.0, r.bottom() - 40.0), vec2(96.0, 34.0)), "Open", Color32::from_white_alpha(200), INK, 14.0) {
            self.page = Page::Timer;
        }
    }
}

// ---------------------------------------------------------------- timer page
impl PomoglowApp {
    fn timer_page(&mut self, ui: &mut Ui, avail_h: f32) {
        let p = ui.painter().clone();
        let w = ui.available_width() - 26.0;
        let h = avail_h.max(640.0) - 20.0;
        let (r, _) = ui.allocate_exact_size(vec2(w + 26.0, h), Sense::hover());
        let card_w = w.min(760.0);
        let card = Rect::from_center_size(pos2(r.center().x, r.top() + h / 2.0), vec2(card_w, h));
        let acc = self.accent();
        glass(&p, card, Color32::from_rgb(255, 250, 244), 40.0);
        let cx = card.center().x;

        // mode pills
        let modes = [(TimerMode::Focus, "Focus"), (TimerMode::ShortBreak, "Short Break"), (TimerMode::LongBreak, "Long Break")];
        let seg = Rect::from_center_size(pos2(cx, card.top() + 46.0), vec2(380.0, 44.0));
        p.rect_filled(seg, radius(22.0), alpha(INK2, 28));
        for (i, (m, label)) in modes.iter().enumerate() {
            let b = Rect::from_min_size(seg.min + vec2(4.0 + i as f32 * 124.0, 4.0), vec2(124.0, 36.0));
            let on = self.timer.mode == *m;
            let resp = ui.interact(b, Id::new(("mode", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if on {
                p.rect_filled(b, radius(18.0), acc);
            }
            text(&p, b.center(), Align2::CENTER_CENTER, label, 14.0, if on { Color32::WHITE } else { INK2 }, true);
            if resp.clicked() && !on {
                self.timer.set_mode(*m);
            }
        }

        // ring
        let rad = (card.height() * 0.24).clamp(100.0, 150.0);
        let c = pos2(cx, card.top() + 100.0 + rad + 24.0);
        let cd = self.clock_data();
        draw_clock(&p, Rect::from_center_size(c, vec2(card.width() - 70.0, rad * 2.0 + 20.0)), self.data.settings.clock_style, &cd, false, 1.0);

        // controls
        let cy = c.y + rad + 62.0;
        if icon_button(ui, "t_reset", pos2(cx - 104.0, cy), 26.0, Icon::Reset, SKY, INK) {
            self.timer.reset();
        }
        let kind = if self.timer.is_running { Icon::Pause } else { Icon::Play };
        if icon_button(ui, "t_play", pos2(cx, cy), 38.0, kind, acc, Color32::WHITE) {
            self.timer.toggle_run();
        }
        if icon_button(ui, "t_skip", pos2(cx + 104.0, cy), 26.0, Icon::Skip, LAVENDER, INK) {
            self.timer.skip_to_next();
        }

        // nudges
        let ny = cy + 66.0;
        for (i, (label, d)) in [("-1m", -60.0), ("+1m", 60.0), ("+5m", 300.0)].iter().enumerate() {
            let b = Rect::from_center_size(pos2(cx - 80.0 + i as f32 * 80.0, ny), vec2(66.0, 32.0));
            if pill_button(ui, &format!("nudge{i}"), b, label, Color32::from_white_alpha(210), INK, 13.5) {
                self.timer.add_seconds(*d);
            }
        }

        // task input
        let ty = ny + 52.0;
        text(&p, pos2(cx - 190.0, ty - 22.0), Align2::LEFT_CENTER, "Working on", 12.5, INK2, true);
        ui.put(
            Rect::from_center_size(pos2(cx, ty + 8.0), vec2(380.0, 36.0)),
            egui::TextEdit::singleline(&mut self.task).hint_text("What are you focusing on?").font(font(15.0, false)).margin(vec2(12.0, 8.0)),
        );

        // cycle dots
        let dy = ty + 62.0;
        let goal = self.timer.cycle_goal;
        for i in 0..goal {
            let x = cx + (i as f32 - (goal as f32 - 1.0) / 2.0) * 30.0;
            let on = i < self.timer.completed_sessions;
            p.circle_filled(pos2(x, dy), 9.0, if on { acc } else { alpha(INK2, 40) });
        }
        text(&p, pos2(cx, dy + 26.0), Align2::CENTER_CENTER, &format!("{} of {} until long break", self.timer.completed_sessions, goal), 12.5, INK2, false);
    }
}

// ---------------------------------------------------------------- profile
impl PomoglowApp {
    fn swatches(&self, ui: &mut Ui, p: &Painter, id: &str, label: &str, x: f32, y: f32, colors: &[[u8; 3]], sel: &mut usize) -> bool {
        text(p, pos2(x, y), Align2::LEFT_CENTER, label, 14.0, INK, true);
        let mut changed = false;
        for (i, col) in colors.iter().enumerate() {
            let c = pos2(x + 130.0 + i as f32 * 42.0, y);
            let resp = ui.interact(Rect::from_center_size(c, vec2(36.0, 36.0)), Id::new((id, i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            p.circle_filled(c, 14.0, rgb(*col));
            p.circle_stroke(c, 14.0, Stroke::new(1.0, alpha(INK, 40)));
            if *sel == i {
                p.circle_stroke(c, 19.0, Stroke::new(2.5, INK));
            }
            if resp.clicked() {
                *sel = i;
                changed = true;
            }
        }
        changed
    }

    fn onboarding(&mut self, ui: &mut Ui, full: Rect) {
        let p = ui.painter().clone();
        let w = (full.width() - 60.0).min(1000.0);
        let area = Rect::from_min_max(pos2(full.center().x - w / 2.0, full.top() + 14.0), pos2(full.center().x + w / 2.0, full.bottom() - 10.0));
        text(&p, pos2(area.left() + 12.0, area.top() + 34.0), Align2::LEFT_CENTER, "Welcome to Pomoglow", 34.0, INK, true);
        text(&p, pos2(area.left() + 12.0, area.top() + 70.0), Align2::LEFT_CENTER, "Set up your profile to get started. You can change everything later.", 15.0, INK2, false);
        let body = Rect::from_min_max(pos2(area.left(), area.top() + 96.0), area.max);
        let mut bui = ui.new_child(UiBuilder::new().max_rect(body));
        bui.set_clip_rect(body.expand2(vec2(14.0, 0.0)).intersect(full));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut bui, |ui| {
            self.profile_page(ui, true);
            ui.add_space(20.0);
        });
    }

    fn profile_page(&mut self, ui: &mut Ui, setup: bool) {
        let p = ui.painter().clone();
        let w = ui.available_width() - 26.0;
        let x0 = ui.min_rect().left() + 12.0;
        let (rr, _) = ui.allocate_exact_size(vec2(w + 26.0, 600.0), Sense::hover());
        let whole = Rect::from_min_size(pos2(x0, rr.top()), vec2(w, 600.0));
        let cols = split(whole, &[0.8, 1.6], 16.0);

        // preview
        let l = cols[0];
        glass(&p, l, PEACH, 32.0);
        let (tint, _) = self.theme();
        p.rect_filled(Rect::from_min_size(l.min + vec2(14.0, 14.0), vec2(l.width() - 28.0, 120.0)), radius(22.0), alpha(tint, 150));
        self.av(&p, pos2(l.center().x, l.top() + 150.0), 90.0);
        text(&p, pos2(l.center().x, l.top() + 262.0), Align2::CENTER_CENTER, &self.data.profile.name, 26.0, INK, true);
        text(&p, pos2(l.center().x, l.top() + 290.0), Align2::CENTER_CENTER, &self.data.profile.tagline, 14.0, INK2, false);
        let total_sessions: u32 = self.data.days.values().map(|d| d.sessions).sum();
        let total_mins: u32 = self.data.days.values().map(|d| d.focus_mins).sum();
        let facts = [("Sessions", total_sessions.to_string()), ("Hours", format!("{:.1}", total_mins as f32 / 60.0)), ("Streak", self.data.streak().to_string())];
        let fr = split(Rect::from_min_size(pos2(l.left() + 18.0, l.top() + 330.0), vec2(l.width() - 36.0, 84.0)), &[1.0; 3], 10.0);
        for (r, (k, v)) in fr.iter().zip(facts) {
            p.rect_filled(*r, radius(18.0), dm(Color32::from_white_alpha(140)));
            text(&p, r.center() + vec2(0.0, -10.0), Align2::CENTER_CENTER, &v, 22.0, INK, true);
            text(&p, r.center() + vec2(0.0, 16.0), Align2::CENTER_CENTER, k, 12.0, INK2, false);
        }
        if pill_button(ui, "pick_photo", Rect::from_center_size(pos2(l.center().x, l.top() + 454.0), vec2(l.width() - 50.0, 40.0)), "Choose Picture", CORAL, Color32::WHITE, 14.5) {
            let ctx = ui.ctx().clone();
            self.pick_photo(&ctx);
        }
        let hw = (l.width() - 50.0 - 8.0) / 2.0;
        if pill_button(ui, "mode_cartoon", Rect::from_center_size(pos2(l.left() + 25.0 + hw / 2.0, l.top() + 506.0), vec2(hw, 34.0)), "Cartoon", Color32::from_white_alpha(220), INK, 13.0) {
            self.data.profile.mode = 1;
            self.dirty = true;
        }
        if pill_button(ui, "mode_none", Rect::from_center_size(pos2(l.right() - 25.0 - hw / 2.0, l.top() + 506.0), vec2(hw, 34.0)), "No Avatar", Color32::from_white_alpha(220), INK, 13.0) {
            self.data.profile.mode = 0;
            self.dirty = true;
        }
        text(&p, pos2(l.center().x, l.bottom() - 60.0), Align2::CENTER_CENTER, "Your timer progress and stats", 12.5, INK2, false);
        text(&p, pos2(l.center().x, l.bottom() - 40.0), Align2::CENTER_CENTER, "are saved to this profile.", 12.5, INK2, false);

        // form
        let f = cols[1];
        glass(&p, f, Color32::from_rgb(255, 250, 244), 32.0);
        let fx = f.left() + 28.0;
        text(&p, pos2(fx, f.top() + 34.0), Align2::LEFT_CENTER, "Customize Profile", 20.0, INK, true);

        let mut changed = false;
        let mut y = f.top() + 84.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Name", 14.0, INK, true);
        changed |= ui.put(Rect::from_min_size(pos2(fx + 130.0, y - 18.0), vec2(f.width() - 190.0, 36.0)), egui::TextEdit::singleline(&mut self.data.profile.name).hint_text("Your name").char_limit(18).font(font(15.0, false)).margin(vec2(12.0, 8.0))).changed();
        y += 52.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Tagline", 14.0, INK, true);
        changed |= ui.put(Rect::from_min_size(pos2(fx + 130.0, y - 18.0), vec2(f.width() - 190.0, 36.0)), egui::TextEdit::singleline(&mut self.data.profile.tagline).hint_text("e.g. Deep work enthusiast").char_limit(32).font(font(15.0, false)).margin(vec2(12.0, 8.0))).changed();
        y += 52.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Daily goal", 14.0, INK, true);
        changed |= ui.put(Rect::from_min_size(pos2(fx + 130.0, y - 14.0), vec2(f.width() - 190.0, 28.0)), egui::Slider::new(&mut self.data.profile.daily_goal, 1..=16).suffix(" sessions")).changed();
        y += 56.0;

        let mut pf = self.data.profile.clone();
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Cartoon avatar", 14.0, INK, true);
        for (i, name) in ["Female", "Male"].iter().enumerate() {
            let b = Rect::from_center_size(pos2(fx + 130.0 + 60.0 + i as f32 * 136.0, y), vec2(124.0, 38.0));
            let on = pf.mode == 1 && pf.gender as usize == i;
            if pill_button(ui, &format!("gender{i}"), b, name, if on { CORAL } else { Color32::from_white_alpha(220) }, if on { Color32::WHITE } else { INK }, 14.5) {
                pf.gender = i as u8;
                pf.mode = 1;
                changed = true;
            }
        }
        y += 62.0;
        let theme_cols: Vec<[u8; 3]> = THEMES.iter().map(|t| t.1).collect();
        changed |= self.swatches(ui, &p, "theme", "App theme", fx, y, &theme_cols, &mut pf.theme);
        if setup {
            let ready = !self.data.profile.name.trim().is_empty();
            let b = Rect::from_center_size(pos2(f.center().x, f.bottom() - 36.0), vec2(240.0, 44.0));
            let fill = if ready { CORAL } else { Color32::from_rgb(214, 204, 194) };
            if pill_button(ui, "get_started", b, "Get Started", fill, Color32::WHITE, 16.0) && ready {
                self.data.profile.name = self.data.profile.name.trim().to_string();
                self.data.onboarded = true;
                self.page = Page::Dashboard;
                self.dirty = true;
            }
            if !ready {
                text(&p, pos2(f.center().x, f.bottom() - 70.0), Align2::CENTER_CENTER, "Enter your name to continue", 12.5, INK2, false);
            }
        }
        pf.name = self.data.profile.name.clone();
        pf.tagline = self.data.profile.tagline.clone();
        pf.daily_goal = self.data.profile.daily_goal;
        self.data.profile = pf;
        if changed {
            self.dirty = true;
        }
    }
}

// ---------------------------------------------------------------- settings
impl PomoglowApp {
    fn settings_page(&mut self, ui: &mut Ui) {
        let p = ui.painter().clone();
        let w = ui.available_width() - 26.0;
        let x0 = ui.min_rect().left() + 12.0;
        let (rr, _) = ui.allocate_exact_size(vec2(w + 26.0, 560.0), Sense::hover());
        let card = Rect::from_min_size(pos2(x0, rr.top()), vec2(w.min(820.0), 560.0));
        glass(&p, card, Color32::from_rgb(255, 250, 244), 32.0);
        let (_, accent) = self.theme();
        let fx = card.left() + 30.0;
        let mut changed = false;
        text(&p, pos2(fx, card.top() + 36.0), Align2::LEFT_CENTER, "Timer Durations", 20.0, INK, true);
        let mut y = card.top() + 90.0;
        let s = &mut self.data.settings;
        for (label, val, max) in [("Focus", &mut s.focus, 120u32), ("Short break", &mut s.short, 60), ("Long break", &mut s.long, 90)] {
            text(&p, pos2(fx, y), Align2::LEFT_CENTER, label, 15.0, INK, true);
            changed |= ui.put(Rect::from_min_size(pos2(fx + 160.0, y - 14.0), vec2(card.width() - 230.0, 28.0)), egui::Slider::new(val, 1..=max).suffix(" min")).changed();
            y += 52.0;
        }
        y += 8.0;
        text(&p, pos2(fx, y), Align2::LEFT_CENTER, "Behavior", 20.0, INK, true);
        y += 50.0;
        let tx = card.right() - 60.0;
        for (id, label, val) in [("snd", "Play sound when a session ends", &mut s.sound), ("ab", "Auto-start breaks", &mut s.auto_breaks), ("af", "Auto-start focus after breaks", &mut s.auto_focus), ("dk", "Dark mode", &mut s.dark)] {
            text(&p, pos2(fx, y), Align2::LEFT_CENTER, label, 15.0, INK, false);
            changed |= toggle(ui, id, pos2(tx, y), val, accent);
            y += 46.0;
        }
        y += 14.0;
        let label = if self.confirm_reset { "Click again to erase all stats" } else { "Reset statistics" };
        if pill_button(ui, "reset", Rect::from_min_size(pos2(fx, y), vec2(260.0, 40.0)), label, if self.confirm_reset { CORAL } else { Color32::from_white_alpha(220) }, if self.confirm_reset { Color32::WHITE } else { INK }, 14.5) {
            if self.confirm_reset {
                self.data.days.clear();
                self.data.log.clear();
                self.timer.completed_sessions = 0;
                self.confirm_reset = false;
                self.dirty = true;
                self.say("Statistics cleared");
            } else {
                self.confirm_reset = true;
            }
        }
        if changed {
            let s = self.data.settings.clone();
            self.timer.apply_settings(s.focus, s.short, s.long, s.sound, s.auto_breaks, s.auto_focus);
            self.dirty = true;
        }
    }
}
