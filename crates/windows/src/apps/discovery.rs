use winsp_core::models::{AppItem, AppTarget};

use super::builtin::built_in_tools;
use super::com::ComGuard;
use super::resolve_shortcut_target;
use super::start_menu::start_menu_dirs;

pub fn enumerate_installed_apps() -> Vec<AppItem> {
    let mut apps = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    scan_start_menu(&start_menu_dirs(), &mut apps, &mut seen_ids);

    for item in built_in_tools() {
        if seen_ids.insert(item.id.clone()) {
            apps.push(item);
        }
    }

    apps
}

fn scan_start_menu(
    dirs: &[std::path::PathBuf],
    apps: &mut Vec<AppItem>,
    seen_ids: &mut std::collections::HashSet<String>,
) {
    let Some(_com_guard) = ComGuard::new() else {
        return;
    };

    for dir_path in dirs {
        scan_directory_for_shortcuts(dir_path, apps, seen_ids);
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    struct ConflictingApartmentGuard;

    impl Drop for ConflictingApartmentGuard {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(Some(0))
            .collect()
    }

    fn create_test_lnk(dir: &std::path::Path, name: &str, target: &str, args: &str) {
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
        assert_eq!(apps[0].name.as_ref(), "Chrome");
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

    #[test]
    fn keeps_urls_with_different_casing_distinct() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        fs::write(
            per_user.path().join("App.url"),
            "[InternetShortcut]\nURL=https://example.com/App\n",
        )
        .unwrap();
        fs::write(
            system_wide.path().join("App.url"),
            "[InternetShortcut]\nURL=https://example.com/app\n",
        )
        .unwrap();

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_directory_for_shortcuts(per_user.path(), &mut apps, &mut seen_ids);
        scan_directory_for_shortcuts(system_wide.path(), &mut apps, &mut seen_ids);

        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn skips_shortcut_scan_when_com_apartment_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _guard = ComGuard::new();
            create_test_lnk(
                dir.path(),
                "Chrome",
                r"C:\Program Files\Chrome\chrome.exe",
                "",
            );
        }

        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
        }
        let _cleanup = ConflictingApartmentGuard;

        let mut apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        scan_start_menu(&[dir.path().to_path_buf()], &mut apps, &mut seen_ids);

        assert!(apps.is_empty());
    }
}
