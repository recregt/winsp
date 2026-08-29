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
struct ComGuard;

#[cfg(windows)]
impl ComGuard {
    fn new() -> Option<Self> {
        use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().ok()?;
        }
        Some(Self)
    }
}

#[cfg(windows)]
impl Drop for ComGuard {
    fn drop(&mut self) {
        use windows::Win32::System::Com::CoUninitialize;
        unsafe {
            CoUninitialize();
        }
    }
}

#[cfg(windows)]
pub fn enumerate_installed_apps() -> Vec<AppItem> {
    let mut apps = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let _com_guard = ComGuard::new();

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
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("URL") {
            Some(value.trim().to_lowercase())
        } else {
            None
        }
    })
}

#[cfg(windows)]
fn wide_str_from_buf(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(windows)]
fn expand_env_vars(raw: &[u16]) -> String {
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
    use windows::core::PCWSTR;

    let mut expanded = [0u16; 260];
    let written = unsafe { ExpandEnvironmentStringsW(PCWSTR(raw.as_ptr()), Some(&mut expanded)) };
    if written == 0 || written as usize > expanded.len() {
        wide_str_from_buf(raw)
    } else {
        wide_str_from_buf(&expanded)
    }
}

#[cfg(windows)]
fn resolve_lnk_target(path: &std::path::Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink};
    use windows::core::{Interface, PCWSTR};

    unsafe {
        let shell_link: IShellLinkW =
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist_file: IPersistFile = shell_link.cast().ok()?;

        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        persist_file
            .Load(PCWSTR(wide_path.as_ptr()), STGM_READ)
            .ok()?;

        let mut raw_path = [0u16; 260];
        shell_link
            .GetPath(&mut raw_path, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
            .ok()?;

        let target = expand_env_vars(&raw_path);
        if target.is_empty() {
            return None;
        }

        let mut args_buf = [0u16; 1024];
        let arguments = shell_link
            .GetArguments(&mut args_buf)
            .ok()
            .map(|()| wide_str_from_buf(&args_buf))
            .unwrap_or_default();

        Some(format!(
            "{}|{}",
            target.to_lowercase(),
            arguments.to_lowercase()
        ))
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
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(Some(0))
            .collect()
    }

    fn create_test_lnk(dir: &std::path::Path, name: &str, target: &str, args: &str) {
        let _guard = ComGuard::new();
        unsafe {
            let shell_link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();

            shell_link.SetPath(PCWSTR(wide(target).as_ptr())).unwrap();
            if !args.is_empty() {
                shell_link
                    .SetArguments(PCWSTR(wide(args).as_ptr()))
                    .unwrap();
            }

            let persist_file: IPersistFile = shell_link.cast().unwrap();
            let lnk_path = dir.join(format!("{name}.lnk"));
            persist_file
                .Save(PCWSTR(wide(&lnk_path.to_string_lossy()).as_ptr()), true)
                .unwrap();
        }
    }

    #[test]
    fn dedupes_real_lnk_shortcuts_with_same_target() {
        let _guard = ComGuard::new();
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        create_test_lnk(per_user.path(), "App", r"C:\Apps\App\app.exe", "");
        create_test_lnk(system_wide.path(), "App", r"C:\Apps\App\app.exe", "");

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn keeps_real_lnk_shortcuts_with_different_targets_same_name() {
        let _guard = ComGuard::new();
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        create_test_lnk(per_user.path(), "App", r"C:\VendorA\app.exe", "");
        create_test_lnk(system_wide.path(), "App", r"C:\VendorB\app.exe", "");

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn keeps_real_lnk_shortcuts_with_same_target_different_arguments() {
        let _guard = ComGuard::new();
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        create_test_lnk(
            per_user.path(),
            "App",
            r"C:\Apps\App\app.exe",
            "--profile=work",
        );
        create_test_lnk(
            system_wide.path(),
            "App",
            r"C:\Apps\App\app.exe",
            "--profile=personal",
        );

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn expands_environment_variables_in_lnk_target() {
        let program_files = std::env::var("ProgramFiles").unwrap();
        let raw = wide(r"%ProgramFiles%\App\app.exe");

        let expanded = expand_env_vars(&raw);

        assert_eq!(
            expanded.to_lowercase(),
            format!(r"{program_files}\App\app.exe").to_lowercase()
        );
    }

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

    #[test]
    fn dedupes_url_shortcuts_regardless_of_key_case_and_spacing() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        fs::write(
            per_user.path().join("App.url"),
            "[InternetShortcut]\nURL=https://example.com/app\n",
        )
        .unwrap();
        fs::write(
            system_wide.path().join("App.url"),
            "[InternetShortcut]\nurl = https://example.com/app\n",
        )
        .unwrap();

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 1);
    }
}
