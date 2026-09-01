mod canvas;
mod message;
mod tray;

use std::sync::OnceLock;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    EndPaint, GetStockObject, InvalidateRect, PAINTSTRUCT, SRCCOPY, SelectObject, SetBkMode,
    TRANSPARENT, WHITE_BRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetClientRect, GetMessageW, GetSystemMetrics, GetWindowRect, HCURSOR, HICON, HWND_TOPMOST,
    IDC_ARROW, IsWindowVisible, LoadCursorW, LoadIconW, MSG, PostQuitMessage, RegisterClassExW,
    SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE, SetForegroundWindow,
    SetWindowPos, ShowWindow, TranslateMessage, WM_CHAR, WM_COMMAND, WM_DESTROY, WM_HOTKEY,
    WM_KEYDOWN, WM_KILLFOCUS, WM_PAINT, WM_RBUTTONUP, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::{HSTRING, PCWSTR};

pub use canvas::{Canvas, Color, Font, FontGuard, FontWeight, Rect, register_embedded_font};
pub use message::{Hotkey, Key, Message};
pub use tray::TrayCommand;

#[cfg(feature = "test-support")]
pub use canvas::testing;

static HANDLER: OnceLock<fn(&Window, Message)> = OnceLock::new();

unsafe extern "system" fn dispatch(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(handler) = HANDLER.get() else {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    let window = Window::new(hwnd);
    match msg {
        WM_HOTKEY => {
            handler(&window, Message::Hotkey);
            LRESULT(0)
        }
        tray::WM_TRAYICON => {
            if lparam.0 as u32 == WM_RBUTTONUP {
                handler(&window, Message::TrayRightClick);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            if let Ok(cmd) = TrayCommand::try_from(wparam.0 & 0xffff) {
                handler(&window, Message::Command(cmd));
            }
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            handler(&window, Message::KillFocus);
            LRESULT(0)
        }
        WM_CHAR => {
            if let Some(c) = message::decode_wm_char(wparam.0 as u16) {
                if !c.is_control() {
                    handler(&window, Message::Char(c));
                }
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            handler(
                &window,
                Message::KeyDown(Key::from_vk(
                    windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(wparam.0 as u16),
                )),
            );
            LRESULT(0)
        }
        WM_PAINT => {
            handler(&window, Message::Paint);
            LRESULT(0)
        }
        WM_DESTROY => {
            tray::remove(hwnd);
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub struct Window {
    hwnd: HWND,
}

impl Window {
    fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn create(
        class_name: &str,
        title: &str,
        width: i32,
        height: i32,
        handler: fn(&Window, Message),
    ) -> Result<Self, std::io::Error> {
        let _ = HANDLER.set(handler);
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

            let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
            let class_name = HSTRING::from(class_name);
            let title = HSTRING::from(title);

            #[allow(clippy::manual_dangling_ptr)]
            let app_icon = PCWSTR(1 as *const u16);
            let icon = LoadIconW(Some(instance), app_icon).unwrap_or(HICON(std::ptr::null_mut()));

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(dispatch),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: icon,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or(HCURSOR(std::ptr::null_mut())),
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(GetStockObject(WHITE_BRUSH).0),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                hIconSm: icon,
            };

            RegisterClassExW(&wnd_class);

            let Ok(hwnd) = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                &class_name,
                &title,
                WS_POPUP,
                100,
                100,
                width,
                height,
                None,
                None,
                Some(instance),
                None,
            ) else {
                return Err(std::io::Error::last_os_error());
            };

            let handle = Self { hwnd };

            let backdrop = DWMSBT_TRANSIENTWINDOW;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop as *const _ as *const _,
                std::mem::size_of_val(&backdrop) as u32,
            );

            let corner_pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_pref as *const _ as *const _,
                std::mem::size_of_val(&corner_pref) as u32,
            );

            Ok(handle)
        }
    }

    pub fn run_message_loop(&self, hotkey: Hotkey) {
        const HOTKEY_ID: i32 = 1;
        unsafe {
            if RegisterHotKey(Some(self.hwnd), HOTKEY_ID, hotkey.modifiers, hotkey.vk).is_err() {
                notify_hotkey_registration_failed(std::io::Error::last_os_error());
            }

            let mut msg = std::mem::zeroed::<MSG>();
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnregisterHotKey(Some(self.hwnd), HOTKEY_ID);
        }
    }

    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.hwnd) }.as_bool()
    }

    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(self.hwnd);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn close(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }

    pub fn invalidate(&self) {
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, true);
        }
    }

    pub fn paint(&self, draw: impl FnOnce(&Canvas, Rect)) {
        unsafe {
            let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
            let hdc = BeginPaint(self.hwnd, &mut ps);

            let mut client_rect = std::mem::zeroed::<RECT>();
            let _ = GetClientRect(self.hwnd, &mut client_rect);

            let mem_dc = CreateCompatibleDC(Some(hdc));
            let mem_bmp = CreateCompatibleBitmap(hdc, client_rect.right, client_rect.bottom);
            let old_bmp = SelectObject(mem_dc, mem_bmp.into());
            SetBkMode(mem_dc, TRANSPARENT);

            let canvas = Canvas::new(mem_dc);
            draw(
                &canvas,
                Rect {
                    left: client_rect.left,
                    top: client_rect.top,
                    right: client_rect.right,
                    bottom: client_rect.bottom,
                },
            );

            let _ = BitBlt(
                hdc,
                0,
                0,
                client_rect.right,
                client_rect.bottom,
                Some(mem_dc),
                0,
                0,
                SRCCOPY,
            );

            SelectObject(mem_dc, old_bmp);
            let _ = DeleteObject(mem_bmp.into());
            let _ = DeleteDC(mem_dc);

            let _ = EndPaint(self.hwnd, &ps);
        }
    }

    pub fn center(&self, width: i32, height: i32) {
        unsafe {
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);

            let x = (screen_width - width) / 2;
            let y = screen_height / 4;

            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE,
            );
        }
    }

    pub fn resize(&self, width: i32, height: i32) {
        unsafe {
            let mut rect = std::mem::zeroed::<RECT>();
            let _ = GetWindowRect(self.hwnd, &mut rect);
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                rect.left,
                rect.top,
                width,
                height,
                SWP_NOMOVE | SWP_NOACTIVATE,
            );
        }
    }

    pub fn enable_dark_mode(&self) {
        crate::system::theme::allow_dark_mode_for_window(self.hwnd);

        let dark_mode: i32 = crate::system::theme::system_uses_dark_mode() as i32;
        unsafe {
            let _ = DwmSetWindowAttribute(
                self.hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const _ as *const _,
                std::mem::size_of_val(&dark_mode) as u32,
            );
        }
    }

    pub fn add_tray_icon(&self) {
        tray::add(self.hwnd);
    }

    pub fn show_tray_menu(&self) {
        tray::show_menu(self.hwnd);
    }
}

fn notify_hotkey_registration_failed(error: std::io::Error) {
    crate::system::toast::show(
        "WinSP",
        &format!("Failed to register global hotkey: {error}"),
    );
}
