use winsp_core::models::{AppItem, AppTarget};

pub(super) fn built_in_tools() -> Vec<AppItem> {
    vec![
        AppItem::new(
            "builtin:calc.exe",
            "Calculator",
            AppTarget::Path("calc.exe".into()),
        )
        .with_description("Microsoft Calculator"),
        AppItem::new(
            "builtin:notepad.exe",
            "Notepad",
            AppTarget::Path("notepad.exe".into()),
        )
        .with_description("Fast text editor"),
        AppItem::new(
            "builtin:wt.exe",
            "Windows Terminal",
            AppTarget::Path("wt.exe".into()),
        )
        .with_description("Modern terminal console"),
        AppItem::new(
            "builtin:cmd.exe",
            "Command Prompt",
            AppTarget::Path("cmd.exe".into()),
        )
        .with_description("Windows command interpreter"),
        AppItem::new(
            "builtin:powershell.exe",
            "PowerShell",
            AppTarget::Path("powershell.exe".into()),
        )
        .with_description("PowerShell scripting environment"),
        AppItem::new(
            "builtin:mspaint.exe",
            "Paint",
            AppTarget::Path("mspaint.exe".into()),
        )
        .with_description("Bitmap image editor"),
        AppItem::new(
            "builtin:snippingtool.exe",
            "Snipping Tool",
            AppTarget::Path("snippingtool.exe".into()),
        )
        .with_description("Screen capture tool"),
        AppItem::new(
            "builtin:regedit.exe",
            "Registry Editor",
            AppTarget::Path("regedit.exe".into()),
        )
        .with_description("Windows registry management"),
        AppItem::new(
            "builtin:control.exe",
            "Control Panel",
            AppTarget::Path("control.exe".into()),
        )
        .with_description("Legacy system control panel"),
        AppItem::new(
            "builtin:explorer.exe",
            "File Explorer",
            AppTarget::Path("explorer.exe".into()),
        )
        .with_description("File management"),
    ]
}
