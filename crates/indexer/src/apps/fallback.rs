use winsp_core::{AppItem, AppTarget};

pub fn enumerate_installed_apps() -> Vec<AppItem> {
    vec![
        AppItem::new(
            "calc",
            "Calculator",
            AppTarget::Aumid("Microsoft.WindowsCalculator".into()),
        )
        .with_description("Standard & Scientific Calculator"),
        AppItem::new("notepad", "Notepad", AppTarget::Path("notepad.exe".into()))
            .with_description("Text Editor"),
        AppItem::new(
            "code",
            "Visual Studio Code",
            AppTarget::Path("code.exe".into()),
        )
        .with_description("Code Editing. Redefined.")
        .with_keywords(vec!["vsc".into(), "ide".into(), "editor".into()]),
        AppItem::new(
            "terminal",
            "Windows Terminal",
            AppTarget::Path("wt.exe".into()),
        )
        .with_description("PowerShell, CMD, WSL command line")
        .with_keywords(vec![
            "cmd".into(),
            "powershell".into(),
            "console".into(),
            "bash".into(),
        ]),
        AppItem::new(
            "chrome",
            "Google Chrome",
            AppTarget::Path("chrome.exe".into()),
        )
        .with_description("Fast, secure web browser")
        .with_keywords(vec!["browser".into(), "internet".into(), "google".into()]),
        AppItem::new(
            "spotify",
            "Spotify",
            AppTarget::Aumid("SpotifyAB.SpotifyMusic".into()),
        )
        .with_description("Music and podcasts")
        .with_keywords(vec!["music".into(), "audio".into(), "songs".into()]),
    ]
}
