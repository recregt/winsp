use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_SHOW, SetForegroundWindow, ShowWindow,
};

use crate::window::win32_window::WINDOW_CLASS_NAME;

const MUTEX_NAME: &str = "WinSP_SingleInstance_Mutex";

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

pub struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Attempts to become the single running instance of WinSP.
/// Returns `None` if another instance already holds the lock — in that case,
/// the existing instance's window is brought to the foreground.
pub fn acquire() -> Option<InstanceGuard> {
    let name = to_wide(MUTEX_NAME);
    let (handle, already_exists) = unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        let already_exists =
            std::io::Error::last_os_error().raw_os_error() == Some(ERROR_ALREADY_EXISTS as i32);
        (handle, already_exists)
    };

    if already_exists {
        focus_existing_window();
        return None;
    }

    Some(InstanceGuard(handle))
}

fn focus_existing_window() {
    let class_name = to_wide(WINDOW_CLASS_NAME);
    unsafe {
        let existing = FindWindowW(class_name.as_ptr(), std::ptr::null());
        if !existing.is_null() {
            ShowWindow(existing, SW_SHOW);
            SetForegroundWindow(existing);
        }
    }
}
