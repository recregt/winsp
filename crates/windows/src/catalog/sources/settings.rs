use winsp_core::models::{AppItem, AppTarget};

/// Returns a curated collection of standard Windows Settings shortcuts.
pub fn list_settings() -> Vec<AppItem> {
    vec![
        AppItem::new(
            "win-settings",
            "Settings",
            AppTarget::SettingUri("ms-settings:".into()),
        )
        .with_description("Windows System Settings")
        .with_keywords(vec![
            "config".into(),
            "control".into(),
            "preferences".into(),
        ]),
        AppItem::new(
            "win-display",
            "Display Settings",
            AppTarget::SettingUri("ms-settings:display".into()),
        )
        .with_description("Resolution, scaling, brightness, multiple displays")
        .with_keywords(vec![
            "monitor".into(),
            "screen".into(),
            "resolution".into(),
            "brightness".into(),
        ]),
        AppItem::new(
            "win-sound",
            "Sound Settings",
            AppTarget::SettingUri("ms-settings:sound".into()),
        )
        .with_description("Audio output, microphone, volume mixer")
        .with_keywords(vec![
            "audio".into(),
            "volume".into(),
            "mic".into(),
            "speaker".into(),
        ]),
        AppItem::new(
            "win-bluetooth",
            "Bluetooth & Devices",
            AppTarget::SettingUri("ms-settings:bluetooth".into()),
        )
        .with_description("Pair devices, mouse, keyboard, printers")
        .with_keywords(vec!["device".into(), "pair".into(), "wireless".into()]),
        AppItem::new(
            "win-network",
            "Network & Internet",
            AppTarget::SettingUri("ms-settings:network".into()),
        )
        .with_description("Wi-Fi, Ethernet, VPN, Proxy")
        .with_keywords(vec![
            "wifi".into(),
            "ethernet".into(),
            "ip".into(),
            "internet".into(),
        ]),
        AppItem::new(
            "win-apps",
            "Installed Apps",
            AppTarget::SettingUri("ms-settings:appsfeatures".into()),
        )
        .with_description("Uninstall and manage installed software")
        .with_keywords(vec!["uninstall".into(), "programs".into(), "remove".into()]),
        AppItem::new(
            "win-update",
            "Windows Update",
            AppTarget::SettingUri("ms-settings:windowsupdate".into()),
        )
        .with_description("Check for system updates and patches")
        .with_keywords(vec!["patch".into(), "upgrade".into(), "version".into()]),
        AppItem::new(
            "win-power",
            "Power & Sleep",
            AppTarget::SettingUri("ms-settings:powersleep".into()),
        )
        .with_description("Battery, sleep timeout, power mode")
        .with_keywords(vec!["battery".into(), "energy".into(), "hibernate".into()]),
        AppItem::new(
            "win-personalization",
            "Personalization",
            AppTarget::SettingUri("ms-settings:personalization".into()),
        )
        .with_description("Wallpaper, themes, colors, lock screen")
        .with_keywords(vec![
            "theme".into(),
            "wallpaper".into(),
            "background".into(),
            "dark mode".into(),
        ]),
        AppItem::new(
            "win-taskmanager",
            "Task Manager",
            AppTarget::Path("taskmgr.exe".into()),
        )
        .with_description("View running processes, performance, startup apps")
        .with_keywords(vec![
            "processes".into(),
            "kill".into(),
            "cpu".into(),
            "memory".into(),
        ]),
    ]
}
