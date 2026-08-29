use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::{APP_STATE, ITEM_ROW_HEIGHT, PADDING, SEARCH_BAR_HEIGHT, WINDOW_WIDTH};

pub(super) fn toggle_visibility(hwnd: HWND) {
    unsafe {
        if IsWindowVisible(hwnd) != 0 {
            ShowWindow(hwnd, SW_HIDE);
        } else {
            center_window(hwnd);
            if let Some(state_arc) = APP_STATE.get() {
                if let Ok(mut state) = state_arc.lock() {
                    state.clear_query();
                }
            }
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            InvalidateRect(hwnd, std::ptr::null(), 1);
        }
    }
}

pub(super) fn center_window(hwnd: HWND) {
    unsafe {
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);

        let x = (screen_width - WINDOW_WIDTH) / 2;
        let y = screen_height / 4; // Upper 1/3 like macOS Spotlight

        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            x,
            y,
            WINDOW_WIDTH,
            SEARCH_BAR_HEIGHT,
            SWP_NOACTIVATE,
        );
    }
}

pub(super) fn resize_window_for_results(hwnd: HWND, results_count: usize) {
    unsafe {
        let height = if results_count == 0 {
            SEARCH_BAR_HEIGHT
        } else {
            SEARCH_BAR_HEIGHT + (results_count as i32 * ITEM_ROW_HEIGHT) + PADDING
        };

        let mut rect = std::mem::zeroed::<RECT>();
        let _ = GetWindowRect(hwnd, &mut rect);
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            rect.left,
            rect.top,
            WINDOW_WIDTH,
            height,
            SWP_NOMOVE | SWP_NOACTIVATE,
        );
    }
}
