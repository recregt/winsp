use super::registry::read_dword;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{FARPROC, HMODULE, HWND};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;

/// Reports whether Windows is currently configured to use dark mode.
pub(crate) fn system_uses_dark_mode() -> bool {
    read_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        "AppsUseLightTheme",
    )
    .is_some_and(|v| v == 0)
}

fn uxtheme() -> HMODULE {
    static HANDLE: OnceLock<isize> = OnceLock::new();
    *HANDLE.get_or_init(|| unsafe { LoadLibraryA(c"uxtheme.dll".as_ptr() as *const u8) as isize })
        as HMODULE
}

fn proc_by_ordinal(ordinal: u16) -> FARPROC {
    let module = uxtheme();
    if module.is_null() {
        return None;
    }
    unsafe { GetProcAddress(module, ordinal as usize as *const u8) }
}

/// Opts the current process into dark mode aware control rendering.
pub fn allow_dark_mode_for_app() {
    type AllowDarkModeForApp = unsafe extern "system" fn(bool) -> bool;
    if let Some(f) = proc_by_ordinal(135) {
        let f: AllowDarkModeForApp = unsafe { std::mem::transmute(f) };
        unsafe {
            f(system_uses_dark_mode());
        }
    }
}

/// Enables dark mode rendering for controls owned by the given window.
pub(crate) fn allow_dark_mode_for_window(hwnd: HWND) {
    if unsafe { IsWindow(hwnd) } == 0 {
        return;
    }
    type AllowDarkModeForWindow = unsafe extern "system" fn(HWND, bool) -> bool;
    if let Some(f) = proc_by_ordinal(133) {
        let f: AllowDarkModeForWindow = unsafe { std::mem::transmute(f) };
        unsafe {
            f(hwnd, system_uses_dark_mode());
        }
    }
}
