use windows::Win32::Foundation::{HINSTANCE, HWND, POINT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, PCWSTR, w};

pub(super) const WM_TRAYICON: u32 = WM_APP + 1;

pub(super) fn taskbar_created_message() -> u32 {
    static MSG_ID: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *MSG_ID.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

pub struct MenuItem<'a> {
    pub id: usize,
    pub label: &'a str,
    pub checked: bool,
}

const TRAY_ICON_ID: u32 = 1;

fn tray_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ICON_ID;
    nid
}

pub(super) fn add(hwnd: HWND) -> bool {
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return false;
    }
    unsafe {
        let mut nid = tray_icon_data(hwnd);
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        #[allow(clippy::manual_dangling_ptr)]
        let app_icon = PCWSTR(1 as *const u16);
        nid.hIcon = LoadIconW(Some(instance), app_icon).unwrap_or(HICON(std::ptr::null_mut()));
        let tip: Vec<u16> = "WinSP".encode_utf16().collect();
        let len = tip.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tip[..len]);
        Shell_NotifyIconW(NIM_ADD, &nid).as_bool()
    }
}

pub(super) fn remove(hwnd: HWND) {
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return;
    }
    unsafe {
        let nid = tray_icon_data(hwnd);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn menu_flags(checked: bool) -> MENU_ITEM_FLAGS {
    if checked {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    }
}

pub(super) fn show_menu(hwnd: HWND, items: &[MenuItem]) {
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return;
    }
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };

        for item in items {
            let label = HSTRING::from(item.label);
            let _ = AppendMenuW(menu, menu_flags(item.checked), item.id, &label);
        }

        let mut cursor = std::mem::MaybeUninit::<POINT>::uninit();
        let cursor = if GetCursorPos(cursor.as_mut_ptr()).is_ok() {
            cursor.assume_init()
        } else {
            POINT { x: 0, y: 0 }
        };

        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, cursor.x, cursor.y, None, hwnd, None);
        let _ = DestroyMenu(menu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassExW, UnregisterClassW,
        WNDCLASSEXW,
    };

    #[test]
    fn menu_flags_sets_the_checked_bit_only_when_checked() {
        assert_eq!(menu_flags(true), MF_STRING | MF_CHECKED);
        assert_eq!(menu_flags(false), MF_STRING);
    }

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
    fn add_and_remove_succeed_on_a_real_window() {
        let class_name = HSTRING::from("WinSpTest_TrayWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");

        unsafe {
            let added = Shell_NotifyIconW(NIM_ADD, &{
                let mut nid = tray_icon_data(hwnd);
                nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
                nid.uCallbackMessage = WM_TRAYICON;
                nid
            });
            assert!(added.as_bool(), "Shell_NotifyIconW(NIM_ADD) should succeed");

            let removed = Shell_NotifyIconW(NIM_DELETE, &tray_icon_data(hwnd));
            assert!(
                removed.as_bool(),
                "Shell_NotifyIconW(NIM_DELETE) should succeed"
            );

            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    #[test]
    fn add_reports_failure_for_an_invalid_window() {
        assert!(
            !add(HWND(std::ptr::null_mut())),
            "add() must report failure instead of silently ignoring an invalid window"
        );
    }

    #[test]
    fn add_reports_success_on_a_real_window() {
        let class_name = HSTRING::from("WinSpTest_TrayAddReturnWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");

        assert!(add(hwnd), "add() should report success on a real window");

        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &tray_icon_data(hwnd));
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }
}
