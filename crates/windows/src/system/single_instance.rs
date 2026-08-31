use super::registry::to_wide;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_SHOW, SetForegroundWindow, ShowWindow,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    const MUTEX_NAME: &str = "WinSpTest_SingleInstance_Mutex";
    const WINDOW_CLASS_NAME: &str = "WinSpTest_NoSuchWindowClass";

    #[test]
    fn detects_collision_and_releases_cleanly() {
        let first = acquire(MUTEX_NAME, WINDOW_CLASS_NAME);
        assert!(first.is_some(), "first acquire should succeed");

        assert!(
            acquire(MUTEX_NAME, WINDOW_CLASS_NAME).is_none(),
            "a second acquire while the first is held should detect the collision"
        );
        assert!(
            acquire(MUTEX_NAME, WINDOW_CLASS_NAME).is_none(),
            "the collision cleanup must not release the mutex the first guard still holds"
        );

        drop(first);

        assert!(
            acquire(MUTEX_NAME, WINDOW_CLASS_NAME).is_some(),
            "after the guard is dropped, acquiring again should succeed"
        );
    }
}
