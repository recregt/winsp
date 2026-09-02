use std::cell::RefCell;
use std::collections::HashMap;

use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::core::HSTRING;

thread_local! {
    static CACHE: RefCell<HashMap<String, Option<HICON>>> = RefCell::new(HashMap::new());
}

pub fn icon_for_path(path: &str) -> Option<HICON> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(icon) = cache.get(path) {
            return *icon;
        }
        let icon = extract_icon(path);
        cache.insert(path.to_string(), icon);
        icon
    })
}

fn extract_icon(path: &str) -> Option<HICON> {
    let wide = HSTRING::from(path);
    let mut info = SHFILEINFOW::default();

    let result = unsafe {
        SHGetFileInfoW(
            &wide,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };

    (result != 0 && !info.hIcon.is_invalid()).then_some(info.hIcon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_path_yields_no_icon_without_panicking() {
        assert!(icon_for_path(r"C:\definitely\not\a\real\path.exe").is_none());
    }

    #[test]
    fn repeated_lookups_of_the_same_missing_path_stay_consistent() {
        assert_eq!(
            icon_for_path(r"C:\also\not\real.exe"),
            icon_for_path(r"C:\also\not\real.exe")
        );
    }
}
