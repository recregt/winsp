use std::sync::OnceLock;

use winsp_core::models::{IconSource, SearchResult, SearchResultKind};
use winsp_windows::window::Anchor;
use winsp_windows::window::gfx::{Canvas, Color, Font, FontWeight, Rect};
use winsp_windows::window::icon_for_path;

use crate::config::WindowPosition;

use super::UiState;
use super::{ITEM_ROW_HEIGHT, PADDING, SEARCH_BAR_HEIGHT, WINDOW_WIDTH};

const HIGHLIGHT_COLOR: Color = Color::hex(0xFFB74D);
const ICON_SIZE: i32 = 32;
const ICON_LEFT: i32 = 16;
const TEXT_LEFT: i32 = ICON_LEFT + ICON_SIZE + 12;

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

static INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
static INTER_DISPLAY_REGULAR: &[u8] = include_bytes!("../../assets/fonts/InterDisplay-Regular.ttf");

struct Fonts {
    search: Font,
    item_title: Font,
    item_sub: Font,
    icon_glyph: Font,
}

fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        Font::register(INTER_REGULAR);
        Font::register(INTER_SEMIBOLD);
        Font::register(INTER_DISPLAY_REGULAR);

        Fonts {
            search: Font::new("Inter Display", 26, FontWeight::Normal),
            item_title: Font::new("Inter SemiBold", 18, FontWeight::Normal),
            item_sub: Font::new("Inter", 14, FontWeight::Normal),
            icon_glyph: Font::new("Segoe MDL2 Assets", 20, FontWeight::Normal),
        }
    })
}

fn draw_result_icon(canvas: &Canvas, result: &SearchResult, rect: Rect) {
    let SearchResultKind::App(item) = &result.kind else {
        return;
    };
    match item.icon() {
        Some(IconSource::Path(path)) => {
            if let Some(icon) = icon_for_path(path) {
                canvas.draw_cached_icon(&icon, rect);
            }
        }
        Some(IconSource::Glyph(glyph)) => {
            let _font = canvas.select_font(&fonts().icon_glyph);
            canvas.set_text_color(Color::hex(0xCCCCCC));
            canvas.draw_icon_glyph(*glyph, rect);
        }
        None => {}
    }
}

fn highlight_segments<'a>(text: &'a str, matched_char_indices: &[usize]) -> Vec<(bool, &'a str)> {
    let mut segments: Vec<(bool, &str)> = Vec::new();
    let mut run_start = 0;
    let mut run_highlighted = false;
    let mut started = false;

    for (char_idx, (byte_idx, _)) in text.char_indices().enumerate() {
        let highlighted = matched_char_indices.contains(&char_idx);
        if !started {
            run_highlighted = highlighted;
            run_start = byte_idx;
            started = true;
        } else if highlighted != run_highlighted {
            segments.push((run_highlighted, &text[run_start..byte_idx]));
            run_start = byte_idx;
            run_highlighted = highlighted;
        }
    }
    if started {
        segments.push((run_highlighted, &text[run_start..]));
    }
    segments
}

fn draw_highlighted_title(
    canvas: &Canvas,
    title: &str,
    matched_char_indices: &[usize],
    bounds: Rect,
    base_color: Color,
) {
    let mut seg_left = bounds.left;
    for (highlighted, segment) in highlight_segments(title, matched_char_indices) {
        if seg_left >= bounds.right {
            break;
        }
        canvas.set_text_color(if highlighted {
            HIGHLIGHT_COLOR
        } else {
            base_color
        });
        let seg_rect = Rect {
            left: seg_left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        };
        seg_left += canvas.draw_text_measured(segment, seg_rect);
    }
}

pub(super) const BACKGROUND_COLOR: Color = Color::hex(0x1E1E1E);

pub(super) fn render(canvas: &Canvas, state: &UiState, client_rect: Rect) {
    canvas.fill_rect(client_rect, BACKGROUND_COLOR);

    if state.is_capturing_hotkey() {
        let _font = canvas.select_font(&fonts().search);
        canvas.set_text_color(Color::hex(0xFFFFFF));
        let prompt_rect = Rect {
            left: 24,
            top: 14,
            right: WINDOW_WIDTH - 24,
            bottom: SEARCH_BAR_HEIGHT - 10,
        };
        canvas.draw_text("Press a key combination... Esc to cancel", prompt_rect);
        return;
    }

    {
        let _font = canvas.select_font(&fonts().search);
        let display_text: &str = if state.query().is_empty() {
            canvas.set_text_color(Color::hex(0x888888));
            "Search apps, settings, math..."
        } else {
            canvas.set_text_color(Color::hex(0xFFFFFF));
            state.query()
        };
        let search_rect = Rect {
            left: 24,
            top: 14,
            right: WINDOW_WIDTH - 24,
            bottom: SEARCH_BAR_HEIGHT - 10,
        };
        canvas.draw_text(display_text, search_rect);
    }

    let mut current_y = SEARCH_BAR_HEIGHT;

    if !state.results().is_empty() {
        canvas.draw_line(
            (16, current_y),
            (WINDOW_WIDTH - 16, current_y),
            Color::hex(0x333333),
        );
        current_y += 8;
    }

    for (idx, result) in state.results().iter().enumerate() {
        let is_selected = idx == state.selected_index();

        let row_rect = Rect {
            left: 12,
            top: current_y,
            right: WINDOW_WIDTH - 12,
            bottom: current_y + ITEM_ROW_HEIGHT - 6,
        };

        if is_selected {
            canvas.fill_rect(row_rect, Color::hex(0x3A3A3A));
        }

        draw_result_icon(
            canvas,
            result,
            Rect {
                left: ICON_LEFT,
                top: current_y + 8,
                right: ICON_LEFT + ICON_SIZE,
                bottom: current_y + 8 + ICON_SIZE,
            },
        );

        let base_color = if is_selected {
            Color::hex(0xFFFFFF)
        } else {
            Color::hex(0xE0E0E0)
        };
        {
            let _font = canvas.select_font(&fonts().item_title);
            draw_highlighted_title(
                canvas,
                &result.title,
                &result.matched_char_indices,
                Rect {
                    left: TEXT_LEFT,
                    top: current_y + 4,
                    right: WINDOW_WIDTH - 32,
                    bottom: current_y + 26,
                },
                base_color,
            );
        }

        if let Some(sub) = &result.subtitle {
            let _font = canvas.select_font(&fonts().item_sub);
            canvas.set_text_color(Color::hex(0x999999));
            let sub_rect = Rect {
                left: TEXT_LEFT,
                top: current_y + 26,
                right: WINDOW_WIDTH - 32,
                bottom: current_y + ITEM_ROW_HEIGHT - 8,
            };
            canvas.draw_text(sub, sub_rect);
        }

        current_y += ITEM_ROW_HEIGHT;
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

#[cfg(test)]
mod render_tests {
    use super::*;
    use winsp_core::models::{AppItem, LaunchTarget};
    use winsp_windows::window::gfx::testing::OffscreenSurface;

    const BITMAP_WIDTH: i32 = 300;
    const BITMAP_HEIGHT: i32 = 40;

    const ICON_BOUNDS: Rect = Rect {
        left: 4,
        top: 4,
        right: 36,
        bottom: 36,
    };

    fn app_result(item: AppItem) -> SearchResult {
        SearchResult::from_app(std::sync::Arc::new(item), 0, Vec::new())
    }

    fn wait_for_icon(path: &str) {
        for _ in 0..200 {
            if icon_for_path(path).is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("icon for {path} did not resolve in time");
    }

    #[test]
    fn render_draws_from_the_state_it_is_given_without_touching_any_static() {
        let surface = OffscreenSurface::new(BITMAP_WIDTH, 200);
        let state = UiState::new(&winsp_core::engine::Engine::new());
        let client_rect = Rect {
            left: 0,
            top: 0,
            right: BITMAP_WIDTH,
            bottom: 200,
        };

        render(&surface.canvas(), &state, client_rect);

        assert!(surface.contains_pixel(BACKGROUND_COLOR));
    }

    #[test]
    fn draw_result_icon_paints_the_glyph_color_for_a_glyph_icon() {
        let surface = OffscreenSurface::new(40, 40);
        let item = AppItem::new("id", "Name", LaunchTarget::OsUri("ms-settings:".into()))
            .with_icon_glyph('A');

        draw_result_icon(&surface.canvas(), &app_result(item), ICON_BOUNDS);

        assert!(surface.contains_pixel(Color::hex(0xCCCCCC)));
    }

    #[test]
    fn draw_result_icon_paints_a_real_shell_icon_for_a_resolvable_path() {
        let surface = OffscreenSurface::new(40, 40);
        let exe = std::env::current_exe().unwrap();
        let exe_path = exe.to_string_lossy().into_owned();
        let item = AppItem::new("id", "Name", LaunchTarget::Path(exe_path.clone()))
            .with_icon(exe_path.clone());

        wait_for_icon(&exe_path);
        draw_result_icon(&surface.canvas(), &app_result(item), ICON_BOUNDS);

        assert!(surface.contains_pixel_other_than(Color::hex(0x000000)));
    }

    #[test]
    fn draw_result_icon_paints_nothing_for_a_missing_path_icon() {
        let surface = OffscreenSurface::new(40, 40);
        let item = AppItem::new("id", "Name", LaunchTarget::Path("missing.exe".into()))
            .with_icon(r"C:\definitely\not\a\real\path.exe");

        draw_result_icon(&surface.canvas(), &app_result(item), ICON_BOUNDS);

        assert!(!surface.contains_pixel_other_than(Color::hex(0x000000)));
    }

    #[test]
    fn draw_result_icon_paints_nothing_for_a_non_app_result() {
        let surface = OffscreenSurface::new(40, 40);
        let result = SearchResult::calculation("1+1".into(), "2".into());

        draw_result_icon(&surface.canvas(), &result, ICON_BOUNDS);

        assert!(!surface.contains_pixel_other_than(Color::hex(0x000000)));
    }

    const BASE_COLOR: Color = Color::hex(0xE0E0E0);

    const TEST_BOUNDS: Rect = Rect {
        left: 4,
        top: 4,
        right: BITMAP_WIDTH - 4,
        bottom: BITMAP_HEIGHT - 4,
    };

    #[test]
    fn test_highlighted_title_paints_the_highlight_color() {
        let surface = OffscreenSurface::new(BITMAP_WIDTH, BITMAP_HEIGHT);
        draw_highlighted_title(
            &surface.canvas(),
            "Notepad",
            &[0, 1, 2],
            TEST_BOUNDS,
            BASE_COLOR,
        );
        assert!(
            surface.contains_pixel(HIGHLIGHT_COLOR),
            "expected at least one pixel painted in the highlight color"
        );
    }

    #[test]
    fn test_unhighlighted_title_never_paints_the_highlight_color() {
        let surface = OffscreenSurface::new(BITMAP_WIDTH, BITMAP_HEIGHT);
        draw_highlighted_title(&surface.canvas(), "Notepad", &[], TEST_BOUNDS, BASE_COLOR);
        assert!(
            !surface.contains_pixel(HIGHLIGHT_COLOR),
            "no pixel should be the highlight color when nothing matched"
        );
    }

    #[test]
    fn test_highlight_color_constant_is_distinct_from_base_and_background() {
        const BACKGROUND_COLOR: Color = Color::hex(0x000000);
        assert_ne!(HIGHLIGHT_COLOR, BASE_COLOR);
        assert_ne!(HIGHLIGHT_COLOR, BACKGROUND_COLOR);
    }

    #[test]
    fn test_no_matches_yields_single_unhighlighted_segment() {
        assert_eq!(highlight_segments("Notepad", &[]), vec![(false, "Notepad")]);
    }

    #[test]
    fn test_contiguous_prefix_match_yields_two_segments() {
        assert_eq!(
            highlight_segments("Notepad", &[0, 1, 2]),
            vec![(true, "Not"), (false, "epad")]
        );
    }

    #[test]
    fn test_scattered_acronym_match_yields_alternating_segments() {
        assert_eq!(
            highlight_segments("Visual Studio", &[0, 7]),
            vec![
                (true, "V"),
                (false, "isual "),
                (true, "S"),
                (false, "tudio"),
            ]
        );
    }

    #[test]
    fn test_full_match_yields_single_highlighted_segment() {
        assert_eq!(highlight_segments("cmd", &[0, 1, 2]), vec![(true, "cmd")]);
    }

    #[test]
    fn test_out_of_range_indices_are_ignored_without_panicking() {
        assert_eq!(
            highlight_segments("cmd", &[0, 99]),
            vec![(true, "c"), (false, "md")]
        );
    }

    #[test]
    fn test_multibyte_characters_split_on_char_boundaries_not_bytes() {
        assert_eq!(
            highlight_segments("日本語アプリ", &[3, 4, 5]),
            vec![(false, "日本語"), (true, "アプリ")]
        );
    }

    #[test]
    fn test_match_at_end_of_title() {
        assert_eq!(
            highlight_segments("Notepad", &[4, 5, 6]),
            vec![(false, "Note"), (true, "pad")]
        );
    }

    #[test]
    fn test_match_in_middle_of_long_title() {
        assert_eq!(
            highlight_segments("Adobe Photoshop Express", &[6, 7, 8, 9, 10]),
            vec![(false, "Adobe "), (true, "Photo"), (false, "shop Express"),]
        );
    }

    #[test]
    fn test_empty_title_yields_no_segments() {
        assert_eq!(highlight_segments("", &[]), Vec::<(bool, &str)>::new());
    }

    #[test]
    fn test_all_indices_out_of_range_yields_unhighlighted_segment() {
        assert_eq!(
            highlight_segments("cmd", &[10, 20, 30]),
            vec![(false, "cmd")]
        );
    }

    #[test]
    fn test_supplementary_plane_character_stays_one_segment_and_encodes_as_surrogate_pair() {
        assert_eq!(
            highlight_segments("😀 Settings", &[0]),
            vec![(true, "😀"), (false, " Settings")]
        );
    }
}
