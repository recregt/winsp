use super::registry::read_dword;
use std::sync::OnceLock;
use windows::Win32::Foundation::{FARPROC, HMODULE, HWND};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::Win32::UI::WindowsAndMessaging::IsWindow;
use windows::core::{PCSTR, s};
use windows_registry::CURRENT_USER;

pub(crate) fn system_uses_dark_mode() -> bool {
    read_dword(
        CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        "AppsUseLightTheme",
    )
    .is_some_and(|v| v == 0)
}

fn uxtheme() -> HMODULE {
    static HANDLE: OnceLock<isize> = OnceLock::new();
    let ptr = *HANDLE.get_or_init(|| unsafe {
        LoadLibraryA(s!("uxtheme.dll"))
            .map(|h| h.0 as isize)
            .unwrap_or(0)
    });
    HMODULE(ptr as *mut core::ffi::c_void)
}

fn proc_by_ordinal(ordinal: u16) -> FARPROC {
    let module = uxtheme();
    if module.is_invalid() {
        return None;
    }
    unsafe { GetProcAddress(module, PCSTR(ordinal as usize as *const u8)) }
}

pub fn allow_dark_mode_for_app() {
    type AllowDarkModeForApp = unsafe extern "system" fn(bool) -> bool;
    if let Some(f) = proc_by_ordinal(135) {
        let f: AllowDarkModeForApp = unsafe { std::mem::transmute(f) };
        unsafe {
            f(system_uses_dark_mode());
        }
    }
}

pub(crate) fn allow_dark_mode_for_window(hwnd: HWND) {
    let exists = unsafe { IsWindow(Some(hwnd)) };
    if !exists.as_bool() {
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
