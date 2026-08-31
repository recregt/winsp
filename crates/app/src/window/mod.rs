#![cfg(windows)]

mod geometry;
mod input;
mod render;
mod wndproc;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::{Arc, Mutex, OnceLock};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::state::AppState;
use geometry::center_window;
use wndproc::wnd_proc;

pub const WINDOW_WIDTH: i32 = 680;
pub const SEARCH_BAR_HEIGHT: i32 = 64;
pub const ITEM_ROW_HEIGHT: i32 = 54;
pub const PADDING: i32 = 12;
pub(crate) const WINDOW_CLASS_NAME: &str = "WinSP_Spotlight_Window";

static APP_STATE: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

pub fn run_app(state: Arc<Mutex<AppState>>) -> Result<(), String> {
    let _ = APP_STATE.set(state);

    winsp_windows::system::theme::allow_dark_mode_for_app();

    let window_handle = winsp_windows::system::WindowHandle::create(
        WINDOW_CLASS_NAME,
        "WinSP",
        WINDOW_WIDTH,
        SEARCH_BAR_HEIGHT,
        Some(wnd_proc),
    )
    .map_err(|e| format!("failed to create window: {e}"))?;
    window_handle.enable_dark_mode();

    let hwnd = window_handle.hwnd();

    unsafe {
        center_window(hwnd);

        let _ = RegisterHotKey(hwnd, 1, MOD_ALT, VK_SPACE as u32);

        window_handle.add_tray_icon();

        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnregisterHotKey(hwnd, 1);
    }
    Ok(())
}
