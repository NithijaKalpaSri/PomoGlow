#![windows_subsystem = "windows"]
//! Tiny self-contained installer / uninstaller for Pomoglow (no external dependencies at runtime).
//!
//!   Pomoglow-Setup.exe            install (asks a couple of questions)
//!   Pomoglow-Setup.exe /S         silent install (+ desktop shortcut)
//!   uninstall.exe                 uninstall (installed copy of this program)

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const APP_Z: &[u8] = include_bytes!("../app.z");
const NAME: &str = "Pomoglow";
const VERSION: &str = "0.2.0";
const NO_WINDOW: u32 = 0x0800_0000;
const UNINSTALL_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Pomoglow";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn ask(text: &str, buttons: u32) -> i32 {
    unsafe { MessageBoxW(std::ptr::null_mut(), wide(text).as_ptr(), wide(&format!("{NAME} Setup")).as_ptr(), buttons | MB_ICONINFORMATION | MB_SETFOREGROUND) }
}

fn say(text: &str) {
    ask(text, MB_OK);
}

fn yes(text: &str) -> bool {
    ask(text, MB_YESNO) == IDYES
}

fn run_hidden(prog: &str, args: &[&str]) -> bool {
    Command::new(prog).args(args).creation_flags(NO_WINDOW).status().map(|s| s.success()).unwrap_or(false)
}

fn install_dir() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| r"C:\Users\Public".into())).join("Programs").join(NAME)
}

fn start_menu_link() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join(r"Microsoft\Windows\Start Menu\Programs").join(format!("{NAME}.lnk"))
}

fn desktop_link() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join("Desktop").join(format!("{NAME}.lnk"))
}

fn make_shortcut(link: &Path, target: &Path, dir: &Path) {
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.IconLocation='{},0';$s.Description='Pomoglow - pastel Pomodoro timer';$s.Save()",
        link.display(),
        target.display(),
        dir.display(),
        target.display()
    );
    run_hidden("powershell", &["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script]);
}

fn kill_running() {
    run_hidden("taskkill", &["/F", "/IM", "pomoglow.exe"]);
    std::thread::sleep(std::time::Duration::from_millis(400));
}

fn reg_add(name: &str, kind: &str, value: &str) {
    run_hidden("reg", &["add", UNINSTALL_KEY, "/v", name, "/t", kind, "/d", value, "/f"]);
}

fn install(silent: bool) {
    if !silent && !yes(&format!("Install {NAME} {VERSION}?\n\nIt will be installed for your user account (no administrator rights needed).")) {
        return;
    }
    let desktop = silent || yes("Create a desktop shortcut?");
    let dir = install_dir();
    kill_running();
    if std::fs::create_dir_all(&dir).is_err() {
        say("Could not create the installation folder.");
        return;
    }
    let Ok(app) = miniz_oxide::inflate::decompress_to_vec_zlib(APP_Z) else {
        say("The installer is damaged. Please download it again.");
        return;
    };
    let exe = dir.join("pomoglow.exe");
    if std::fs::write(&exe, app).is_err() {
        say("Could not write files. Is Pomoglow still running?");
        return;
    }
    let uninstaller = dir.join("uninstall.exe");
    if let Ok(me) = std::env::current_exe() {
        let _ = std::fs::copy(me, &uninstaller);
    }

    make_shortcut(&start_menu_link(), &exe, &dir);
    if desktop {
        make_shortcut(&desktop_link(), &exe, &dir);
    }

    reg_add("DisplayName", "REG_SZ", NAME);
    reg_add("DisplayVersion", "REG_SZ", VERSION);
    reg_add("Publisher", "REG_SZ", NAME);
    reg_add("InstallLocation", "REG_SZ", &dir.display().to_string());
    reg_add("DisplayIcon", "REG_SZ", &exe.display().to_string());
    reg_add("UninstallString", "REG_SZ", &format!("\"{}\"", uninstaller.display()));
    reg_add("NoModify", "REG_DWORD", "1");
    reg_add("NoRepair", "REG_DWORD", "1");

    if silent || yes(&format!("{NAME} was installed successfully.\n\nLaunch it now?")) {
        let _ = Command::new(&exe).current_dir(&dir).spawn();
    }
}

fn uninstall(silent: bool) {
    if !silent && !yes(&format!("Remove {NAME} from this computer?\n\nYour saved profile and stats are kept unless you choose to delete them next.")) {
        return;
    }
    let wipe = !silent && yes("Also delete your saved profile and statistics?");
    kill_running();
    let _ = std::fs::remove_file(start_menu_link());
    let _ = std::fs::remove_file(desktop_link());
    run_hidden("reg", &["delete", UNINSTALL_KEY, "/f"]);
    if wipe {
        if let Ok(a) = std::env::var("APPDATA") {
            let _ = std::fs::remove_dir_all(PathBuf::from(a).join(NAME));
        }
    }
    let dir = install_dir();
    let _ = std::fs::remove_file(dir.join("pomoglow.exe"));
    // This program is running from the install folder, so let a helper remove it after we exit.
    let _ = Command::new("cmd")
        .raw_arg(format!("/C ping 127.0.0.1 -n 3 >nul & del /f /q \"{0}\\uninstall.exe\" & rmdir \"{0}\"", dir.display()))
        .creation_flags(NO_WINDOW)
        .spawn();
    if !silent {
        say(&format!("{NAME} was removed."));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let silent = args.iter().any(|a| a.eq_ignore_ascii_case("/S"));
    let exe_name = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase())).unwrap_or_default();
    if exe_name.starts_with("uninstall") || args.iter().any(|a| a.eq_ignore_ascii_case("--uninstall")) {
        uninstall(silent);
    } else {
        install(silent);
    }
}
