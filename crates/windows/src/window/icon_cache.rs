use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};
use windows::core::HSTRING;

const MAX_CACHED_ICONS: usize = 512;

struct CachedIcon(HICON);

impl Drop for CachedIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

thread_local! {
    static CACHE: RefCell<HashMap<String, Option<CachedIcon>>> = RefCell::new(HashMap::new());
    static INSERTION_ORDER: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
}

pub fn icon_for_path(path: &str) -> Option<HICON> {
    if let Some(icon) = CACHE.with(|cache| {
        cache
            .borrow()
            .get(path)
            .map(|icon| icon.as_ref().map(|i| i.0))
    }) {
        return icon;
    }

    let icon = extract_icon(path);
    CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(path.to_string(), icon.map(CachedIcon))
    });
    INSERTION_ORDER.with(|order| order.borrow_mut().push_back(path.to_string()));
    evict_oldest_if_over_capacity();
    icon
}

fn evict_oldest_if_over_capacity() {
    INSERTION_ORDER.with(|order| {
        let mut order = order.borrow_mut();
        while order.len() > MAX_CACHED_ICONS {
            let Some(evicted_path) = order.pop_front() else {
                break;
            };
            CACHE.with(|cache| cache.borrow_mut().remove(&evicted_path));
        }
    });
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

    #[test]
    fn evicts_the_oldest_entry_and_frees_its_icon_handle_once_over_capacity() {
        let real_path = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(icon_for_path(&real_path).is_some());

        for i in 0..MAX_CACHED_ICONS {
            icon_for_path(&format!(r"C:\eviction-test\{i}.exe"));
        }

        let len = CACHE.with(|cache| cache.borrow().len());
        assert_eq!(len, MAX_CACHED_ICONS);

        let real_path_still_cached = CACHE.with(|cache| cache.borrow().contains_key(&real_path));
        assert!(
            !real_path_still_cached,
            "the oldest entry (holding a real icon handle) should have been evicted first"
        );
    }
}
