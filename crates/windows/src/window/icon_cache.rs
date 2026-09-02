use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::Threading::{PTP_CALLBACK_INSTANCE, TrySubmitThreadpoolCallback};
use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};
use windows::core::HSTRING;

const MAX_CACHED_ICONS: usize = 512;

struct CachedIcon(HICON);

unsafe impl Send for CachedIcon {}

impl Drop for CachedIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

enum IconState {
    Pending,
    Ready(Option<CachedIcon>),
}

fn cache() -> &'static Mutex<HashMap<String, IconState>> {
    static CACHE: OnceLock<Mutex<HashMap<String, IconState>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn insertion_order() -> &'static Mutex<VecDeque<String>> {
    static ORDER: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
    ORDER.get_or_init(|| Mutex::new(VecDeque::new()))
}

static REPAINT_HWND: OnceLock<isize> = OnceLock::new();

pub(super) fn register_repaint_target(hwnd: HWND) {
    let _ = REPAINT_HWND.set(hwnd.0 as isize);
}

fn request_repaint() {
    if let Some(&raw) = REPAINT_HWND.get() {
        unsafe {
            let _ = InvalidateRect(Some(HWND(raw as *mut _)), None, true);
        }
    }
}

unsafe extern "system" fn run_extraction(_instance: PTP_CALLBACK_INSTANCE, context: *mut c_void) {
    let path = *unsafe { Box::from_raw(context as *mut String) };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let icon = extract_icon(&path);
    unsafe {
        CoUninitialize();
    }

    if let Ok(mut cache) = cache().lock() {
        cache.insert(path, IconState::Ready(icon.map(CachedIcon)));
    }
    request_repaint();
}

fn dispatch_extraction(path: String) {
    let context = Box::into_raw(Box::new(path)) as *mut c_void;
    let submitted =
        unsafe { TrySubmitThreadpoolCallback(Some(run_extraction), Some(context), None) };

    if submitted.is_err() {
        let path = *unsafe { Box::from_raw(context as *mut String) };
        if let Ok(mut cache) = cache().lock() {
            cache.remove(&path);
        }
    }
}

pub fn icon_for_path(path: &str) -> Option<HICON> {
    {
        let cache = cache().lock().ok()?;
        match cache.get(path) {
            Some(IconState::Ready(icon)) => return icon.as_ref().map(|i| i.0),
            Some(IconState::Pending) => return None,
            None => {}
        }
    }

    let owned = path.to_string();
    if let Ok(mut cache) = cache().lock() {
        cache.insert(owned.clone(), IconState::Pending);
    }
    if let Ok(mut order) = insertion_order().lock() {
        order.push_back(owned.clone());
    }
    evict_oldest_if_over_capacity();
    dispatch_extraction(owned);
    None
}

fn evict_oldest_if_over_capacity() {
    let Ok(mut order) = insertion_order().lock() else {
        return;
    };
    while order.len() > MAX_CACHED_ICONS {
        let Some(evicted_path) = order.pop_front() else {
            break;
        };
        if let Ok(mut cache) = cache().lock() {
            cache.remove(&evicted_path);
        }
    }
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

    fn reset_for_test() -> std::sync::MutexGuard<'static, ()> {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut c) = cache().lock() {
            c.clear();
        }
        if let Ok(mut o) = insertion_order().lock() {
            o.clear();
        }
        guard
    }

    fn wait_for_icon(path: &str) -> HICON {
        for _ in 0..200 {
            if let Some(icon) = icon_for_path(path) {
                return icon;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("icon for {path} did not resolve in time");
    }

    #[test]
    fn missing_path_yields_no_icon_without_panicking() {
        let _guard = reset_for_test();
        assert!(icon_for_path(r"C:\definitely\not\a\real\path.exe").is_none());
    }

    #[test]
    fn repeated_lookups_of_the_same_missing_path_stay_consistent() {
        let _guard = reset_for_test();
        assert_eq!(
            icon_for_path(r"C:\also\not\real.exe"),
            icon_for_path(r"C:\also\not\real.exe")
        );
    }

    #[test]
    fn evicts_the_oldest_entry_and_frees_its_icon_handle_once_over_capacity() {
        let _guard = reset_for_test();
        let real_path = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        wait_for_icon(&real_path);

        for i in 0..MAX_CACHED_ICONS {
            icon_for_path(&format!(r"C:\eviction-test\{i}.exe"));
        }

        let tracked = insertion_order().lock().unwrap().len();
        assert_eq!(
            tracked, MAX_CACHED_ICONS,
            "insertion_order is only ever touched synchronously by the calling thread, \
             unlike cache which the background worker can still be draining into"
        );

        let real_path_still_cached = cache().lock().unwrap().contains_key(&real_path);
        assert!(
            !real_path_still_cached,
            "the oldest entry (holding a real icon handle) should have been evicted first"
        );
    }
}
