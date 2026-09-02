use windows::ApplicationModel::DataTransfer::{Clipboard, DataPackage};
use windows::core::HSTRING;

use super::com::ComGuard;

pub fn copy(text: &str) -> bool {
    let _com = ComGuard::new();
    try_copy(text).is_ok()
}

fn try_copy(text: &str) -> windows::core::Result<()> {
    let package = build_package(text)?;
    Clipboard::SetContent(&package)
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
    fn build_package_sets_text_content_successfully() {
        let result = build_package("2 + 2 = 4");
        assert!(
            result.is_ok(),
            "expected DataPackage construction to succeed, got {result:?}"
        );
    }
}
