use windows::ApplicationModel::DataTransfer::{Clipboard, DataPackage, StandardDataFormats};
use windows::core::HSTRING;

use super::com::ComGuard;

pub fn copy(text: &str) -> bool {
    let _com = ComGuard::new();
    try_copy(text).is_ok()
}

pub fn paste() -> Option<String> {
    let _com = ComGuard::new();
    try_paste().ok()
}

fn try_copy(text: &str) -> windows::core::Result<()> {
    let package = build_package(text)?;
    Clipboard::SetContent(&package)?;
    Clipboard::Flush()
}

fn try_paste() -> windows::core::Result<String> {
    let content = Clipboard::GetContent()?;
    if !content.Contains(&StandardDataFormats::Text()?)? {
        return Ok(String::new());
    }
    Ok(content.GetTextAsync()?.join()?.to_string_lossy())
}

fn build_package(text: &str) -> windows::core::Result<DataPackage> {
    let package = DataPackage::new()?;
    package.SetText(&HSTRING::from(text))?;
    Ok(package)
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
