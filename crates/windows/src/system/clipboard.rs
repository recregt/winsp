use windows::ApplicationModel::DataTransfer::{Clipboard, DataPackage};
use windows::core::HSTRING;

pub fn copy(text: &str) -> bool {
    try_copy(text).is_ok()
}

fn try_copy(text: &str) -> windows::core::Result<()> {
    let package = DataPackage::new()?;
    package.SetText(&HSTRING::from(text))?;
    Clipboard::SetContent(&package)
}
