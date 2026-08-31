use crate::system::registry::to_wide;
use windows_sys::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    AddFontMemResourceEx, CreateFontW, CreatePen, CreateSolidBrush, DEFAULT_GUI_FONT, DT_LEFT,
    DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, FW_NORMAL, FW_SEMIBOLD, FillRect,
    GetStockObject, GetTextExtentPoint32W, HDC, HFONT, HGDIOBJ, LineTo, MoveToEx, PS_SOLID,
    SelectObject, SetTextColor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl From<Rect> for RECT {
    fn from(rect: Rect) -> RECT {
        RECT {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub COLORREF);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    SemiBold,
}

impl FontWeight {
    fn as_gdi(self) -> i32 {
        match self {
            FontWeight::Normal => FW_NORMAL as i32,
            FontWeight::SemiBold => FW_SEMIBOLD as i32,
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

    /// # Safety
    /// `hdc` must be a valid device context handle for the lifetime of the returned `Canvas`.
    pub unsafe fn from_raw(hdc: HDC) -> Self {
        Self { hdc }
    }

    pub fn fill_rect(&self, rect: Rect, color: Color) {
        unsafe {
            let brush = CreateSolidBrush(color.0);
            FillRect(self.hdc, &rect.into(), brush);
            DeleteObject(brush);
        }
    }

    pub fn set_text_color(&self, color: Color) {
        unsafe {
            SetTextColor(self.hdc, color.0);
        }
    }

    pub fn draw_text(&self, text: &str, rect: Rect) {
        unsafe {
            let mut wide = to_wide(text);
            let mut rect: RECT = rect.into();
            DrawTextW(
                self.hdc,
                wide.as_mut_ptr(),
                wide.len() as i32 - 1,
                &mut rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }

    pub fn draw_text_measured(&self, text: &str, rect: Rect) -> i32 {
        unsafe {
            let mut wide = to_wide(text);
            let mut gdi_rect: RECT = rect.into();
            DrawTextW(
                self.hdc,
                wide.as_mut_ptr(),
                wide.len() as i32 - 1,
                &mut gdi_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let mut size = std::mem::zeroed::<SIZE>();
            GetTextExtentPoint32W(self.hdc, wide.as_ptr(), wide.len() as i32 - 1, &mut size);
            size.cx
        }
    }

    pub fn draw_line(&self, from: (i32, i32), to: (i32, i32), color: Color) {
        unsafe {
            let pen = CreatePen(PS_SOLID, 1, color.0);
            let old_pen = SelectObject(self.hdc, pen);
            MoveToEx(self.hdc, from.0, from.1, std::ptr::null_mut());
            LineTo(self.hdc, to.0, to.1);
            SelectObject(self.hdc, old_pen);
            DeleteObject(pen);
        }
    }

    pub fn select_font(&self, font: &Font) -> FontGuard {
        let old_font = unsafe { SelectObject(self.hdc, font.handle) };
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
            let font_name = to_wide(name);
            let handle = CreateFontW(
                size,
                0,
                0,
                0,
                weight.as_gdi(),
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
            let handle = if handle.is_null() {
                GetStockObject(DEFAULT_GUI_FONT) as HFONT
            } else {
                handle
            };
            Self { handle }
        }
    }
}

pub fn register_embedded_font(data: &'static [u8]) -> bool {
    let mut num_fonts: u32 = 0;
    let result = unsafe {
        AddFontMemResourceEx(
            data.as_ptr() as *const _,
            data.len() as u32,
            std::ptr::null(),
            &mut num_fonts as *mut u32 as *const u32,
        )
    };
    !result.is_null()
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.handle);
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
