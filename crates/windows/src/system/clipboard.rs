use windows::ApplicationModel::DataTransfer::{Clipboard, DataPackage};
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::core::HSTRING;

use super::com::ComGuard;

pub fn copy(text: &str) -> bool {
    let _com = ComGuard::new();
    try_copy(text).is_ok()
}

/// Reads clipboard text via the classic synchronous clipboard API rather
/// than WinRT's `Clipboard::GetContent().GetTextAsync()`: that async call
/// completes on the calling (STA) thread's message queue, so awaiting it
/// with a blocking wait from inside the UI thread's own message loop would
/// deadlock — the thread can't pump the very message that would complete it.
pub fn paste() -> Option<String> {
    try_paste().ok()
}

fn try_copy(text: &str) -> windows::core::Result<()> {
    let package = build_package(text)?;
    Clipboard::SetContent(&package)?;
    Clipboard::Flush()
}

fn build_package(text: &str) -> windows::core::Result<DataPackage> {
    let package = DataPackage::new()?;
    package.SetText(&HSTRING::from(text))?;
    Ok(package)
}

fn try_paste() -> windows::core::Result<String> {
    unsafe {
        OpenClipboard(None)?;
        let result = read_clipboard_unicode_text();
        let _ = CloseClipboard();
        result
    }
}

unsafe fn read_clipboard_unicode_text() -> windows::core::Result<String> {
    unsafe {
        let handle = GetClipboardData(CF_UNICODETEXT.0 as u32)?;
        let source = HGLOBAL(handle.0);
        let ptr = GlobalLock(source) as *const u16;
        if ptr.is_null() {
            return Ok(String::new());
        }
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        let _ = GlobalUnlock(source);
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_returns_what_was_just_copied() {
        if !copy("WinSpTest_ClipboardRoundTrip") {
            eprintln!("skipping: clipboard is not available in this environment");
            return;
        }

        assert_eq!(paste(), Some("WinSpTest_ClipboardRoundTrip".to_string()));
    }
}
