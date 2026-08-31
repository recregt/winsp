use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const WM_TRAYICON: u32 = WM_APP + 1;
pub const ID_TOGGLE: usize = 1001;
pub const ID_EXIT: usize = 1002;
pub const ID_AUTOSTART: usize = 1003;

const TRAY_ICON_ID: u32 = 1;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn tray_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ICON_ID;
    nid
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn add(hwnd: HWND) {
    unsafe {
        let mut nid = tray_icon_data(hwnd);
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = LoadIconW(GetModuleHandleW(std::ptr::null()), 1u16 as _);
        let tip = to_wide("WinSP");
        let len = tip.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tip[..len]);
        Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn remove(hwnd: HWND) {
    unsafe {
        let nid = tray_icon_data(hwnd);
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn show_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        let toggle_label = to_wide("Toggle Search");
        let autostart_label = to_wide("Start with Windows");
        let exit_label = to_wide("Exit");
        let autostart_flags = if super::autostart::is_enabled() {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        AppendMenuW(menu, MF_STRING, ID_TOGGLE, toggle_label.as_ptr());
        AppendMenuW(
            menu,
            autostart_flags,
            ID_AUTOSTART,
            autostart_label.as_ptr(),
        );
        AppendMenuW(menu, MF_STRING, ID_EXIT, exit_label.as_ptr());

        let mut cursor = std::mem::zeroed::<POINT>();
        GetCursorPos(&mut cursor);

        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON,
            cursor.x,
            cursor.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);
    }
}
