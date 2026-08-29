use windows_sys::Win32::Foundation::{COLORREF, HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
    DT_LEFT, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FW_NORMAL, FW_SEMIBOLD,
    FillRect, HDC, LineTo, MoveToEx, PS_SOLID, SRCCOPY, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::{APP_STATE, ITEM_ROW_HEIGHT, SEARCH_BAR_HEIGHT, WINDOW_WIDTH, to_wide};

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
                    SetTextColor(mem_dc, if is_selected { 0x00FFFFFF } else { 0x00E0E0E0 });
                    let mut title_wide = to_wide(&result.title);
                    let mut title_rect = RECT {
                        left: 32,
                        top: current_y + 4,
                        right: WINDOW_WIDTH - 32,
                        bottom: current_y + 26,
                    };
                    DrawTextW(
                        mem_dc,
                        title_wide.as_mut_ptr(),
                        title_wide.len() as i32 - 1,
                        &mut title_rect,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
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
