use std::borrow::Cow;
use std::sync::OnceLock;

use winsp_service::{ResultRow, RowIcon};
use winsp_windows::window::gfx::{Canvas, Color, Font, FontWeight, Rect};
use winsp_windows::window::icon_for_path;

use super::UiState;
use super::{ITEM_ROW_HEIGHT, PADDING, SEARCH_BAR_HEIGHT, WINDOW_WIDTH};

const HIGHLIGHT_COLOR: Color = Color::hex(0xFFB74D);
const ICON_SIZE: i32 = 32;
const ICON_LEFT: i32 = 16;
const TEXT_LEFT: i32 = ICON_LEFT + ICON_SIZE + 12;

pub(super) fn result_list_height(results_count: usize) -> i32 {
    if results_count == 0 {
        SEARCH_BAR_HEIGHT
    } else {
        SEARCH_BAR_HEIGHT + (results_count as i32 * ITEM_ROW_HEIGHT) + PADDING
    }
}

/// Index of the result row rendered under the given client-area y, mirroring
/// the layout `render` lays the rows out with.
pub(super) fn result_index_at(y: i32, results_count: usize) -> Option<usize> {
    if results_count == 0 {
        return None;
    }
    let list_top = SEARCH_BAR_HEIGHT + 8;
    if y < list_top {
        return None;
    }
    let index = usize::try_from((y - list_top) / ITEM_ROW_HEIGHT).ok()?;
    (index < results_count).then_some(index)
}

/// Shortens a filesystem-path-shaped subtitle to `drive\folder\...\filename`
/// before drawing, so the collapsed portion always lands in the same
/// predictable place rather than wherever `DrawTextW`'s own `DT_PATH_ELLIPSIS`
/// happens to cut. That GDI-level ellipsis still runs on the result (via
/// `draw_text_ellipsized`) as a fallback for when even this shortened form is
/// still too wide for the row, e.g. an unusually long file name.
fn shorten_path_for_display(text: &str) -> Cow<'_, str> {
    if !text.contains('\\') {
        return Cow::Borrowed(text);
    }

    let is_unc = text.starts_with(r"\\");
    let segments: Vec<&str> = text.split('\\').filter(|s| !s.is_empty()).collect();
    let Some((filename, rest)) = segments.split_last() else {
        return Cow::Borrowed(text);
    };
    if segments.len() <= 3 {
        return Cow::Borrowed(text);
    }

    let head_len = 2.min(rest.len());
    let head = rest[..head_len].join("\\");
    let prefix = if is_unc { r"\\" } else { "" };
    Cow::Owned(format!("{prefix}{head}\\...\\{filename}"))
}

static INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
static INTER_DISPLAY_REGULAR: &[u8] = include_bytes!("../assets/fonts/InterDisplay-Regular.ttf");

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

fn draw_result_icon(canvas: &Canvas, result: &ResultRow, rect: Rect) {
    match &result.icon {
        Some(RowIcon::Path(path)) => {
            if let Some(icon) = icon_for_path(path) {
                canvas.draw_cached_icon(&icon, rect);
            }
        }
        Some(RowIcon::Glyph(glyph)) => {
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
            canvas.draw_text_ellipsized(&shorten_path_for_display(sub), sub_rect);
        }

        current_y += ITEM_ROW_HEIGHT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_list_height_with_no_results_is_just_the_search_bar() {
        assert_eq!(result_list_height(0), SEARCH_BAR_HEIGHT);
    }

    #[test]
    fn shortens_a_deeply_nested_path_to_drive_folder_ellipsis_filename() {
        assert_eq!(
            shorten_path_for_display(r"C:\Program Files\Vendor\Product\1.2.3\bin\app.exe"),
            r"C:\Program Files\...\app.exe"
        );
    }

    #[test]
    fn leaves_a_short_path_unchanged() {
        assert_eq!(
            shorten_path_for_display(r"C:\Windows\app.exe"),
            r"C:\Windows\app.exe"
        );
    }

    #[test]
    fn leaves_plain_text_without_backslashes_unchanged() {
        assert_eq!(
            shorten_path_for_display("Microsoft Calculator"),
            "Microsoft Calculator"
        );
    }

    #[test]
    fn preserves_the_unc_prefix_on_a_long_network_path() {
        assert_eq!(
            shorten_path_for_display(r"\\server\share\folder\file.exe"),
            r"\\server\share\...\file.exe"
        );
    }

    #[test]
    fn does_not_panic_on_a_path_of_only_separators() {
        assert_eq!(shorten_path_for_display(r"\\\"), r"\\\");
    }

    #[test]
    fn does_not_panic_on_an_empty_string() {
        assert_eq!(shorten_path_for_display(""), "");
    }

    #[test]
    fn result_index_at_is_none_with_no_results() {
        assert_eq!(result_index_at(SEARCH_BAR_HEIGHT + 20, 0), None);
    }

    #[test]
    fn result_index_at_is_none_above_the_result_list() {
        assert_eq!(result_index_at(SEARCH_BAR_HEIGHT, 3), None);
    }

    #[test]
    fn result_index_at_finds_the_first_row() {
        assert_eq!(result_index_at(SEARCH_BAR_HEIGHT + 8, 3), Some(0));
    }

    #[test]
    fn result_index_at_finds_a_later_row() {
        let y = SEARCH_BAR_HEIGHT + 8 + ITEM_ROW_HEIGHT + 5;
        assert_eq!(result_index_at(y, 3), Some(1));
    }

    #[test]
    fn result_index_at_is_none_past_the_last_row() {
        let y = SEARCH_BAR_HEIGHT + 8 + (3 * ITEM_ROW_HEIGHT);
        assert_eq!(result_index_at(y, 3), None);
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
    use winsp_windows::window::gfx::testing::OffscreenSurface;

    const BITMAP_WIDTH: i32 = 300;
    const BITMAP_HEIGHT: i32 = 40;

    const ICON_BOUNDS: Rect = Rect {
        left: 4,
        top: 4,
        right: 36,
        bottom: 36,
    };

    fn row(icon: Option<RowIcon>) -> ResultRow {
        ResultRow {
            title: "Name".into(),
            subtitle: None,
            matched_char_indices: Vec::new(),
            icon,
        }
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
        let state = UiState::new(&winsp_service::testing::empty_service());
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

        draw_result_icon(
            &surface.canvas(),
            &row(Some(RowIcon::Glyph('A'))),
            ICON_BOUNDS,
        );

        assert!(surface.contains_pixel(Color::hex(0xCCCCCC)));
    }

    #[test]
    fn draw_result_icon_paints_a_real_shell_icon_for_a_resolvable_path() {
        let surface = OffscreenSurface::new(40, 40);
        let exe = std::env::current_exe().unwrap();
        let exe_path = exe.to_string_lossy().into_owned();

        wait_for_icon(&exe_path);
        draw_result_icon(
            &surface.canvas(),
            &row(Some(RowIcon::Path(exe_path))),
            ICON_BOUNDS,
        );

        assert!(surface.contains_pixel_other_than(Color::hex(0x000000)));
    }

    #[test]
    fn draw_result_icon_paints_nothing_for_a_missing_path_icon() {
        let surface = OffscreenSurface::new(40, 40);

        draw_result_icon(
            &surface.canvas(),
            &row(Some(RowIcon::Path(
                r"C:\definitely\not\a\real\path.exe".into(),
            ))),
            ICON_BOUNDS,
        );

        assert!(!surface.contains_pixel_other_than(Color::hex(0x000000)));
    }

    #[test]
    fn draw_result_icon_paints_nothing_for_a_non_app_result() {
        let surface = OffscreenSurface::new(40, 40);

        draw_result_icon(&surface.canvas(), &row(None), ICON_BOUNDS);

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
