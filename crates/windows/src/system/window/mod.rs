mod tray;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};

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

        let dark_mode: i32 = super::theme::system_uses_dark_mode() as i32;
        unsafe {
            let _ = DwmSetWindowAttribute(
                self.hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                &dark_mode as *const _ as *const _,
                std::mem::size_of_val(&dark_mode) as u32,
            );
        }
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
