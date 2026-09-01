use winsp_windows::window::WindowHandle;

use super::{APP_STATE, ITEM_ROW_HEIGHT, PADDING, SEARCH_BAR_HEIGHT, WINDOW_WIDTH};

pub(super) fn toggle_visibility(handle: &WindowHandle) {
    if handle.is_visible() {
        handle.hide();
    } else {
        handle.center(WINDOW_WIDTH, SEARCH_BAR_HEIGHT);
        if let Some(state_arc) = APP_STATE.get() {
            if let Ok(mut state) = state_arc.lock() {
                state.clear_query();
            }
        }
        handle.show();
        handle.invalidate();
    }
}

pub(super) fn resize_window_for_results(handle: &WindowHandle, results_count: usize) {
    let height = if results_count == 0 {
        SEARCH_BAR_HEIGHT
    } else {
        SEARCH_BAR_HEIGHT + (results_count as i32 * ITEM_ROW_HEIGHT) + PADDING
    };
    handle.resize(WINDOW_WIDTH, height);
}
