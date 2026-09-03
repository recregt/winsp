use winsp_core::models::{AppItem, AppTarget};

use super::apps::resolve_system_exe;

/// Returns a curated collection of standard Windows Settings shortcuts.
pub fn list_settings() -> Vec<AppItem> {
    vec![
        AppItem::new(
            "win-settings",
            "Settings",
            AppTarget::Uri("ms-settings:".into()),
        )
        .with_description("Windows System Settings")
        .with_icon_glyph('\u{E713}')
        .with_keywords(vec![
            "config".into(),
            "control".into(),
            "preferences".into(),
        ]),
        AppItem::new(
            "win-display",
            "Display Settings",
            AppTarget::Uri("ms-settings:display".into()),
        )
        .with_description("Resolution, scaling, brightness, multiple displays")
        .with_icon_glyph('\u{E7F4}')
        .with_keywords(vec![
            "monitor".into(),
            "screen".into(),
            "resolution".into(),
            "brightness".into(),
        ]),
        AppItem::new(
            "win-sound",
            "Sound Settings",
            AppTarget::Uri("ms-settings:sound".into()),
        )
        .with_description("Audio output, microphone, volume mixer")
        .with_icon_glyph('\u{E7F5}')
        .with_keywords(vec![
            "audio".into(),
            "volume".into(),
            "mic".into(),
            "speaker".into(),
        ]),
        AppItem::new(
            "win-bluetooth",
            "Bluetooth & Devices",
            AppTarget::Uri("ms-settings:bluetooth".into()),
        )
        .with_description("Pair devices, mouse, keyboard, printers")
        .with_icon_glyph('\u{E702}')
        .with_keywords(vec!["device".into(), "pair".into(), "wireless".into()]),
        AppItem::new(
            "win-network",
            "Network & Internet",
            AppTarget::Uri("ms-settings:network".into()),
        )
        .with_description("Wi-Fi, Ethernet, VPN, Proxy")
        .with_icon_glyph('\u{E968}')
        .with_keywords(vec![
            "wifi".into(),
            "ethernet".into(),
            "ip".into(),
            "internet".into(),
        ]),
        AppItem::new(
            "win-apps",
            "Installed Apps",
            AppTarget::Uri("ms-settings:appsfeatures".into()),
        )
        .with_description("Uninstall and manage installed software")
        .with_icon_glyph('\u{ED35}')
        .with_keywords(vec!["uninstall".into(), "programs".into(), "remove".into()]),
        AppItem::new(
            "win-update",
            "Windows Update",
            AppTarget::Uri("ms-settings:windowsupdate".into()),
        )
        .with_description("Check for system updates and patches")
        .with_icon_glyph('\u{E777}')
        .with_keywords(vec!["patch".into(), "upgrade".into(), "version".into()]),
        AppItem::new(
            "win-power",
            "Power & Sleep",
            AppTarget::Uri("ms-settings:powersleep".into()),
        )
        .with_description("Battery, sleep timeout, power mode")
        .with_icon_glyph('\u{E7E8}')
        .with_keywords(vec!["battery".into(), "energy".into(), "hibernate".into()]),
        AppItem::new(
            "win-personalization",
            "Personalization",
            AppTarget::Uri("ms-settings:personalization".into()),
        )
        .with_description("Wallpaper, themes, colors, lock screen")
        .with_icon_glyph('\u{E771}')
        .with_keywords(vec![
            "theme".into(),
            "wallpaper".into(),
            "background".into(),
            "dark mode".into(),
        ]),
        {
            let mut item = AppItem::new(
                "win-taskmanager",
                "Task Manager",
                AppTarget::Path("taskmgr.exe".into()),
            );
            if let Some(icon) = resolve_system_exe("taskmgr.exe") {
                item = item.with_icon(icon);
            }
            item
        }
        .with_description("View running processes, performance, startup apps")
        .with_keywords(vec![
            "processes".into(),
            "kill".into(),
            "cpu".into(),
            "memory".into(),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use winsp_core::models::IconSource;

    #[test]
    fn every_setting_uri_entry_carries_a_glyph_icon() {
        for item in list_settings() {
            if matches!(item.target, AppTarget::Uri(_)) {
                assert!(
                    matches!(item.icon, Some(IconSource::Glyph(_))),
                    "{} has no glyph icon",
                    item.name
                );
            }
        }
    }
}
