use std::os::windows::ffi::OsStrExt;

use lnk::encoding::WINDOWS_1252;
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::core::PCWSTR;

pub(super) fn resolve_lnk_target(path: &std::path::Path) -> Option<String> {
    let shell_link = lnk::ShellLink::open(path, WINDOWS_1252).ok()?;

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
    let wide: Vec<u16> = std::ffi::OsStr::new(raw)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut expanded = [0u16; 260];
    let written = unsafe { ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), Some(&mut expanded)) };
    if written == 0 || written as usize > expanded.len() {
        raw.to_string()
    } else {
        let end = expanded
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(expanded.len());
        String::from_utf16_lossy(&expanded[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_environment_variables_in_lnk_target() {
        let program_files = std::env::var("ProgramFiles").unwrap();

        let expanded = expand_env_vars(r"%ProgramFiles%\App\app.exe");

        assert_eq!(
            expanded.to_lowercase(),
            format!(r"{program_files}\App\app.exe").to_lowercase()
        );
    }
}
