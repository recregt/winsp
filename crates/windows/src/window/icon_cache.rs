use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use lru::LruCache;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};
use windows::core::HSTRING;

use crate::system::com::ComGuard;
use crate::system::threadpool::spawn_on_threadpool;

const MAX_CACHED_ICONS: usize = 512;

pub struct CachedIcon(HICON);

unsafe impl Send for CachedIcon {}
unsafe impl Sync for CachedIcon {}

impl CachedIcon {
    pub fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for CachedIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

enum IconState {
    Pending,
    Ready(Option<Arc<CachedIcon>>),
}

fn cache() -> &'static Mutex<LruCache<String, IconState>> {
    static CACHE: OnceLock<Mutex<LruCache<String, IconState>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(MAX_CACHED_ICONS).unwrap())))
}

static REPAINT_HWND: AtomicIsize = AtomicIsize::new(0);
static REPAINT_PENDING: AtomicBool = AtomicBool::new(false);

pub(super) fn register_repaint_target(hwnd: HWND) {
    REPAINT_HWND.store(hwnd.0 as isize, Ordering::Release);
}

pub fn mark_paint_started() {
    REPAINT_PENDING.store(false, Ordering::Release);
}

fn request_repaint() {
    if REPAINT_PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    let raw = REPAINT_HWND.load(Ordering::Acquire);
    if raw != 0 {
        unsafe {
            let _ = InvalidateRect(Some(HWND(raw as *mut _)), None, false);
        }
    }
}

fn dispatch_extraction(path: String) {
    let cleanup_path = path.clone();

    let submitted = spawn_on_threadpool(move || {
        let icon = {
            let _com = ComGuard::new();
            extract_icon(&path).map(CachedIcon).map(Arc::new)
        };

        let still_wanted = if let Ok(mut cache) = cache().lock() {
            if matches!(cache.peek(&path), Some(IconState::Pending)) {
                cache.put(path, IconState::Ready(icon));
                true
            } else {
                false
            }
        } else {
            false
        };

        if still_wanted {
            request_repaint();
        }
    });

    if !submitted && let Ok(mut cache) = cache().lock() {
        cache.put(cleanup_path, IconState::Ready(None));
    }
}

pub fn icon_for_path(path: &str) -> Option<Arc<CachedIcon>> {
    let mut cache = cache().lock().ok()?;
    match cache.get(path) {
        Some(IconState::Ready(icon)) => return icon.clone(),
        Some(IconState::Pending) => return None,
        None => {}
    }

    let owned = path.to_string();
    cache.put(owned.clone(), IconState::Pending);
    drop(cache);
    dispatch_extraction(owned);
    None
}

fn extract_icon(path: &str) -> Option<HICON> {
    let wide = HSTRING::from(path);
    let mut info = std::mem::MaybeUninit::<SHFILEINFOW>::uninit();

    let result = unsafe {
        SHGetFileInfoW(
            &wide,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(info.as_mut_ptr()),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if result == 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };

    (!info.hIcon.is_invalid()).then_some(info.hIcon)
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
        REPAINT_PENDING.store(false, Ordering::Release);
        REPAINT_HWND.store(0, Ordering::Release);
        guard
    }

    #[test]
    fn register_repaint_target_replaces_the_previous_hwnd() {
        let _guard = reset_for_test();

        register_repaint_target(HWND(0x1000 as *mut _));
        assert_eq!(REPAINT_HWND.load(Ordering::Acquire), 0x1000);

        register_repaint_target(HWND(0x2000 as *mut _));
        assert_eq!(
            REPAINT_HWND.load(Ordering::Acquire),
            0x2000,
            "a freshly created window must replace the previous repaint target, not be ignored"
        );
    }

    fn wait_for_icon(path: &str) -> Arc<CachedIcon> {
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
        assert!(icon_for_path(r"C:\also\not\real.exe").is_none());
        assert!(icon_for_path(r"C:\also\not\real.exe").is_none());
    }

    #[test]
    fn a_completion_for_an_already_evicted_entry_does_not_resurrect_it() {
        let _guard = reset_for_test();
        let real_path = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        assert!(icon_for_path(&real_path).is_none());
        cache().lock().unwrap().pop(&real_path);

        std::thread::sleep(std::time::Duration::from_millis(500));

        let resurrected = cache().lock().unwrap().contains(&real_path);
        assert!(
            !resurrected,
            "a late completion for an evicted path must not reinsert it"
        );
    }

    #[test]
    fn request_repaint_coalesces_until_a_paint_starts() {
        let _guard = reset_for_test();
        assert!(!REPAINT_PENDING.load(Ordering::Acquire));

        request_repaint();
        assert!(REPAINT_PENDING.load(Ordering::Acquire));

        request_repaint();
        assert!(REPAINT_PENDING.load(Ordering::Acquire));

        mark_paint_started();
        assert!(!REPAINT_PENDING.load(Ordering::Acquire));
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

        let len = cache().lock().unwrap().len();
        assert!(
            len <= MAX_CACHED_ICONS,
            "LruCache must never grow past its configured capacity"
        );

        let real_path_still_cached = cache().lock().unwrap().contains(&real_path);
        assert!(
            !real_path_still_cached,
            "the oldest entry (holding a real icon handle) should have been evicted first"
        );
    }

    #[test]
    fn a_caller_holding_an_arc_keeps_the_icon_valid_after_cache_eviction() {
        let _guard = reset_for_test();
        let real_path = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let held = wait_for_icon(&real_path);

        cache().lock().unwrap().pop(&real_path);

        assert_eq!(
            Arc::strong_count(&held),
            1,
            "the cache's own reference should be gone, proving eviction doesn't wait on callers"
        );

        let surface = super::super::testing::OffscreenSurface::new(40, 40);
        surface.canvas().draw_icon(
            held.handle(),
            super::super::Rect {
                left: 4,
                top: 4,
                right: 36,
                bottom: 36,
            },
        );
        assert!(
            surface.contains_pixel_other_than(super::super::Color(0x00000000)),
            "the icon handle must still be valid and drawable after the cache dropped its copy"
        );
    }
}
