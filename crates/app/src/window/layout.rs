use winsp_windows::window::Anchor;

use crate::config::WindowPosition;

use super::{ITEM_ROW_HEIGHT, PADDING, SEARCH_BAR_HEIGHT};

pub(super) fn to_anchor(position: WindowPosition) -> Anchor {
    match position {
        WindowPosition::Top => Anchor::Top,
        WindowPosition::Center => Anchor::Center,
    }
}

pub(super) fn result_list_height(results_count: usize) -> i32 {
    if results_count == 0 {
        SEARCH_BAR_HEIGHT
    } else {
        SEARCH_BAR_HEIGHT + (results_count as i32 * ITEM_ROW_HEIGHT) + PADDING
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_anchor_maps_each_position_to_its_matching_anchor() {
        assert_eq!(to_anchor(WindowPosition::Top), Anchor::Top);
        assert_eq!(to_anchor(WindowPosition::Center), Anchor::Center);
    }

    #[test]
    fn result_list_height_with_no_results_is_just_the_search_bar() {
        assert_eq!(result_list_height(0), SEARCH_BAR_HEIGHT);
    }

    #[test]
    fn result_list_height_grows_with_the_result_count() {
        let one = result_list_height(1);
        let two = result_list_height(2);
        assert!(one > SEARCH_BAR_HEIGHT);
        assert_eq!(two - one, ITEM_ROW_HEIGHT);
    }
}
