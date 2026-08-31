use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_SHOW, SetForegroundWindow, ShowWindow,
};

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

pub fn acquire(mutex_name: &str, window_class_name: &str) -> Option<InstanceGuard> {
    let name = to_wide(mutex_name);
    let (handle, already_exists) = unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        let already_exists =
            std::io::Error::last_os_error().raw_os_error() == Some(ERROR_ALREADY_EXISTS as i32);
        (handle, already_exists)
    };

    if handle.is_null() {
        return None;
    }

    if already_exists {
        unsafe {
            CloseHandle(handle);
        }
        focus_existing_window(window_class_name);
        return None;
    }

    Some(InstanceGuard(handle))
}

fn focus_existing_window(window_class_name: &str) {
    let class_name = to_wide(window_class_name);
    unsafe {
        let existing = FindWindowW(class_name.as_ptr(), std::ptr::null());
        if !existing.is_null() {
            ShowWindow(existing, SW_SHOW);
            SetForegroundWindow(existing);
        }
    }
}
