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

                        let identity = resolve_shortcut_target(&path, &ext_lower)
                            .unwrap_or_else(|| stem_lower.clone());
                        let id = format!("shortcut:{}", identity);
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

#[cfg(windows)]
fn resolve_shortcut_target(path: &std::path::Path, ext_lower: &str) -> Option<String> {
    match ext_lower {
        "lnk" => resolve_lnk_target(path),
        "url" => resolve_url_target(path),
        _ => None,
    }
}

#[cfg(windows)]
fn resolve_url_target(path: &std::path::Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    contents
        .lines()
        .find_map(|line| line.strip_prefix("URL="))
        .map(|url| url.trim().to_lowercase())
}

#[cfg(windows)]
fn resolve_lnk_target(path: &std::path::Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink};
    use windows::core::{Interface, PCWSTR};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let shell_link: IShellLinkW =
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist_file: IPersistFile = shell_link.cast().ok()?;

        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        persist_file
            .Load(PCWSTR(wide_path.as_ptr()), STGM_READ)
            .ok()?;

        let mut buffer = [0u16; 260];
        shell_link
            .GetPath(&mut buffer, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
            .ok()?;

        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        if end == 0 {
            None
        } else {
            Some(String::from_utf16_lossy(&buffer[..end]).to_lowercase())
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dedupes_by_stem_when_target_cannot_be_resolved() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        fs::write(per_user.path().join("Chrome.lnk"), []).unwrap();
        fs::write(system_wide.path().join("Chrome.lnk"), []).unwrap();

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Chrome");
    }

    #[test]
    fn keeps_distinctly_named_shortcuts_across_directories() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        fs::write(per_user.path().join("Chrome.lnk"), []).unwrap();
        fs::write(system_wide.path().join("Firefox.lnk"), []).unwrap();

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn dedupes_same_named_shortcut_with_same_resolved_target() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        let contents = "[InternetShortcut]\nURL=https://example.com/app\n";
        fs::write(per_user.path().join("App.url"), contents).unwrap();
        fs::write(system_wide.path().join("App.url"), contents).unwrap();

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn keeps_same_named_shortcuts_with_different_resolved_targets() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        fs::write(
            per_user.path().join("App.url"),
            "[InternetShortcut]\nURL=https://vendor-a.example.com/app\n",
        )
        .unwrap();
        fs::write(
            system_wide.path().join("App.url"),
            "[InternetShortcut]\nURL=https://vendor-b.example.com/app\n",
        )
        .unwrap();

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 2);
    }
}
