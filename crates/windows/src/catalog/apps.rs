use winsp_core::models::{AppItem, LaunchTarget};

pub(super) fn built_in_tools() -> Vec<AppItem> {
    vec![
        AppItem::new(
            "builtin:calc.exe",
            "Calculator",
            LaunchTarget::Path("calc.exe".into()),
        )
        .with_description("Microsoft Calculator"),
        AppItem::new(
            "builtin:notepad.exe",
            "Notepad",
            LaunchTarget::Path("notepad.exe".into()),
        )
        .with_description("Fast text editor"),
        AppItem::new(
            "builtin:wt.exe",
            "Windows Terminal",
            LaunchTarget::Path("wt.exe".into()),
        )
        .with_description("Modern terminal console"),
        AppItem::new(
            "builtin:cmd.exe",
            "Command Prompt",
            LaunchTarget::Path("cmd.exe".into()),
        )
        .with_description("Windows command interpreter"),
        AppItem::new(
            "builtin:powershell.exe",
            "PowerShell",
            LaunchTarget::Path("powershell.exe".into()),
        )
        .with_description("PowerShell scripting environment"),
        AppItem::new(
            "builtin:mspaint.exe",
            "Paint",
            LaunchTarget::Path("mspaint.exe".into()),
        )
        .with_description("Bitmap image editor"),
        AppItem::new(
            "builtin:snippingtool.exe",
            "Snipping Tool",
            LaunchTarget::Path("snippingtool.exe".into()),
        )
        .with_description("Screen capture tool"),
        AppItem::new(
            "builtin:regedit.exe",
            "Registry Editor",
            LaunchTarget::Path("regedit.exe".into()),
        )
        .with_description("Windows registry management"),
        AppItem::new(
            "builtin:control.exe",
            "Control Panel",
            LaunchTarget::Path("control.exe".into()),
        )
        .with_description("Legacy system control panel"),
        AppItem::new(
            "builtin:explorer.exe",
            "File Explorer",
            LaunchTarget::Path("explorer.exe".into()),
        )
        .with_description("File management"),
    ]
    .into_iter()
    .map(|item| match item.target() {
        LaunchTarget::Path(exe) => match resolve_system_exe(exe) {
            Some(icon) => item.with_icon(icon),
            None => item,
        },
        _ => item,
    })
    .collect()
}

pub(crate) fn resolve_system_exe(name: &str) -> Option<String> {
    use windows::Win32::Storage::FileSystem::SearchPathW;
    use windows::core::HSTRING;

    let name = HSTRING::from(name);
    let mut buffer = vec![0u16; 260];

    let len = unsafe { SearchPathW(None, &name, None, Some(&mut buffer), None) };

    (len != 0 && (len as usize) < buffer.len())
        .then(|| String::from_utf16_lossy(&buffer[..len as usize]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use winsp_core::models::IconSource;

    #[test]
    fn resolve_system_exe_returns_none_for_a_name_that_does_not_exist_on_path() {
        assert_eq!(
            resolve_system_exe("definitely-not-a-real-executable.exe"),
            None
        );
    }

    #[test]
    fn every_resolved_builtin_tool_carries_a_path_icon_matching_its_target() {
        for item in built_in_tools() {
            let LaunchTarget::Path(exe) = item.target() else {
                continue;
            };
            if let Some(resolved) = resolve_system_exe(exe) {
                assert_eq!(item.icon(), Some(&IconSource::Path(resolved)));
            }
        }
    }
}
