use winsp_windows::window::{Anchor, Window};

use super::interaction;
use super::settings::WindowPosition;
use super::{
    APP_STATE, ITEM_ROW_HEIGHT, PADDING, RECONCILE_TX, SEARCH_BAR_HEIGHT, SETTINGS, WINDOW_WIDTH,
};

pub(super) fn to_anchor(position: WindowPosition) -> Anchor {
    match position {
        WindowPosition::Top => Anchor::Top,
        WindowPosition::Center => Anchor::Center,
    }
}

pub(super) fn current_anchor() -> Anchor {
    SETTINGS
        .get()
        .and_then(|settings| settings.lock().ok().map(|settings| settings.position))
        .map(to_anchor)
        .unwrap_or(Anchor::Top)
}

pub(super) fn show_fresh(handle: &Window) {
    handle.center(WINDOW_WIDTH, SEARCH_BAR_HEIGHT, current_anchor());
    if let Some(state_arc) = APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            interaction::clear_query(&mut state);
        }
    }
    if let Some(tx) = RECONCILE_TX.get() {
        let _ = tx.send(());
    }
    handle.show();
    handle.invalidate();
}

pub(super) fn toggle_visibility(handle: &Window) {
    if handle.is_visible() {
        handle.hide();
    } else {
        show_fresh(handle);
    }
}

pub(super) fn resize_window_for_results(handle: &Window, results_count: usize) {
    let height = if results_count == 0 {
        SEARCH_BAR_HEIGHT
    } else {
        SEARCH_BAR_HEIGHT + (results_count as i32 * ITEM_ROW_HEIGHT) + PADDING
    };
    handle.resize(WINDOW_WIDTH, height);
}

pub(super) fn begin_hotkey_capture(handle: &Window) {
    handle.center(WINDOW_WIDTH, SEARCH_BAR_HEIGHT, current_anchor());
    resize_window_for_results(handle, 0);
    handle.show();
    handle.invalidate();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_anchor_maps_each_position_to_its_matching_anchor() {
        assert_eq!(to_anchor(WindowPosition::Top), Anchor::Top);
        assert_eq!(to_anchor(WindowPosition::Center), Anchor::Center);
    }
}
