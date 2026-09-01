use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE, LPARAM, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};
use windows::core::HSTRING;

use crate::window::WM_SHOW_REQUEST;

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
            let _ = PostMessageW(Some(existing), WM_SHOW_REQUEST, WPARAM(0), LPARAM(0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LRESULT};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, MSG, PM_REMOVE, PeekMessageW,
        RegisterClassExW, UnregisterClassW, WNDCLASSEXW,
    };
    use windows::core::PCWSTR;

    const MUTEX_NAME: &str = "WinSpTest_SingleInstance_Mutex";
    const WINDOW_CLASS_NAME: &str = "WinSpTest_NoSuchWindowClass";

    unsafe extern "system" fn noop_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn create_test_window(class_name: &HSTRING) -> HWND {
        unsafe {
            let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(noop_wnd_proc),
                hInstance: instance,
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..std::mem::zeroed()
            };
            RegisterClassExW(&wnd_class);
            CreateWindowExW(
                Default::default(),
                class_name,
                PCWSTR::null(),
                Default::default(),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                None,
            )
            .unwrap_or(HWND(std::ptr::null_mut()))
        }
    }

    #[test]
    fn focus_existing_window_posts_a_show_request_to_the_matching_class() {
        let class_name = HSTRING::from("WinSpTest_FocusExistingWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");

        focus_existing_window("WinSpTest_FocusExistingWindow");

        let received = unsafe {
            let mut msg = std::mem::zeroed::<MSG>();
            PeekMessageW(
                &mut msg,
                Some(hwnd),
                WM_SHOW_REQUEST,
                WM_SHOW_REQUEST,
                PM_REMOVE,
            )
            .as_bool()
        };
        assert!(
            received,
            "expected WM_SHOW_REQUEST to be posted to the window matching the class name"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    #[test]
    fn focus_existing_window_is_a_no_op_when_no_window_matches() {
        focus_existing_window("WinSpTest_NoSuchWindowClassEither");
    }

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
