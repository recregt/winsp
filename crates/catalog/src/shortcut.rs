pub(crate) fn resolve_target(path: &std::path::Path, ext_lower: &str) -> Option<String> {
    match ext_lower {
        "lnk" => resolve_lnk_target(path),
        "url" => resolve_url_target(path),
        _ => None,
    }
}

fn resolve_lnk_target(path: &std::path::Path) -> Option<String> {
    let shell_link =
        lnk::ShellLink::open(path, winsp_windows::system::codepage::system_encoding()).ok()?;
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
    let bytes = std::fs::read(path).ok()?;
    let contents = winsp_windows::system::codepage::decode(&bytes);
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("URL") {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
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

    #[test]
    fn resolves_url_target_from_a_legacy_code_page_encoded_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("App.url");
        let text = "[InternetShortcut]\nURL=https://example.com/café\n";
        let (encoded, _, had_unmappable) =
            winsp_windows::system::codepage::system_encoding().encode(text);
        assert!(
            !had_unmappable,
            "the system code page should be able to represent this test string"
        );
        std::fs::write(&path, &encoded).unwrap();

        assert_eq!(
            resolve_url_target(&path),
            Some("https://example.com/café".to_string())
        );
    }

    #[test]
    fn resolves_url_target_from_a_utf8_bom_encoded_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("App.url");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"[InternetShortcut]\nURL=https://example.com/app\n");
        std::fs::write(&path, &bytes).unwrap();

        assert_eq!(
            resolve_url_target(&path),
            Some("https://example.com/app".to_string())
        );
    }
}
