mod tray;

use crate::system::registry::to_wide;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Dwm::{
    DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, WHITE_BRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, IDC_ARROW, LoadCursorW, LoadIconW, RegisterClassExW,
    WNDCLASSEXW, WNDPROC, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

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

    /// Registers the window class (if needed) and creates the window,
    /// applying WinSP's standard appearance (transient Mica backdrop,
    /// rounded corners).
    pub fn create(
        class_name: &str,
        title: &str,
        width: i32,
        height: i32,
        wnd_proc: WNDPROC,
    ) -> Result<Self, std::io::Error> {
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = to_wide(class_name);
            let title = to_wide(title);

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: wnd_proc,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: LoadIconW(instance, 1u16 as _),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: GetStockObject(WHITE_BRUSH) as _,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: LoadIconW(instance, 1u16 as _),
            };

            // A failed class registration surfaces via CreateWindowExW failing below.
            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                100,
                100,
                width,
                height,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );

            if hwnd.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let handle = Self { hwnd };

            // Cosmetic only; older Windows builds may not support these attributes.
            let backdrop = DWMSBT_TRANSIENTWINDOW;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE as u32,
                &backdrop as *const _ as *const _,
                std::mem::size_of_val(&backdrop) as u32,
            );

            let corner_pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &corner_pref as *const _ as *const _,
                std::mem::size_of_val(&corner_pref) as u32,
            );

            Ok(handle)
        }
    }

    /// The raw window handle, for operations not yet wrapped by this type.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
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
