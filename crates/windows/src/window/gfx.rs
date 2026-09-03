use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AddFontMemResourceEx, CreateFontW, CreatePen, CreateSolidBrush, DEFAULT_GUI_FONT, DT_CENTER,
    DT_LEFT, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, FONT_CHARSET, FONT_CLIP_PRECISION,
    FONT_OUTPUT_PRECISION, FONT_QUALITY, FW_NORMAL, FW_SEMIBOLD, FillRect, GetStockObject,
    GetTextExtentPoint32W, HDC, HFONT, HGDIOBJ, LineTo, MoveToEx, PS_SOLID, SelectObject,
    SetTextColor,
};
use windows::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DrawIconEx, HICON};
use windows::core::HSTRING;

use super::icon_cache::CachedIcon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    fn to_win32(self) -> RECT {
        RECT {
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(u32);

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
    }

    pub const fn hex(rgb: u32) -> Self {
        let r = ((rgb >> 16) & 0xFF) as u8;
        let g = ((rgb >> 8) & 0xFF) as u8;
        let b = (rgb & 0xFF) as u8;
        Self::rgb(r, g, b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    SemiBold,
}

impl FontWeight {
    fn as_gdi(self) -> i32 {
        match self {
            FontWeight::Normal => FW_NORMAL.0 as i32,
            FontWeight::SemiBold => FW_SEMIBOLD.0 as i32,
        }
    }
}

pub struct Canvas {
    hdc: HDC,
}

impl Canvas {
    pub(super) fn new(hdc: HDC) -> Self {
        Self { hdc }
    }

    #[cfg(feature = "test-support")]
    unsafe fn from_raw(hdc: HDC) -> Self {
        Self { hdc }
    }

    pub fn fill_rect(&self, rect: Rect, color: Color) {
        unsafe {
            let brush = CreateSolidBrush(COLORREF(color.0));
            FillRect(self.hdc, &rect.to_win32(), brush);
            let _ = DeleteObject(brush.into());
        }
    }

    pub fn set_text_color(&self, color: Color) {
        unsafe {
            SetTextColor(self.hdc, COLORREF(color.0));
        }
    }

    pub fn draw_text(&self, text: &str, rect: Rect) {
        unsafe {
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let mut gdi_rect: RECT = rect.to_win32();
            DrawTextW(
                self.hdc,
                &mut wide,
                &mut gdi_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }

    pub fn draw_text_measured(&self, text: &str, rect: Rect) -> i32 {
        unsafe {
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let mut gdi_rect: RECT = rect.to_win32();
            DrawTextW(
                self.hdc,
                &mut wide,
                &mut gdi_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let mut size = std::mem::MaybeUninit::<SIZE>::uninit();
            if GetTextExtentPoint32W(self.hdc, &wide, size.as_mut_ptr()).as_bool() {
                size.assume_init().cx
            } else {
                0
            }
        }
    }

    pub fn draw_line(&self, from: (i32, i32), to: (i32, i32), color: Color) {
        unsafe {
            let pen = CreatePen(PS_SOLID, 1, COLORREF(color.0));
            let old_pen = SelectObject(self.hdc, pen.into());
            let _ = MoveToEx(self.hdc, from.0, from.1, None);
            let _ = LineTo(self.hdc, to.0, to.1);
            SelectObject(self.hdc, old_pen);
            let _ = DeleteObject(pen.into());
        }
    }

    fn draw_icon(&self, icon: HICON, rect: Rect) {
        unsafe {
            let _ = DrawIconEx(
                self.hdc,
                rect.left,
                rect.top,
                icon,
                rect.right - rect.left,
                rect.bottom - rect.top,
                0,
                None,
                DI_NORMAL,
            );
        }
    }

    pub fn draw_cached_icon(&self, icon: &CachedIcon, rect: Rect) {
        self.draw_icon(icon.handle(), rect);
    }

    pub fn draw_icon_glyph(&self, glyph: char, rect: Rect) {
        unsafe {
            let mut buf = [0u16; 2];
            let wide = glyph.encode_utf16(&mut buf);
            let mut gdi_rect: RECT = rect.to_win32();
            DrawTextW(
                self.hdc,
                wide,
                &mut gdi_rect,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }

    pub fn select_font(&self, font: &Font) -> FontGuard {
        let old_font = unsafe { SelectObject(self.hdc, font.handle.into()) };
        FontGuard {
            hdc: self.hdc,
            old_font,
        }
    }
}

pub struct Font {
    handle: HFONT,
}

impl Font {
    pub fn new(name: &str, size: i32, weight: FontWeight) -> Self {
        unsafe {
            let font_name = HSTRING::from(name);
            let handle = CreateFontW(
                size,
                0,
                0,
                0,
                weight.as_gdi(),
                0,
                0,
                0,
                FONT_CHARSET(0),
                FONT_OUTPUT_PRECISION(0),
                FONT_CLIP_PRECISION(0),
                FONT_QUALITY(0),
                0,
                &font_name,
            );
            let handle = if handle.is_invalid() {
                HFONT(GetStockObject(DEFAULT_GUI_FONT).0)
            } else {
                handle
            };
            Self { handle }
        }
    }

    pub fn register(data: &'static [u8]) -> bool {
        let num_fonts: u32 = 0;
        let result = unsafe {
            AddFontMemResourceEx(
                data.as_ptr() as *const _,
                data.len() as u32,
                None,
                &num_fonts,
            )
        };
        !result.is_invalid()
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.handle.into());
        }
    }
}

unsafe impl Send for Font {}
unsafe impl Sync for Font {}

pub struct FontGuard {
    hdc: HDC,
    old_font: HGDIOBJ,
}

impl Drop for FontGuard {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.hdc, self.old_font);
        }
    }
}

#[cfg(feature = "test-support")]
pub mod testing {
    use super::{Canvas, Color};
    use windows::Win32::Foundation::{COLORREF, RECT};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject,
        FillRect, GetDC, GetPixel, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SelectObject, SetBkMode,
        TRANSPARENT,
    };

    pub struct OffscreenSurface {
        hdc: HDC,
        bitmap: HBITMAP,
        old_bitmap: HGDIOBJ,
        width: i32,
        height: i32,
    }

    impl OffscreenSurface {
        pub fn new(width: i32, height: i32) -> Self {
            unsafe {
                let screen_dc = GetDC(None);
                let hdc = CreateCompatibleDC(Some(screen_dc));
                let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
                ReleaseDC(None, screen_dc);
                let old_bitmap = SelectObject(hdc, bitmap.into());

                let fill_rect = RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: height,
                };
                let bg_brush = CreateSolidBrush(COLORREF(0x00000000));
                FillRect(hdc, &fill_rect, bg_brush);
                let _ = DeleteObject(bg_brush.into());
                SetBkMode(hdc, TRANSPARENT);

                Self {
                    hdc,
                    bitmap,
                    old_bitmap,
                    width,
                    height,
                }
            }
        }

        pub fn canvas(&self) -> Canvas {
            unsafe { Canvas::from_raw(self.hdc) }
        }

        pub fn contains_pixel(&self, color: Color) -> bool {
            unsafe {
                for y in 0..self.height {
                    for x in 0..self.width {
                        if GetPixel(self.hdc, x, y) == COLORREF(color.0) {
                            return true;
                        }
                    }
                }
            }
            false
        }

        pub fn contains_pixel_other_than(&self, color: Color) -> bool {
            unsafe {
                for y in 0..self.height {
                    for x in 0..self.width {
                        if GetPixel(self.hdc, x, y) != COLORREF(color.0) {
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
                let _ = DeleteObject(self.bitmap.into());
                let _ = DeleteDC(self.hdc);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::OffscreenSurface;
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{IDI_APPLICATION, LoadIconW};

    #[test]
    fn draw_icon_paints_pixels_into_its_rect() {
        let surface = OffscreenSurface::new(40, 40);
        let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap();

        surface.canvas().draw_icon(
            icon,
            Rect {
                left: 4,
                top: 4,
                right: 36,
                bottom: 36,
            },
        );

        assert!(surface.contains_pixel_other_than(Color(0x00000000)));
    }

    #[test]
    fn draw_icon_glyph_paints_the_selected_text_color() {
        let surface = OffscreenSurface::new(40, 40);
        let canvas = surface.canvas();
        let font = Font::new("Arial", 24, FontWeight::Normal);

        let _font_guard = canvas.select_font(&font);
        canvas.set_text_color(Color(0x00FF00FF));
        canvas.draw_icon_glyph(
            'A',
            Rect {
                left: 0,
                top: 0,
                right: 40,
                bottom: 40,
            },
        );

        assert!(surface.contains_pixel(Color(0x00FF00FF)));
    }
}
