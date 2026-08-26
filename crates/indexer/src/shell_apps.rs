use winsp_core::{AppItem, AppTarget};

#[cfg(windows)]
pub fn start_menu_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(app_data) = std::env::var("APPDATA") {
        dirs.push(format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            app_data
        ));
    }
    if let Ok(program_data) = std::env::var("ProgramData") {
        dirs.push(format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            program_data
        ));
    }

    dirs.into_iter()
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}

#[cfg(windows)]
pub fn enumerate_installed_apps() -> Vec<AppItem> {
    let mut apps = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for dir_path in start_menu_dirs() {
        scan_directory_for_shortcuts(&dir_path, &mut apps, &mut seen_ids);
    }

    // 2. Add Essential Windows Built-in Tools
    let default_tools = vec![
        ("Calculator", "calc.exe", "Microsoft Calculator"),
        ("Notepad", "notepad.exe", "Fast text editor"),
        ("Windows Terminal", "wt.exe", "Modern terminal console"),
        ("Command Prompt", "cmd.exe", "Windows command interpreter"),
        (
            "PowerShell",
            "powershell.exe",
            "PowerShell scripting environment",
        ),
        ("Paint", "mspaint.exe", "Bitmap image editor"),
        ("Snipping Tool", "snippingtool.exe", "Screen capture tool"),
        (
            "Registry Editor",
            "regedit.exe",
            "Windows registry management",
        ),
        (
            "Control Panel",
            "control.exe",
            "Legacy system control panel",
        ),
        ("File Explorer", "explorer.exe", "File management"),
    ];

    for (name, exe, desc) in default_tools {
        let id = format!("builtin:{}", exe);
        if seen_ids.insert(id.clone()) {
            apps.push(AppItem::new(id, name, AppTarget::Path(exe.into())).with_description(desc));
        }
    }

    apps
}

#[cfg(windows)]
fn scan_directory_for_shortcuts(
    dir: &std::path::Path,
    apps: &mut Vec<AppItem>,
    seen_ids: &mut std::collections::HashSet<String>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_directory_for_shortcuts(&path, apps, seen_ids);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "lnk" || ext_lower == "url" {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let stem_lower = stem.to_lowercase();
                        if stem_lower.contains("uninstall") || stem_lower.contains("help") {
                            continue;
                        }

                        let id = format!("shortcut:{}", path.display());
                        if seen_ids.insert(id.clone()) {
                            apps.push(
                                AppItem::new(
                                    id,
                                    stem,
                                    AppTarget::Path(path.to_string_lossy().into()),
                                )
                                .with_description(path.to_string_lossy().to_string()),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(windows))]
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
