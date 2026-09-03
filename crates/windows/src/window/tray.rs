use num_enum::TryFromPrimitive;
use windows::Win32::Foundation::{HINSTANCE, HWND, POINT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, w};

use super::Anchor;

pub(super) const WM_TRAYICON: u32 = WM_APP + 1;

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum TrayCommand {
    Toggle = 1001,
    Autostart = 1002,
    Exit = 1003,
    ChangeHotkey = 1004,
    PositionTop = 1005,
    PositionCenter = 1006,
}

const TRAY_ICON_ID: u32 = 1;

fn tray_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ICON_ID;
    nid
}

pub(super) fn add(hwnd: HWND) {
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return;
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
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
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

fn checked_flag(is_current: bool) -> MENU_ITEM_FLAGS {
    if is_current {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING | MF_UNCHECKED
    }
}

fn position_flags(current_position: Anchor) -> (MENU_ITEM_FLAGS, MENU_ITEM_FLAGS) {
    (
        checked_flag(current_position == Anchor::Top),
        checked_flag(current_position == Anchor::Center),
    )
}

pub(super) fn show_menu(hwnd: HWND, current_position: Anchor) {
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return;
    }
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        let autostart_flags = checked_flag(crate::system::autostart::is_enabled());
        let (top_flags, center_flags) = position_flags(current_position);
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            TrayCommand::Toggle as usize,
            w!("Toggle Search"),
        );
        let _ = AppendMenuW(
            menu,
            autostart_flags,
            TrayCommand::Autostart as usize,
            w!("Start with Windows"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            TrayCommand::ChangeHotkey as usize,
            w!("Change Hotkey…"),
        );
        let _ = AppendMenuW(
            menu,
            top_flags,
            TrayCommand::PositionTop as usize,
            w!("Position: Top"),
        );
        let _ = AppendMenuW(
            menu,
            center_flags,
            TrayCommand::PositionCenter as usize,
            w!("Position: Center"),
        );
        let _ = AppendMenuW(menu, MF_STRING, TrayCommand::Exit as usize, w!("Exit"));

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
    use windows::core::HSTRING;

    #[test]
    fn position_flags_checks_only_the_current_anchors_item() {
        let (top, center) = position_flags(Anchor::Top);
        assert_eq!(top, MF_STRING | MF_CHECKED);
        assert_eq!(center, MF_STRING | MF_UNCHECKED);

        let (top, center) = position_flags(Anchor::Center);
        assert_eq!(top, MF_STRING | MF_UNCHECKED);
        assert_eq!(center, MF_STRING | MF_CHECKED);
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
}
