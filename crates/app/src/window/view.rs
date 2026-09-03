use std::sync::OnceLock;

use winsp_core::models::{IconSource, SearchResult, SearchResultKind};
use winsp_windows::window::gfx::{Canvas, Color, Font, FontWeight, Rect};
use winsp_windows::window::icon_for_path;

use super::{APP_STATE, ITEM_ROW_HEIGHT, SEARCH_BAR_HEIGHT, WINDOW_WIDTH};

const HIGHLIGHT_COLOR: Color = Color::hex(0xFFB74D);
const ICON_SIZE: i32 = 32;
const ICON_LEFT: i32 = 16;
const TEXT_LEFT: i32 = ICON_LEFT + ICON_SIZE + 12;

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
    match &item.icon {
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

fn highlight_segments(text: &str, matched_indices: &[usize]) -> Vec<(bool, String)> {
    let char_count = text.chars().count();
    let mut is_match = vec![false; char_count];
    for &i in matched_indices {
        if let Some(flag) = is_match.get_mut(i) {
            *flag = true;
        }
    }

    let mut segments: Vec<(bool, String)> = Vec::new();
    for (ch, &highlighted) in text.chars().zip(is_match.iter()) {
        match segments.last_mut() {
            Some((last_highlighted, run)) if *last_highlighted == highlighted => run.push(ch),
            _ => segments.push((highlighted, ch.to_string())),
        }
    }
    segments
}

fn draw_highlighted_title(
    canvas: &Canvas,
    title: &str,
    matched_indices: &[usize],
    bounds: Rect,
    base_color: Color,
) {
    let mut seg_left = bounds.left;
    for (highlighted, segment) in highlight_segments(title, matched_indices) {
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
        seg_left += canvas.draw_text_measured(&segment, seg_rect);
    }
}

pub(super) fn render_ui(canvas: &Canvas, client_rect: Rect) {
    canvas.fill_rect(client_rect, Color::hex(0x1E1E1E));

    let Some(state_arc) = APP_STATE.get() else {
        return;
    };
    let Ok(state) = state_arc.lock() else {
        return;
    };

    if state.capturing_hotkey {
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
        let display_text = if state.query.is_empty() {
            canvas.set_text_color(Color::hex(0x888888));
            "Search apps, settings, math...".to_string()
        } else {
            canvas.set_text_color(Color::hex(0xFFFFFF));
            state.query.clone()
        };
        let search_rect = Rect {
            left: 24,
            top: 14,
            right: WINDOW_WIDTH - 24,
            bottom: SEARCH_BAR_HEIGHT - 10,
        };
        canvas.draw_text(&display_text, search_rect);
    }

    let mut current_y = SEARCH_BAR_HEIGHT;

    if !state.results.is_empty() {
        canvas.draw_line(
            (16, current_y),
            (WINDOW_WIDTH - 16, current_y),
            Color::hex(0x333333),
        );
        current_y += 8;
    }

    for (idx, result) in state.results.iter().enumerate() {
        let is_selected = idx == state.selected_index;

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
                &result.matched_indices,
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
    use winsp_core::models::{AppItem, AppTarget};
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
    fn draw_result_icon_paints_the_glyph_color_for_a_glyph_icon() {
        let surface = OffscreenSurface::new(40, 40);
        let item =
            AppItem::new("id", "Name", AppTarget::Uri("ms-settings:".into())).with_icon_glyph('A');

        draw_result_icon(&surface.canvas(), &app_result(item), ICON_BOUNDS);

        assert!(surface.contains_pixel(Color::hex(0xCCCCCC)));
    }

    #[test]
    fn draw_result_icon_paints_a_real_shell_icon_for_a_resolvable_path() {
        let surface = OffscreenSurface::new(40, 40);
        let exe = std::env::current_exe().unwrap();
        let exe_path = exe.to_string_lossy().into_owned();
        let item = AppItem::new("id", "Name", AppTarget::Path(exe_path.clone()))
            .with_icon(exe_path.clone());

        wait_for_icon(&exe_path);
        draw_result_icon(&surface.canvas(), &app_result(item), ICON_BOUNDS);

        assert!(surface.contains_pixel_other_than(Color::hex(0x000000)));
    }

    #[test]
    fn draw_result_icon_paints_nothing_for_a_missing_path_icon() {
        let surface = OffscreenSurface::new(40, 40);
        let item = AppItem::new("id", "Name", AppTarget::Path("missing.exe".into()))
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
        assert_eq!(
            highlight_segments("Notepad", &[]),
            vec![(false, "Notepad".to_string())]
        );
    }

    #[test]
    fn test_contiguous_prefix_match_yields_two_segments() {
        assert_eq!(
            highlight_segments("Notepad", &[0, 1, 2]),
            vec![(true, "Not".to_string()), (false, "epad".to_string())]
        );
    }

    #[test]
    fn test_scattered_acronym_match_yields_alternating_segments() {
        assert_eq!(
            highlight_segments("Visual Studio", &[0, 7]),
            vec![
                (true, "V".to_string()),
                (false, "isual ".to_string()),
                (true, "S".to_string()),
                (false, "tudio".to_string()),
            ]
        );
    }

    #[test]
    fn test_full_match_yields_single_highlighted_segment() {
        assert_eq!(
            highlight_segments("cmd", &[0, 1, 2]),
            vec![(true, "cmd".to_string())]
        );
    }

    #[test]
    fn test_out_of_range_indices_are_ignored_without_panicking() {
        assert_eq!(
            highlight_segments("cmd", &[0, 99]),
            vec![(true, "c".to_string()), (false, "md".to_string())]
        );
    }

    #[test]
    fn test_multibyte_characters_split_on_char_boundaries_not_bytes() {
        assert_eq!(
            highlight_segments("日本語アプリ", &[3, 4, 5]),
            vec![(false, "日本語".to_string()), (true, "アプリ".to_string()),]
        );
    }

    #[test]
    fn test_match_at_end_of_title() {
        assert_eq!(
            highlight_segments("Notepad", &[4, 5, 6]),
            vec![(false, "Note".to_string()), (true, "pad".to_string())]
        );
    }

    #[test]
    fn test_match_in_middle_of_long_title() {
        assert_eq!(
            highlight_segments("Adobe Photoshop Express", &[6, 7, 8, 9, 10]),
            vec![
                (false, "Adobe ".to_string()),
                (true, "Photo".to_string()),
                (false, "shop Express".to_string()),
            ]
        );
    }

    #[test]
    fn test_empty_title_yields_no_segments() {
        assert_eq!(highlight_segments("", &[]), Vec::<(bool, String)>::new());
    }

    #[test]
    fn test_all_indices_out_of_range_yields_unhighlighted_segment() {
        assert_eq!(
            highlight_segments("cmd", &[10, 20, 30]),
            vec![(false, "cmd".to_string())]
        );
    }

    #[test]
    fn test_supplementary_plane_character_stays_one_segment_and_encodes_as_surrogate_pair() {
        assert_eq!(
            highlight_segments("😀 Settings", &[0]),
            vec![(true, "😀".to_string()), (false, " Settings".to_string())]
        );
    }
}
