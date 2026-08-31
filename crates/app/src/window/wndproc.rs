use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use winsp_windows::system::{TrayCommand, WM_TRAYICON};

use super::APP_STATE;
use super::geometry::{resize_window_for_results, toggle_visibility};
use super::input::{PENDING_HIGH_SURROGATE, decode_utf16_unit};
use super::render::render_ui;

pub(super) unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            toggle_visibility(hwnd);
            0
        }
        WM_TRAYICON => {
            if lparam as u32 == WM_RBUTTONUP {
                winsp_windows::system::WindowHandle::new(hwnd).show_tray_menu();
            }
            0
        }
        WM_COMMAND => {
            if let Ok(cmd) = TrayCommand::try_from(wparam & 0xffff) {
                match cmd {
                    TrayCommand::Toggle => toggle_visibility(hwnd),
                    TrayCommand::Autostart => {
                        use winsp_windows::system::autostart;
                        autostart::set_enabled(!autostart::is_enabled());
                    }
                    TrayCommand::Exit => unsafe {
                        DestroyWindow(hwnd);
                    },
                }
            }
            0
        }
        WM_KILLFOCUS => {
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
            0
        }
        WM_CHAR => {
            let mut pending = PENDING_HIGH_SURROGATE.lock().unwrap();
            if let Some(c) = decode_utf16_unit(&mut pending, wparam as u16) {
                if !c.is_control() {
                    if let Some(state_arc) = APP_STATE.get() {
                        if let Ok(mut state) = state_arc.lock() {
                            state.insert_char(c);
                            resize_window_for_results(hwnd, state.results.len());
                        }
                    }
                    unsafe {
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    }
                }
            }
            0
        }
        WM_KEYDOWN => {
            let vk = wparam as u16;
            if let Some(state_arc) = APP_STATE.get() {
                let mut should_resize = false;
                let mut results_count = 0;
                let mut should_hide = false;

                if let Ok(mut state) = state_arc.lock() {
                    match vk {
                        VK_BACK => {
                            state.backspace();
                            should_resize = true;
                            results_count = state.results.len();
                        }
                        VK_DOWN | VK_TAB => {
                            state.select_next();
                        }
                        VK_UP => {
                            state.select_prev();
                        }
                        VK_RETURN => {
                            let _ = state.execute_selected();
                            should_hide = true;
                        }
                        VK_ESCAPE => {
                            should_hide = true;
                        }
                        _ => {}
                    }
                }

                unsafe {
                    if should_hide {
                        ShowWindow(hwnd, SW_HIDE);
                    } else {
                        if should_resize {
                            resize_window_for_results(hwnd, results_count);
                        }
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    }
                }
            }
            0
        }
        WM_PAINT => {
            unsafe {
                let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
                let hdc = BeginPaint(hwnd, &mut ps);
                render_ui(hwnd, hdc);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_DESTROY => {
            winsp_windows::system::WindowHandle::new(hwnd).remove_tray_icon();
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
