use lnk::encoding::WINDOWS_1252;

pub fn resolve_target(path: &std::path::Path, ext_lower: &str) -> Option<String> {
    match ext_lower {
        "lnk" => resolve_lnk_target(path),
        "url" => resolve_url_target(path),
        _ => None,
    }
}

fn resolve_lnk_target(path: &std::path::Path) -> Option<String> {
    let shell_link = lnk::ShellLink::open(path, WINDOWS_1252).ok()?;
    identity_from_shell_link(&shell_link)
}

fn identity_from_shell_link(shell_link: &lnk::ShellLink) -> Option<String> {
    let raw_target = shell_link
        .link_info()
        .as_ref()
        .and_then(|info| {
            let full_path = format!(
                "{}{}",
                info.local_base_path().unwrap_or_default(),
                info.common_path_suffix()
            );
            (!full_path.is_empty()).then_some(full_path)
        })
        .or_else(|| shell_link.string_data().relative_path().clone())?;

    let target = expand_env_vars(&raw_target);
    if target.is_empty() {
        return None;
    }

    let arguments = shell_link
        .string_data()
        .command_line_arguments()
        .clone()
        .unwrap_or_default();

    Some(format!(
        "{}|{}",
        target.to_lowercase(),
        arguments.to_lowercase()
    ))
}

fn expand_env_vars(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut rest = raw;

    while let Some(start) = rest.find('%') {
        let (before, after_percent) = rest.split_at(start);
        result.push_str(before);
        let after_percent = &after_percent[1..];

        match after_percent.find('%') {
            Some(end) => {
                let name = &after_percent[..end];
                match std::env::var(name) {
                    Ok(value) => result.push_str(&value),
                    Err(_) => {
                        result.push('%');
                        result.push_str(name);
                        result.push('%');
                    }
                }
                rest = &after_percent[end + 1..];
            }
            None => {
                result.push('%');
                rest = after_percent;
            }
        }
    }

    result.push_str(rest);
    result
}

fn resolve_url_target(path: &std::path::Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("URL") {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

#[cfg(feature = "test-support")]
pub mod testing {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    pub struct ComGuard;

    impl ComGuard {
        pub fn new() -> Self {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
            }
            Self
        }
    }

    impl Default for ComGuard {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub fn create_test_lnk(dir: &Path, name: &str, target: &str, args: &str) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_environment_variables_in_lnk_target() {
        let path = std::env::var("PATH").unwrap();

        let expanded = expand_env_vars(r"%PATH%\App\app.exe");

        assert_eq!(expanded, format!(r"{path}\App\app.exe"));
    }

    #[test]
    fn leaves_unknown_variables_untouched() {
        let expanded = expand_env_vars(r"%DefinitelyNotARealVariable%\App\app.exe");

        assert_eq!(expanded, r"%DefinitelyNotARealVariable%\App\app.exe");
    }

    #[test]
    fn falls_back_to_none_when_lnk_carries_no_resolvable_target() {
        let shell_link = lnk::ShellLink::default();

        assert_eq!(identity_from_shell_link(&shell_link), None);
    }
}
