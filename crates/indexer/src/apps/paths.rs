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
