//! System tray icon + "hide to tray" window handling (Windows).

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub enum TrayAction {
    Show,
    Toggle,
    Overlay,
    Quit,
}

type Queue = std::sync::Arc<std::sync::Mutex<Vec<TrayAction>>>;

pub struct Tray {
    _icon: TrayIcon,
    queue: Queue,
}

fn make_icon() -> Icon {
    if let Ok(img) = image::load_from_memory(include_bytes!("../assets/icon64.png")) {
        let img = img.to_rgba8();
        if let Ok(i) = Icon::from_rgba(img.clone().into_raw(), img.width(), img.height()) {
            return i;
        }
    }
    make_fallback_icon()
}

fn make_fallback_icon() -> Icon {
    let n = 32u32;
    let mut px = vec![0u8; (n * n * 4) as usize];
    let c = (n as f32 - 1.0) / 2.0;
    for y in 0..n {
        for x in 0..n {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let i = ((y * n + x) * 4) as usize;
            let (rgb, a) = if d > 15.0 {
                ([0, 0, 0], 0)
            } else if d > 10.5 {
                ([234, 120, 88], 255)
            } else if d > 8.0 {
                ([253, 246, 236], 255)
            } else {
                ([252, 216, 202], 255)
            };
            px[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
        }
    }
    // clock hands
    for (x, y) in (9..16).map(|y| (16u32, y)).chain((16..21).map(|x| (x, 16u32))) {
        let i = ((y * n + x) * 4) as usize;
        px[i..i + 4].copy_from_slice(&[234, 120, 88, 255]);
    }
    Icon::from_rgba(px, n, n).expect("icon")
}

impl Tray {
    pub fn new(ctx: &egui::Context) -> Option<Self> {
        let menu = Menu::new();
        let show = MenuItem::new("Open Pomoglow", true, None);
        let toggle = MenuItem::new("Start / Pause timer", true, None);
        let overlay = MenuItem::new("Show / Hide overlay clock", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[&show, &toggle, &overlay, &PredefinedMenuItem::separator(), &quit]).ok()?;
        let icon = TrayIconBuilder::new().with_menu(Box::new(menu)).with_menu_on_left_click(false).with_tooltip("Pomoglow").with_icon(make_icon()).build().ok()?;
        // With an event handler set, tray-icon does not fill its receiver channels, so we queue ourselves.
        let queue: Queue = Default::default();
        let (show_id, toggle_id, overlay_id, quit_id) = (show.id().clone(), toggle.id().clone(), overlay.id().clone(), quit.id().clone());
        let (q1, c1) = (queue.clone(), ctx.clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let act = if e.id == show_id {
                TrayAction::Show
            } else if e.id == toggle_id {
                TrayAction::Toggle
            } else if e.id == overlay_id {
                TrayAction::Overlay
            } else if e.id == quit_id {
                TrayAction::Quit
            } else {
                return;
            };
            q1.lock().unwrap().push(act);
            c1.request_repaint();
        }));
        let (q2, c2) = (queue.clone(), ctx.clone());
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = e {
                q2.lock().unwrap().push(TrayAction::Show);
                c2.request_repaint();
            }
        }));
        Some(Self { _icon: icon, queue })
    }

    pub fn set_tooltip(&self, s: &str) {
        let _ = self._icon.set_tooltip(Some(s));
    }

    pub fn test_push_show(&self) {
        self.queue.lock().unwrap().push(TrayAction::Show);
    }

    pub fn test_push_quit(&self) {
        self.queue.lock().unwrap().push(TrayAction::Quit);
    }

    pub fn poll(&self) -> Vec<TrayAction> {
        std::mem::take(&mut *self.queue.lock().unwrap())
    }
}

#[cfg(windows)]
pub mod win {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    static FOUND: AtomicIsize = AtomicIsize::new(0);
    static SAVED_X: AtomicIsize = AtomicIsize::new(100);
    static SAVED_Y: AtomicIsize = AtomicIsize::new(100);

    unsafe extern "system" fn each(hwnd: HWND, _: LPARAM) -> i32 {
        unsafe {
            let mut buf = [0u16; 64];
            let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), 64) as usize;
            let t = String::from_utf16_lossy(&buf[..n]);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            if !t.is_empty() && !t.contains("Overlay") && (style & WS_CAPTION) == WS_CAPTION && t.contains("Pomoglow") {
                FOUND.store(hwnd as isize, Ordering::Relaxed);
                return 0;
            }
        }
        1
    }

    fn main_hwnd() -> Option<HWND> {
        FOUND.store(0, Ordering::Relaxed);
        unsafe { EnumThreadWindows(GetCurrentThreadId(), Some(each), 0) };
        let h = FOUND.load(Ordering::Relaxed);
        if h == 0 { None } else { Some(h as HWND) }
    }

    /// Moves the main window off-screen and removes it from the taskbar (it keeps running and painting).
    pub fn hide() {
        let Some(h) = main_hwnd() else { return };
        unsafe {
            let mut rc: RECT = std::mem::zeroed();
            GetWindowRect(h, &mut rc);
            if rc.left > -30000 {
                SAVED_X.store(rc.left as isize, Ordering::Relaxed);
                SAVED_Y.store(rc.top as isize, Ordering::Relaxed);
            }
            ShowWindow(h, SW_HIDE);
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            SetWindowLongPtrW(h, GWL_EXSTYLE, (ex | WS_EX_TOOLWINDOW as isize) & !(WS_EX_APPWINDOW as isize));
            SetWindowPos(h, std::ptr::null_mut(), -32000, -32000, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER);
            ShowWindow(h, SW_SHOWNA);
        }
    }

    pub fn show() {
        let Some(h) = main_hwnd() else { return };
        unsafe {
            ShowWindow(h, SW_HIDE);
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            SetWindowLongPtrW(h, GWL_EXSTYLE, (ex & !(WS_EX_TOOLWINDOW as isize)) | WS_EX_APPWINDOW as isize);
            SetWindowPos(h, std::ptr::null_mut(), SAVED_X.load(Ordering::Relaxed) as i32, SAVED_Y.load(Ordering::Relaxed) as i32, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
            ShowWindow(h, SW_RESTORE);
            ShowWindow(h, SW_SHOW);
            SetForegroundWindow(h);
        }
    }
}
