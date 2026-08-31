mod tray;

use windows_sys::Win32::Foundation::HWND;

pub use tray::{TrayCommand, WM_TRAYICON};

/// A handle to the application's window, used to perform window-scoped
/// system integration such as dark mode theming and the tray icon.
pub struct WindowHandle {
    hwnd: HWND,
}

impl WindowHandle {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn enable_dark_mode(&self) {
        super::theme::allow_dark_mode_for_window(self.hwnd);
    }

    pub fn add_tray_icon(&self) {
        tray::add(self.hwnd);
    }

    pub fn remove_tray_icon(&self) {
        tray::remove(self.hwnd);
    }

    pub fn show_tray_menu(&self) {
        tray::show_menu(self.hwnd);
    }
}
