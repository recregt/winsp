use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_SHOW, SetForegroundWindow, ShowWindow,
};
use windows::core::HSTRING;

pub struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub fn acquire(mutex_name: &str, window_class_name: &str) -> Option<InstanceGuard> {
    let name = HSTRING::from(mutex_name);
    let handle = unsafe { CreateMutexW(None, false, &name) }.ok()?;
    let already_exists =
        std::io::Error::last_os_error().raw_os_error() == Some(ERROR_ALREADY_EXISTS.0 as i32);

    if already_exists {
        unsafe {
            let _ = CloseHandle(handle);
        }
        focus_existing_window(window_class_name);
        return None;
    }

    Some(InstanceGuard(handle))
}

fn focus_existing_window(window_class_name: &str) {
    let class_name = HSTRING::from(window_class_name);
    unsafe {
        if let Ok(existing) = FindWindowW(&class_name, None) {
            let _ = ShowWindow(existing, SW_SHOW);
            let _ = SetForegroundWindow(existing);
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
