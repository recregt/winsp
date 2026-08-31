use std::sync::OnceLock;

use winsp_windows::system::{Canvas, Color, Font, FontWeight, Rect};

use super::{APP_STATE, ITEM_ROW_HEIGHT, SEARCH_BAR_HEIGHT, WINDOW_WIDTH};

const HIGHLIGHT_COLOR: Color = Color(0x00FFB74D);

struct Fonts {
    search: Font,
    item_title: Font,
    item_sub: Font,
}

fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| Fonts {
        search: Font::new("Segoe UI Variable Display", 26, FontWeight::Normal),
        item_title: Font::new("Segoe UI Variable Text", 18, FontWeight::SemiBold),
        item_sub: Font::new("Segoe UI Variable Text", 14, FontWeight::Normal),
    })
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
    canvas.fill_rect(client_rect, Color(0x001E1E1E));

    let Some(state_arc) = APP_STATE.get() else {
        return;
    };
    let Ok(state) = state_arc.lock() else {
        return;
    };

    {
        let _font = canvas.select_font(&fonts().search);
        let display_text = if state.query.is_empty() {
            canvas.set_text_color(Color(0x00888888));
            "Search apps, settings, math...".to_string()
        } else {
            canvas.set_text_color(Color(0x00FFFFFF));
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
            Color(0x00333333),
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
            canvas.fill_rect(row_rect, Color(0x003A3A3A));
        }

        let base_color = if is_selected {
            Color(0x00FFFFFF)
        } else {
            Color(0x00E0E0E0)
        };
        {
            let _font = canvas.select_font(&fonts().item_title);
            draw_highlighted_title(
                canvas,
                &result.title,
                &result.matched_indices,
                Rect {
                    left: 32,
                    top: current_y + 4,
                    right: WINDOW_WIDTH - 32,
                    bottom: current_y + 26,
                },
                base_color,
            );
        }

        if let Some(sub) = &result.subtitle {
            let _font = canvas.select_font(&fonts().item_sub);
            canvas.set_text_color(Color(0x00999999));
            let sub_rect = Rect {
                left: 32,
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
    use windows_sys::Win32::Foundation::COLORREF;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC,
        DeleteObject as GdiDeleteObject, FillRect, GetDC, GetPixel, HBITMAP, HDC, HGDIOBJ,
        ReleaseDC, SelectObject, SetBkMode, TRANSPARENT,
    };

    const BITMAP_WIDTH: i32 = 300;
    const BITMAP_HEIGHT: i32 = 40;

    struct OffscreenSurface {
        hdc: HDC,
        bitmap: HBITMAP,
        old_bitmap: HGDIOBJ,
    }

    impl OffscreenSurface {
        fn new() -> Self {
            unsafe {
                let screen_dc = GetDC(std::ptr::null_mut());
                let hdc = CreateCompatibleDC(screen_dc);
                let bitmap = CreateCompatibleBitmap(screen_dc, BITMAP_WIDTH, BITMAP_HEIGHT);
                ReleaseDC(std::ptr::null_mut(), screen_dc);
                let old_bitmap = SelectObject(hdc, bitmap);

                let fill_rect = RECT {
                    left: 0,
                    top: 0,
                    right: BITMAP_WIDTH,
                    bottom: BITMAP_HEIGHT,
                };
                let bg_brush = CreateSolidBrush(0x00000000);
                FillRect(hdc, &fill_rect, bg_brush);
                GdiDeleteObject(bg_brush);
                SetBkMode(hdc, TRANSPARENT as i32);

                Self {
                    hdc,
                    bitmap,
                    old_bitmap,
                }
            }
        }

        fn canvas(&self) -> Canvas {
            unsafe { Canvas::from_raw(self.hdc) }
        }

        fn contains_pixel(&self, color: COLORREF) -> bool {
            unsafe {
                for y in 0..BITMAP_HEIGHT {
                    for x in 0..BITMAP_WIDTH {
                        if GetPixel(self.hdc, x, y) == color {
                            return true;
                        }
                    }
                }
            }
            false
        }
    }

    impl Drop for OffscreenSurface {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.hdc, self.old_bitmap);
                GdiDeleteObject(self.bitmap);
                DeleteDC(self.hdc);
            }
        }
    }

    const BASE_COLOR: Color = Color(0x00E0E0E0);

    const TEST_BOUNDS: Rect = Rect {
        left: 4,
        top: 4,
        right: BITMAP_WIDTH - 4,
        bottom: BITMAP_HEIGHT - 4,
    };

    #[test]
    fn test_highlighted_title_paints_the_highlight_color() {
        let surface = OffscreenSurface::new();
        draw_highlighted_title(
            &surface.canvas(),
            "Notepad",
            &[0, 1, 2],
            TEST_BOUNDS,
            BASE_COLOR,
        );
        assert!(
            surface.contains_pixel(HIGHLIGHT_COLOR.0),
            "expected at least one pixel painted in the highlight color"
        );
    }

    #[test]
    fn test_unhighlighted_title_never_paints_the_highlight_color() {
        let surface = OffscreenSurface::new();
        draw_highlighted_title(&surface.canvas(), "Notepad", &[], TEST_BOUNDS, BASE_COLOR);
        assert!(
            !surface.contains_pixel(HIGHLIGHT_COLOR.0),
            "no pixel should be the highlight color when nothing matched"
        );
    }

    #[test]
    fn test_highlight_color_constant_is_distinct_from_base_and_background() {
        const BACKGROUND_COLOR: Color = Color(0x00000000);
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
