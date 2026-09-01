use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::core::HSTRING;

pub fn show(title: &str, body: &str) {
    let _ = try_show(title, body);
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

    #[test]
    fn build_toast_produces_xml_the_real_parser_accepts() {
        let result = build_toast("WinSP", "failed to launch notepad.exe");
        assert!(result.is_ok(), "expected valid toast XML, got {result:?}");
    }

    #[test]
    fn build_toast_escapes_reserved_characters_into_valid_xml() {
        let result = build_toast("WinSP", r#"failed: <a> & "b" 'c'"#);
        assert!(
            result.is_ok(),
            "unescaped reserved characters should not produce malformed XML, got {result:?}"
        );
    }
}
