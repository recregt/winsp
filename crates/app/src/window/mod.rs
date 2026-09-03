#![cfg(windows)]

mod controller;
mod hotkey;
mod layout;
mod view;

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};

use winsp_windows::window::{Hotkey, Key, Modifiers, Window};

use crate::config::Settings;
use crate::state::AppState;
use controller::handle_event;
use layout::to_anchor;

pub const WINDOW_WIDTH: i32 = 680;
pub const SEARCH_BAR_HEIGHT: i32 = 64;
pub const ITEM_ROW_HEIGHT: i32 = 54;
pub const PADDING: i32 = 12;
pub(crate) const WINDOW_CLASS_NAME: &str = "WinSP_Spotlight_Window";
pub(crate) const CATALOG_READY_EVENT: u32 = 1;

static APP_STATE: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();
static RECONCILE_TX: OnceLock<Sender<()>> = OnceLock::new();
static SETTINGS: OnceLock<Mutex<Settings>> = OnceLock::new();

pub fn set_reconcile_hook(tx: Sender<()>) {
    let _ = RECONCILE_TX.set(tx);
}

pub fn run_app(state: Arc<Mutex<AppState>>) -> Result<(), String> {
    let _ = APP_STATE.set(state);

    let settings = Settings::load();
    if let Err(err) = settings.save() {
        eprintln!("failed to save settings: {err}");
    }

    let modifiers = Modifiers {
        ctrl: settings.hotkey.ctrl,
        shift: settings.hotkey.shift,
        alt: settings.hotkey.alt,
        win: settings.hotkey.win,
    };
    let hotkey = Hotkey::new(modifiers, Key::Other(settings.hotkey.vk));
    let anchor = to_anchor(settings.position);
    let _ = SETTINGS.set(Mutex::new(settings));

    winsp_windows::system::theme::allow_dark_mode_for_app();

    let window_handle = Window::create(
        WINDOW_CLASS_NAME,
        "WinSP",
        WINDOW_WIDTH,
        SEARCH_BAR_HEIGHT,
        handle_event,
    )
    .map_err(|e| format!("failed to create window: {e}"))?;
    window_handle.enable_dark_mode();

    window_handle.center(WINDOW_WIDTH, SEARCH_BAR_HEIGHT, anchor);
    if !window_handle.add_tray_icon() {
        winsp_windows::system::toast::show(
            "WinSP",
            "Couldn't add the tray icon. Use the hotkey to open WinSP.",
        );
    }
    window_handle.run_message_loop(hotkey);

    Ok(())
}
