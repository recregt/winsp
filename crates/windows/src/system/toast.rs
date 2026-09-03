use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::Win32::Foundation::APPMODEL_ERROR_NO_PACKAGE;
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::core::HSTRING;

use super::com::ComGuard;

pub fn show(title: &str, body: &str) {
    if !has_package_identity() {
        return;
    }
    let _com = ComGuard::new();
    let _ = try_show(title, body);
}

fn has_package_identity() -> bool {
    let mut length = 0u32;
    unsafe { GetCurrentPackageFullName(&mut length, None) != APPMODEL_ERROR_NO_PACKAGE }
}

fn try_show(title: &str, body: &str) -> windows::core::Result<()> {
    let toast = build_toast(title, body)?;
    ToastNotificationManager::CreateToastNotifier()?.Show(&toast)
}

fn build_toast(title: &str, body: &str) -> windows::core::Result<ToastNotification> {
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape_xml(title),
        escape_xml(body),
    );

    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;

    ToastNotification::CreateToastNotification(&doc)
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_xml_handles_all_reserved_characters() {
        assert_eq!(
            escape_xml(r#"<a> & "b" 'c'"#),
            "&lt;a&gt; &amp; &quot;b&quot; &apos;c&apos;"
        );
    }

    #[test]
    fn escape_xml_leaves_plain_text_untouched() {
        assert_eq!(
            escape_xml("failed to launch notepad.exe"),
            "failed to launch notepad.exe"
        );
    }
}
