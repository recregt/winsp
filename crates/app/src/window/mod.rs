#![cfg(windows)]

mod geometry;
mod render;
mod wndproc;

use std::sync::{Arc, Mutex, OnceLock};

use winsp_windows::window::{Hotkey, Key};

use crate::state::AppState;
use wndproc::handle_message;

pub const WINDOW_WIDTH: i32 = 680;
pub const SEARCH_BAR_HEIGHT: i32 = 64;
pub const ITEM_ROW_HEIGHT: i32 = 54;
pub const PADDING: i32 = 12;
pub(crate) const WINDOW_CLASS_NAME: &str = "WinSP_Spotlight_Window";

static APP_STATE: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();

pub fn run_app(state: Arc<Mutex<AppState>>) -> Result<(), String> {
    let _ = APP_STATE.set(state);

    winsp_windows::system::theme::allow_dark_mode_for_app();

    let window_handle = winsp_windows::window::WindowHandle::create(
        WINDOW_CLASS_NAME,
        "WinSP",
        WINDOW_WIDTH,
        SEARCH_BAR_HEIGHT,
        handle_message,
    )
    .map_err(|e| format!("failed to create window: {e}"))?;
    window_handle.enable_dark_mode();

    window_handle.center(WINDOW_WIDTH, SEARCH_BAR_HEIGHT);
    window_handle.add_tray_icon();
    window_handle.run_message_loop(Hotkey::alt(Key::Space));

    Ok(())
}
