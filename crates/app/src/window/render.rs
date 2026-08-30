use windows_sys::Win32::Foundation::{COLORREF, HWND, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
    DT_LEFT, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FW_NORMAL, FW_SEMIBOLD,
    FillRect, GetTextExtentPoint32W, HDC, LineTo, MoveToEx, PS_SOLID, SRCCOPY, SelectObject,
    SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::{APP_STATE, ITEM_ROW_HEIGHT, SEARCH_BAR_HEIGHT, WINDOW_WIDTH, to_wide};

const HIGHLIGHT_COLOR: COLORREF = 0x00FFB74D;

/// `matched_indices` are zero-based Unicode scalar (`char`) indices into `text`, not byte offsets.
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

unsafe fn draw_highlighted_title(
    hdc: HDC,
    title: &str,
    matched_indices: &[usize],
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    base_color: COLORREF,
) {
    unsafe {
        let mut seg_left = left;
        for (highlighted, segment) in highlight_segments(title, matched_indices) {
            if seg_left >= right {
                break;
            }
            SetTextColor(
                hdc,
                if highlighted {
                    HIGHLIGHT_COLOR
                } else {
                    base_color
                },
            );
            let mut seg_wide = to_wide(&segment);
            let mut seg_rect = RECT {
                left: seg_left,
                top,
                right,
                bottom,
            };
            DrawTextW(
                hdc,
                seg_wide.as_mut_ptr(),
                seg_wide.len() as i32 - 1,
                &mut seg_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );

            let mut extent = std::mem::zeroed::<SIZE>();
            GetTextExtentPoint32W(
                hdc,
                seg_wide.as_ptr(),
                seg_wide.len() as i32 - 1,
                &mut extent,
            );
            seg_left += extent.cx;
        }
    }
}

pub(super) unsafe fn render_ui(hwnd: HWND, hdc: HDC) {
    unsafe {
        let mut client_rect = std::mem::zeroed::<RECT>();
        let _ = GetClientRect(hwnd, &mut client_rect);

        let mem_dc = CreateCompatibleDC(hdc);
        let mem_bmp = CreateCompatibleBitmap(hdc, client_rect.right, client_rect.bottom);
        let old_bmp = SelectObject(mem_dc, mem_bmp);

        let bg_brush = CreateSolidBrush(0x001E1E1E as COLORREF);
        FillRect(mem_dc, &client_rect, bg_brush);
        DeleteObject(bg_brush);

        SetBkMode(mem_dc, TRANSPARENT as i32);

        if let Some(state_arc) = APP_STATE.get() {
            if let Ok(state) = state_arc.lock() {
                let font_name = to_wide("Segoe UI Variable Display");
                let font_title = CreateFontW(
                    26,
                    0,
                    0,
                    0,
                    FW_NORMAL as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    font_name.as_ptr(),
                );
                let old_font = SelectObject(mem_dc, font_title);

                let display_text = if state.query.is_empty() {
                    SetTextColor(mem_dc, 0x00888888);
                    "Search apps, settings, math...".to_string()
                } else {
                    SetTextColor(mem_dc, 0x00FFFFFF);
                    state.query.clone()
                };

                let mut text_wide = to_wide(&display_text);
                let mut search_rect = RECT {
                    left: 24,
                    top: 14,
                    right: WINDOW_WIDTH - 24,
                    bottom: SEARCH_BAR_HEIGHT - 10,
                };
                DrawTextW(
                    mem_dc,
                    text_wide.as_mut_ptr(),
                    text_wide.len() as i32 - 1,
                    &mut search_rect,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                );

                let font_item_name = to_wide("Segoe UI Variable Text");
                let font_item_title = CreateFontW(
                    18,
                    0,
                    0,
                    0,
                    FW_SEMIBOLD as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    font_item_name.as_ptr(),
                );
                let font_item_sub = CreateFontW(
                    14,
                    0,
                    0,
                    0,
                    FW_NORMAL as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    font_item_name.as_ptr(),
                );

                let mut current_y = SEARCH_BAR_HEIGHT;

                if !state.results.is_empty() {
                    let sep_pen = CreatePen(PS_SOLID, 1, 0x00333333);
                    let old_pen = SelectObject(mem_dc, sep_pen);
                    MoveToEx(mem_dc, 16, current_y, std::ptr::null_mut());
                    LineTo(mem_dc, WINDOW_WIDTH - 16, current_y);
                    SelectObject(mem_dc, old_pen);
                    DeleteObject(sep_pen);
                    current_y += 8;
                }

                for (idx, result) in state.results.iter().enumerate() {
                    let is_selected = idx == state.selected_index;

                    let row_rect = RECT {
                        left: 12,
                        top: current_y,
                        right: WINDOW_WIDTH - 12,
                        bottom: current_y + ITEM_ROW_HEIGHT - 6,
                    };

                    if is_selected {
                        let sel_brush = CreateSolidBrush(0x003A3A3A);
                        FillRect(mem_dc, &row_rect, sel_brush);
                        DeleteObject(sel_brush);
                    }

                    SelectObject(mem_dc, font_item_title);
                    let base_color = if is_selected { 0x00FFFFFF } else { 0x00E0E0E0 };
                    draw_highlighted_title(
                        mem_dc,
                        &result.title,
                        &result.matched_indices,
                        32,
                        WINDOW_WIDTH - 32,
                        current_y + 4,
                        current_y + 26,
                        base_color,
                    );

                    if let Some(sub) = &result.subtitle {
                        SelectObject(mem_dc, font_item_sub);
                        SetTextColor(mem_dc, 0x00999999);
                        let mut sub_wide = to_wide(sub);
                        let mut sub_rect = RECT {
                            left: 32,
                            top: current_y + 26,
                            right: WINDOW_WIDTH - 32,
                            bottom: current_y + ITEM_ROW_HEIGHT - 8,
                        };
                        DrawTextW(
                            mem_dc,
                            sub_wide.as_mut_ptr(),
                            sub_wide.len() as i32 - 1,
                            &mut sub_rect,
                            DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                        );
                    }

                    current_y += ITEM_ROW_HEIGHT;
                }

                SelectObject(mem_dc, old_font);
                DeleteObject(font_title);
                DeleteObject(font_item_title);
                DeleteObject(font_item_sub);
            }
        }

        BitBlt(
            hdc,
            0,
            0,
            client_rect.right,
            client_rect.bottom,
            mem_dc,
            0,
            0,
            SRCCOPY,
        );

        SelectObject(mem_dc, old_bmp);
        DeleteObject(mem_bmp);
        DeleteDC(mem_dc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Graphics::Gdi::{
        DeleteObject as GdiDeleteObject, GetDC, GetPixel, ReleaseDC,
    };

    const BITMAP_WIDTH: i32 = 300;
    const BITMAP_HEIGHT: i32 = 40;

    struct OffscreenSurface {
        hdc: HDC,
        bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
        old_bitmap: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
    }

    impl OffscreenSurface {
        fn new() -> Self {
            unsafe {
                // A DC compatible with NULL defaults to monochrome; use a real
                // screen DC as the reference so the bitmap is full color, same
                // as render_ui does with the window's own paint HDC.
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

    const BASE_COLOR: COLORREF = 0x00E0E0E0;

    #[test]
    fn test_highlighted_title_paints_the_highlight_color() {
        let surface = OffscreenSurface::new();
        unsafe {
            draw_highlighted_title(
                surface.hdc,
                "Notepad",
                &[0, 1, 2],
                4,
                BITMAP_WIDTH - 4,
                4,
                BITMAP_HEIGHT - 4,
                BASE_COLOR,
            );
        }
        assert!(
            surface.contains_pixel(HIGHLIGHT_COLOR),
            "expected at least one pixel painted in the highlight color"
        );
    }

    #[test]
    fn test_unhighlighted_title_never_paints_the_highlight_color() {
        let surface = OffscreenSurface::new();
        unsafe {
            draw_highlighted_title(
                surface.hdc,
                "Notepad",
                &[],
                4,
                BITMAP_WIDTH - 4,
                4,
                BITMAP_HEIGHT - 4,
                BASE_COLOR,
            );
        }
        assert!(
            !surface.contains_pixel(HIGHLIGHT_COLOR),
            "no pixel should be the highlight color when nothing matched"
        );
    }

    #[test]
    fn test_highlight_color_constant_is_distinct_from_base_and_background() {
        const BACKGROUND_COLOR: COLORREF = 0x00000000;
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

        let wide = to_wide("😀");
        assert_eq!(wide.len(), 3, "expected a UTF-16 surrogate pair plus NUL");
    }
}
