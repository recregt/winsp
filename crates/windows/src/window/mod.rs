mod canvas;
mod icon_cache;
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
use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GWLP_USERDATA, GetClientRect, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowRect, HCURSOR, HICON, HWND_TOPMOST, IDC_ARROW, IsWindowVisible,
    LoadCursorW, LoadIconW, MSG, PM_REMOVE, PeekMessageW, PostMessageW, PostQuitMessage,
    RegisterClassExW, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage,
    WM_APP, WM_CHAR, WM_COMMAND, WM_DESTROY, WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_KILLFOCUS,
    WM_NCCREATE, WM_PAINT, WM_RBUTTONUP, WM_SYSKEYDOWN, WNDCLASSEXW, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{HSTRING, PCWSTR};

pub use canvas::{Canvas, Color, Font, FontGuard, FontWeight, Rect, register_embedded_font};
pub use icon_cache::{icon_for_path, mark_paint_started};
pub use message::{Hotkey, HotkeySlot, Key, Message, Modifiers};
pub use tray::TrayCommand;

#[cfg(feature = "test-support")]
pub use canvas::testing;

pub(crate) const WM_SHOW_REQUEST: u32 = WM_APP + 2;
const WM_CATALOG_READY: u32 = WM_APP + 3;

static MAIN_HWND: OnceLock<isize> = OnceLock::new();

pub fn notify_catalog_ready() {
    if let Some(&raw) = MAIN_HWND.get() {
        unsafe {
            let _ = PostMessageW(
                Some(HWND(raw as *mut _)),
                WM_CATALOG_READY,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

unsafe extern "system" fn dispatch(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let create_struct = lparam.0 as *const CREATESTRUCTW;
        if let Some(create_struct) = unsafe { create_struct.as_ref() } {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create_struct.lpCreateParams as isize)
            };
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }

    let handler_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if handler_ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let handler: fn(&Window, Message) = unsafe { std::mem::transmute(handler_ptr as usize) };
    let window = Window::new(hwnd);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match msg {
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
            if let Some(c) = message::decode_wm_char(wparam.0 as u16)
                && !c.is_control()
            {
                handler(&window, Message::Char(c));
            }
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            handler(
                &window,
                Message::KeyDown(
                    Key::from_vk(windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(
                        wparam.0 as u16,
                    )),
                    message::current_modifiers(),
                ),
            );
            LRESULT(0)
        }
        WM_SHOW_REQUEST => {
            handler(&window, Message::ShowRequest);
            LRESULT(0)
        }
        WM_CATALOG_READY => {
            handler(&window, Message::CatalogReady);
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
        _ if msg == tray::taskbar_created_message() => {
            handler(&window, Message::TaskbarRestarted);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }));

    result.unwrap_or_else(|_| unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Top,
    Center,
}

impl Anchor {
    fn position_for(self, width: i32, height: i32) -> (i32, i32) {
        unsafe {
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);

            let x = (screen_width - width) / 2;
            let y = match self {
                Anchor::Top => screen_height / 4,
                Anchor::Center => (screen_height - height) / 2,
            };
            (x, y)
        }
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
        let handler_ptr = handler as *const () as *mut std::ffi::c_void;
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
                Some(handler_ptr as *const _),
            ) else {
                return Err(std::io::Error::last_os_error());
            };

            let handle = Self { hwnd };
            icon_cache::register_repaint_target(hwnd);
            let _ = MAIN_HWND.set(hwnd.0 as isize);

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

    pub fn register_hotkey(&self, slot: HotkeySlot, hotkey: Hotkey) -> bool {
        unsafe {
            RegisterHotKey(
                Some(self.hwnd),
                slot.id(),
                hotkey.modifiers | MOD_NOREPEAT,
                hotkey.vk,
            )
            .is_ok()
        }
    }

    pub fn unregister_hotkey(&self, slot: HotkeySlot) {
        unsafe {
            let _ = UnregisterHotKey(Some(self.hwnd), slot.id());
        }
    }

    pub fn run_message_loop(&self, hotkey: Hotkey) {
        if !self.register_hotkey(HotkeySlot::Primary, hotkey) {
            notify_hotkey_registration_failed(std::io::Error::last_os_error());
        }

        unsafe {
            let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
            while GetMessageW(msg.as_mut_ptr(), None, 0, 0).0 > 0 {
                let msg = msg.assume_init_ref();
                let _ = TranslateMessage(msg);
                DispatchMessageW(msg);
            }
        }

        self.unregister_hotkey(HotkeySlot::Primary);
    }

    pub fn discard_pending_char(&self) {
        unsafe {
            let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
            while PeekMessageW(
                msg.as_mut_ptr(),
                Some(self.hwnd),
                WM_CHAR,
                WM_CHAR,
                PM_REMOVE,
            )
            .as_bool()
            {}
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
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn paint(&self, draw: impl FnOnce(&Canvas, Rect)) {
        unsafe {
            let mut ps = std::mem::MaybeUninit::<PAINTSTRUCT>::uninit();
            let hdc = BeginPaint(self.hwnd, ps.as_mut_ptr());
            if hdc.is_invalid() {
                return;
            }

            let mut client_rect = std::mem::MaybeUninit::<RECT>::uninit();
            let client_rect = if GetClientRect(self.hwnd, client_rect.as_mut_ptr()).is_ok() {
                client_rect.assume_init()
            } else {
                RECT::default()
            };

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

            let _ = EndPaint(self.hwnd, ps.as_ptr());
        }
    }

    pub fn center(&self, width: i32, height: i32, anchor: Anchor) {
        let (x, y) = anchor.position_for(width, height);
        unsafe {
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
            let mut rect = std::mem::MaybeUninit::<RECT>::uninit();
            let rect = if GetWindowRect(self.hwnd, rect.as_mut_ptr()).is_ok() {
                rect.assume_init()
            } else {
                RECT::default()
            };
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

    pub fn reposition(&self, anchor: Anchor) {
        unsafe {
            let mut rect = std::mem::MaybeUninit::<RECT>::uninit();
            let rect = if GetWindowRect(self.hwnd, rect.as_mut_ptr()).is_ok() {
                rect.assume_init()
            } else {
                RECT::default()
            };
            let (x, y) = anchor.position_for(rect.right - rect.left, rect.bottom - rect.top);
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
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

    pub fn add_tray_icon(&self) -> bool {
        tray::add(self.hwnd)
    }

    pub fn show_tray_menu(&self, current_position: Anchor) {
        tray::show_menu(self.hwnd, current_position);
    }
}

fn notify_hotkey_registration_failed(error: std::io::Error) {
    crate::system::toast::show(
        "WinSP",
        &format!("Failed to register global hotkey: {error}"),
    );
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

    const VK_F13: u16 = 0x7C;
    const VK_F14: u16 = 0x7D;

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
    fn erase_background_is_suppressed_to_avoid_a_white_flash() {
        let result = unsafe {
            dispatch(
                HWND(std::ptr::null_mut()),
                WM_ERASEBKGND,
                WPARAM(0),
                LPARAM(0),
            )
        };
        assert_eq!(result, LRESULT(1));
    }

    static TASKBAR_RESTARTED_CALLED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    fn taskbar_restart_test_handler(_window: &Window, message: Message) {
        if matches!(message, Message::TaskbarRestarted) {
            TASKBAR_RESTARTED_CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn taskbar_created_message_re_adds_the_tray_icon() {
        TASKBAR_RESTARTED_CALLED.store(false, std::sync::atomic::Ordering::SeqCst);

        let class_name = HSTRING::from("WinSpTest_TaskbarRestartWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");

        unsafe {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                taskbar_restart_test_handler as *const () as isize,
            )
        };

        let _ = unsafe { dispatch(hwnd, tray::taskbar_created_message(), WPARAM(0), LPARAM(0)) };
        assert!(
            TASKBAR_RESTARTED_CALLED.load(std::sync::atomic::Ordering::SeqCst),
            "the handler must receive TaskbarRestarted when Explorer's taskbar-created message arrives"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    static SURVIVED_HANDLER_CALLED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    fn panicking_test_handler(_window: &Window, message: Message) {
        if matches!(message, Message::TrayRightClick) {
            panic!("deliberate panic to prove dispatch recovers from it");
        }
        SURVIVED_HANDLER_CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    #[test]
    fn dispatch_survives_a_panicking_handler_and_keeps_working() {
        let class_name = HSTRING::from("WinSpTest_DispatchPanicWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");

        unsafe {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                panicking_test_handler as *const () as isize,
            )
        };

        let panicked_result = unsafe {
            dispatch(
                hwnd,
                tray::WM_TRAYICON,
                WPARAM(0),
                LPARAM(WM_RBUTTONUP as isize),
            )
        };
        assert_eq!(panicked_result, LRESULT(0));

        SURVIVED_HANDLER_CALLED.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = unsafe { dispatch(hwnd, WM_HOTKEY, WPARAM(0), LPARAM(0)) };
        assert!(
            SURVIVED_HANDLER_CALLED.load(std::sync::atomic::Ordering::SeqCst),
            "the handler must still run on a later message after an earlier one panicked"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    static FIRST_WINDOW_HANDLER_CALLED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    static SECOND_WINDOW_HANDLER_CALLED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    fn first_window_test_handler(_window: &Window, _message: Message) {
        FIRST_WINDOW_HANDLER_CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn second_window_test_handler(_window: &Window, _message: Message) {
        SECOND_WINDOW_HANDLER_CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    #[test]
    fn a_recreated_window_uses_its_own_handler_not_a_stale_one() {
        FIRST_WINDOW_HANDLER_CALLED.store(false, std::sync::atomic::Ordering::SeqCst);
        SECOND_WINDOW_HANDLER_CALLED.store(false, std::sync::atomic::Ordering::SeqCst);

        let first = Window::create(
            "WinSpTest_FirstRecreatedWindow",
            "first",
            10,
            10,
            first_window_test_handler,
        )
        .expect("first window creation should succeed");
        let second = Window::create(
            "WinSpTest_SecondRecreatedWindow",
            "second",
            10,
            10,
            second_window_test_handler,
        )
        .expect("second window creation should succeed");

        let _ = unsafe { dispatch(second.hwnd, WM_HOTKEY, WPARAM(0), LPARAM(0)) };
        assert!(
            SECOND_WINDOW_HANDLER_CALLED.load(std::sync::atomic::Ordering::SeqCst),
            "the second window's own handler should run for its messages"
        );
        assert!(
            !FIRST_WINDOW_HANDLER_CALLED.load(std::sync::atomic::Ordering::SeqCst),
            "the first window's stale handler must not run for the second window's messages"
        );

        first.close();
        second.close();
    }

    #[test]
    fn primary_and_secondary_slots_hold_independent_registrations() {
        let class_name = HSTRING::from("WinSpTest_HotkeySlotsWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");
        let window = Window::new(hwnd);

        let primary = Hotkey::new(
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
            Key::Other(VK_F13),
        );
        let secondary = Hotkey::new(
            Modifiers {
                shift: true,
                ..Default::default()
            },
            Key::Other(VK_F14),
        );

        assert!(window.register_hotkey(HotkeySlot::Primary, primary));
        assert!(window.register_hotkey(HotkeySlot::Secondary, secondary));

        window.unregister_hotkey(HotkeySlot::Primary);
        window.unregister_hotkey(HotkeySlot::Secondary);

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    #[test]
    fn discard_pending_char_removes_a_queued_char_message() {
        let class_name = HSTRING::from("WinSpTest_DiscardPendingCharWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");
        let window = Window::new(hwnd);

        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CHAR, WPARAM('a' as usize), LPARAM(0));
        }

        window.discard_pending_char();

        let still_pending = unsafe {
            let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
            PeekMessageW(msg.as_mut_ptr(), Some(hwnd), WM_CHAR, WM_CHAR, PM_REMOVE).as_bool()
        };
        assert!(
            !still_pending,
            "expected the queued WM_CHAR to have been discarded"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    #[test]
    fn discard_pending_char_leaves_unrelated_messages_alone() {
        let class_name = HSTRING::from("WinSpTest_DiscardPendingCharUnrelatedWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");
        let window = Window::new(hwnd);

        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_APP, WPARAM(0), LPARAM(0));
        }

        window.discard_pending_char();

        let still_pending = unsafe {
            let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
            PeekMessageW(msg.as_mut_ptr(), Some(hwnd), WM_APP, WM_APP, PM_REMOVE).as_bool()
        };
        assert!(
            still_pending,
            "discard_pending_char should not remove messages other than WM_CHAR"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    #[test]
    fn position_for_top_horizontally_centers_and_anchors_near_the_top() {
        let (x, y) = Anchor::Top.position_for(680, 64);
        unsafe {
            assert_eq!(x, (GetSystemMetrics(SM_CXSCREEN) - 680) / 2);
            assert_eq!(y, GetSystemMetrics(SM_CYSCREEN) / 4);
        }
    }

    #[test]
    fn position_for_center_vertically_centers_using_the_given_height() {
        let (x, y) = Anchor::Center.position_for(680, 400);
        unsafe {
            assert_eq!(x, (GetSystemMetrics(SM_CXSCREEN) - 680) / 2);
            assert_eq!(y, (GetSystemMetrics(SM_CYSCREEN) - 400) / 2);
        }
    }

    #[test]
    fn top_and_center_agree_on_x_but_differ_on_y() {
        let (top_x, top_y) = Anchor::Top.position_for(680, 64);
        let (center_x, center_y) = Anchor::Center.position_for(680, 64);
        assert_eq!(top_x, center_x);
        assert_ne!(top_y, center_y);
    }

    #[test]
    fn center_places_the_window_at_the_anchors_computed_position() {
        let class_name = HSTRING::from("WinSpTest_CenterWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");
        let window = Window::new(hwnd);

        window.center(680, 64, Anchor::Center);

        let (expected_x, expected_y) = Anchor::Center.position_for(680, 64);
        let mut rect = std::mem::MaybeUninit::<RECT>::uninit();
        let rect = unsafe {
            GetWindowRect(hwnd, rect.as_mut_ptr()).unwrap();
            rect.assume_init()
        };
        assert_eq!(rect.left, expected_x);
        assert_eq!(rect.top, expected_y);
        assert_eq!(rect.right - rect.left, 680);
        assert_eq!(rect.bottom - rect.top, 64);

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }

    #[test]
    fn reposition_moves_the_window_without_changing_its_size() {
        let class_name = HSTRING::from("WinSpTest_RepositionWindow");
        let hwnd = create_test_window(&class_name);
        assert!(!hwnd.is_invalid(), "test window creation should succeed");
        let window = Window::new(hwnd);

        window.center(680, 64, Anchor::Top);
        window.resize(680, 400);

        window.reposition(Anchor::Center);

        let (expected_x, expected_y) = Anchor::Center.position_for(680, 400);
        let mut rect = std::mem::MaybeUninit::<RECT>::uninit();
        let rect = unsafe {
            GetWindowRect(hwnd, rect.as_mut_ptr()).unwrap();
            rect.assume_init()
        };
        assert_eq!(rect.left, expected_x);
        assert_eq!(rect.top, expected_y);
        assert_eq!(
            rect.right - rect.left,
            680,
            "reposition must not change width"
        );
        assert_eq!(
            rect.bottom - rect.top,
            400,
            "reposition must not change height"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(&class_name, Some(GetModuleHandleW(None).unwrap().into()));
        }
    }
}
