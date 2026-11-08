#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBeep(u_type: u32) -> i32;
}

/// Plays a clean, native Windows notification chime with zero external audio engine overhead.
pub fn play_completion_sound() {
    #[cfg(target_os = "windows")]
    unsafe {
        // 0x00000040 = MB_ICONASTERISK / MB_ICONINFORMATION (gentle chime)
        MessageBeep(0x00000040);
    }
}
