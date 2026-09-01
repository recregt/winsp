use std::collections::HashSet;
use std::path::{Path, PathBuf};

use winsp_core::models::{AppItem, AppTarget};

use super::resolve_shortcut_target;
use super::start_menu::start_menu_dirs;

struct ScannedShortcut {
    item: AppItem,
    priority: usize,
}

pub struct StartMenuCatalog {
    dirs: Vec<PathBuf>,
    shortcuts: std::collections::HashMap<PathBuf, ScannedShortcut>,
}

impl StartMenuCatalog {
    pub fn for_start_menu() -> Self {
        Self::scan(start_menu_dirs())
    }

    pub fn scan(dirs: Vec<PathBuf>) -> Self {
        let mut shortcuts = std::collections::HashMap::new();
        for (priority, dir) in dirs.iter().enumerate() {
            collect_shortcuts(dir, priority, &mut shortcuts);
        }
        Self { dirs, shortcuts }
    }

    pub fn rescan(&mut self) {
        *self = Self::scan(std::mem::take(&mut self.dirs));
    }

    pub fn apply_changes(&mut self, changed_paths: &[PathBuf]) {
        for path in changed_paths {
            self.apply_change(path);
        }
    }

    fn apply_change(&mut self, path: &Path) {
        self.shortcuts.retain(|p, _| !p.starts_with(path));

        if !path.exists() {
            return;
        }

        let priority = self
            .dirs
            .iter()
            .position(|dir| path.starts_with(dir))
            .unwrap_or(usize::MAX);

        if path.is_dir() {
            collect_shortcuts(path, priority, &mut self.shortcuts);
        } else {
            insert_if_shortcut(path, priority, &mut self.shortcuts);
        }
    }

    pub fn items(&self) -> Vec<AppItem> {
        let mut ordered: Vec<(&PathBuf, &ScannedShortcut)> = self.shortcuts.iter().collect();
        ordered.sort_by(|(path_a, a), (path_b, b)| {
            a.priority.cmp(&b.priority).then_with(|| path_a.cmp(path_b))
        });

        let mut seen_ids = HashSet::new();
        let mut items = Vec::new();
        for (_, scanned) in ordered {
            if seen_ids.insert(scanned.item.id.clone()) {
                items.push(scanned.item.clone());
            }
        }
        items
    }
}

fn collect_shortcuts(
    dir: &Path,
    priority: usize,
    shortcuts: &mut std::collections::HashMap<PathBuf, ScannedShortcut>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_shortcuts(&path, priority, shortcuts);
            } else {
                insert_if_shortcut(&path, priority, shortcuts);
            }
        }
    }
}

fn insert_if_shortcut(
    path: &Path,
    priority: usize,
    shortcuts: &mut std::collections::HashMap<PathBuf, ScannedShortcut>,
) {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return;
    };
    let ext_lower = ext.to_lowercase();
    if ext_lower != "lnk" && ext_lower != "url" {
        return;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let stem_lower = stem.to_lowercase();

    let resolved = resolve_shortcut_target(path, &ext_lower);

    if ext_lower == "lnk" {
        if let Some(identity) = &resolved {
            if let Some((target, arguments)) = identity.split_once('|') {
                if targets_uninstaller(target, arguments) {
                    return;
                }
            }
        }
    } else if matches!(stem_lower.as_str(), "uninstall" | "remove") {
        return;
    }

    let identity = resolved.unwrap_or_else(|| path.to_string_lossy().into_owned());
    let id = format!("shortcut:{identity}");
    let item = AppItem::new(id, stem, AppTarget::Path(path.to_string_lossy().into()))
        .with_description(path.to_string_lossy().to_string());

    shortcuts.insert(path.to_path_buf(), ScannedShortcut { item, priority });
}

const KNOWN_UNINSTALLER_EXE_NAMES: &[&str] = &[
    "uninstall.exe",
    "uninstaller.exe",
    "unwise.exe",
    "unwise32.exe",
];
const NUMBERED_UNINSTALLER_PREFIXES: &[&str] = &["unins", "uninst"];

fn targets_uninstaller(target: &str, arguments: &str) -> bool {
    let exe_name = Path::new(target)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    if is_uninstaller_exe_name(exe_name) {
        return true;
    }

    exe_name == "msiexec.exe"
        && arguments
            .split_whitespace()
            .any(|arg| arg == "/x" || arg == "/uninstall")
}

fn is_uninstaller_exe_name(exe_name: &str) -> bool {
    if KNOWN_UNINSTALLER_EXE_NAMES.contains(&exe_name) {
        return true;
    }

    let Some(stem) = exe_name.strip_suffix(".exe") else {
        return false;
    };

    NUMBERED_UNINSTALLER_PREFIXES.iter().any(|prefix| {
        stem.strip_prefix(prefix)
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    struct ComInit;

    impl ComInit {
        fn new() -> Self {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
            }
            Self
        }
    }

    impl Drop for ComInit {
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

    fn create_test_lnk(dir: &Path, name: &str, target: &str, args: &str) {
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

    fn names(items: &[AppItem]) -> Vec<&str> {
        items.iter().map(|i| i.name.as_ref()).collect()
    }

    #[test]
    fn dedupes_real_lnk_shortcuts_with_same_target() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
            create_test_lnk(per_user.path(), "App", r"C:\Apps\App\app.exe", "");
            create_test_lnk(system_wide.path(), "App", r"C:\Apps\App\app.exe", "");
        }

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 1);
    }

    #[test]
    fn keeps_real_lnk_shortcuts_with_different_targets_same_name() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
            create_test_lnk(per_user.path(), "App", r"C:\VendorA\app.exe", "");
            create_test_lnk(system_wide.path(), "App", r"C:\VendorB\app.exe", "");
        }

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 2);
    }

    #[test]
    fn keeps_real_lnk_shortcuts_with_same_target_different_arguments() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
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
        }

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 2);
    }

    #[test]
    fn keeps_unresolvable_shortcuts_with_the_same_stem_as_distinct_entries() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        fs::write(per_user.path().join("Chrome.lnk"), []).unwrap();
        fs::write(system_wide.path().join("Chrome.lnk"), []).unwrap();

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);
        let items = catalog.items();

        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.name.as_ref() == "Chrome"));
    }

    #[test]
    fn keeps_distinctly_named_shortcuts_across_directories() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        fs::write(per_user.path().join("Chrome.lnk"), []).unwrap();
        fs::write(system_wide.path().join("Firefox.lnk"), []).unwrap();

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 2);
    }

    #[test]
    fn dedupes_same_named_shortcut_with_same_resolved_target() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        let contents = "[InternetShortcut]\nURL=https://example.com/app\n";
        fs::write(per_user.path().join("App.url"), contents).unwrap();
        fs::write(system_wide.path().join("App.url"), contents).unwrap();

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 1);
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

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 2);
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

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 1);
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

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);

        assert_eq!(catalog.items().len(), 2);
    }

    #[test]
    fn per_user_wins_dedup_tie_over_system_wide() {
        let per_user = tempfile::tempdir().unwrap();
        let system_wide = tempfile::tempdir().unwrap();

        let contents = "[InternetShortcut]\nURL=https://example.com/app\n";
        fs::write(system_wide.path().join("App.url"), contents).unwrap();
        fs::write(per_user.path().join("App.url"), contents).unwrap();

        let catalog =
            StartMenuCatalog::scan(vec![per_user.path().into(), system_wide.path().into()]);
        let items = catalog.items();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].target,
            AppTarget::Path(per_user.path().join("App.url").to_string_lossy().into())
        );
    }

    #[test]
    fn apply_changes_picks_up_a_newly_created_shortcut() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = StartMenuCatalog::scan(vec![dir.path().into()]);
        assert_eq!(catalog.items().len(), 0);

        let contents = "[InternetShortcut]\nURL=https://example.com/app\n";
        let new_shortcut = dir.path().join("App.url");
        fs::write(&new_shortcut, contents).unwrap();

        catalog.apply_changes(&[new_shortcut]);

        assert_eq!(names(&catalog.items()), vec!["App"]);
    }

    #[test]
    fn apply_changes_removes_a_deleted_shortcut() {
        let dir = tempfile::tempdir().unwrap();
        let contents = "[InternetShortcut]\nURL=https://example.com/app\n";
        let shortcut = dir.path().join("App.url");
        fs::write(&shortcut, contents).unwrap();

        let mut catalog = StartMenuCatalog::scan(vec![dir.path().into()]);
        assert_eq!(catalog.items().len(), 1);

        fs::remove_file(&shortcut).unwrap();
        catalog.apply_changes(&[shortcut]);

        assert_eq!(catalog.items().len(), 0);
    }

    #[test]
    fn apply_changes_walks_a_newly_created_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = StartMenuCatalog::scan(vec![dir.path().into()]);
        assert_eq!(catalog.items().len(), 0);

        let subfolder = dir.path().join("VendorX");
        fs::create_dir(&subfolder).unwrap();
        let contents = "[InternetShortcut]\nURL=https://example.com/one\n";
        fs::write(subfolder.join("One.url"), contents).unwrap();
        let contents = "[InternetShortcut]\nURL=https://example.com/two\n";
        fs::write(subfolder.join("Two.url"), contents).unwrap();

        catalog.apply_changes(&[subfolder]);

        let scanned = catalog.items();
        let mut items = names(&scanned);
        items.sort_unstable();
        assert_eq!(items, vec!["One", "Two"]);
    }

    #[test]
    fn apply_changes_removes_everything_under_a_deleted_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        let subfolder = dir.path().join("VendorX");
        fs::create_dir(&subfolder).unwrap();
        let contents = "[InternetShortcut]\nURL=https://example.com/one\n";
        fs::write(subfolder.join("One.url"), contents).unwrap();

        let mut catalog = StartMenuCatalog::scan(vec![dir.path().into()]);
        assert_eq!(catalog.items().len(), 1);

        fs::remove_dir_all(&subfolder).unwrap();
        catalog.apply_changes(&[subfolder]);

        assert_eq!(catalog.items().len(), 0);
    }

    #[test]
    fn rescan_reflects_changes_made_since_the_last_scan() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = StartMenuCatalog::scan(vec![dir.path().into()]);
        assert_eq!(catalog.items().len(), 0);

        let contents = "[InternetShortcut]\nURL=https://example.com/app\n";
        fs::write(dir.path().join("App.url"), contents).unwrap();

        catalog.rescan();

        assert_eq!(names(&catalog.items()), vec!["App"]);
    }

    #[test]
    fn filters_out_shortcuts_targeting_a_known_uninstaller_exe() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
            create_test_lnk(dir.path(), "Remove Foo", r"C:\Apps\Foo\unins000.exe", "");
        }

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        assert_eq!(catalog.items().len(), 0);
    }

    #[test]
    fn filters_out_msiexec_uninstall_shortcuts() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
            create_test_lnk(
                dir.path(),
                "Modify or Remove Foo",
                r"C:\Windows\System32\msiexec.exe",
                "/x {12345678-1234-1234-1234-123456789012}",
            );
        }

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        assert_eq!(catalog.items().len(), 0);
    }

    #[test]
    fn does_not_filter_msiexec_shortcuts_with_unrelated_flags() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
            create_test_lnk(
                dir.path(),
                "Configure Foo",
                r"C:\Windows\System32\msiexec.exe",
                "/xmlconfig /import settings.xml",
            );
        }

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        assert_eq!(catalog.items().len(), 1);
    }

    #[test]
    fn does_not_filter_shortcuts_targeting_similarly_named_but_different_exes() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
            create_test_lnk(dir.path(), "Installer", r"C:\Apps\Foo\installer.exe", "");
        }

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        assert_eq!(catalog.items().len(), 1);
    }

    #[test]
    fn keeps_real_apps_whose_name_merely_contains_uninstall_or_help() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _com = ComInit::new();
            create_test_lnk(
                dir.path(),
                "Help Scout",
                r"C:\Apps\HelpScout\helpscout.exe",
                "",
            );
            create_test_lnk(
                dir.path(),
                "Uninstall Manager",
                r"C:\Apps\UninstallManager\app.exe",
                "",
            );
        }

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        let scanned = catalog.items();
        let mut items = names(&scanned);
        items.sort_unstable();
        assert_eq!(items, vec!["Help Scout", "Uninstall Manager"]);
    }

    #[test]
    fn keeps_unresolvable_lnk_even_if_named_like_an_uninstaller() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Uninstall Foo.lnk"), []).unwrap();

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        assert_eq!(names(&catalog.items()), vec!["Uninstall Foo"]);
    }

    #[test]
    fn keeps_url_shortcut_named_help() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Help.url"),
            "[InternetShortcut]\nURL=https://example.com/help\n",
        )
        .unwrap();

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        assert_eq!(names(&catalog.items()), vec!["Help"]);
    }

    #[test]
    fn filters_out_url_shortcuts_named_exactly_uninstall_or_remove() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Uninstall.url"),
            "[InternetShortcut]\nURL=https://example.com/uninstall\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("Remove.url"),
            "[InternetShortcut]\nURL=https://example.com/remove\n",
        )
        .unwrap();

        let catalog = StartMenuCatalog::scan(vec![dir.path().into()]);

        assert_eq!(catalog.items().len(), 0);
    }
}
