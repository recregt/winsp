use std::path::PathBuf;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_CommonPrograms, FOLDERID_Programs, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
};
use windows::core::GUID;

/// The current user's Start Menu `Programs` folder, resolved via the shell
/// rather than assumed from `%APPDATA%` so it still works when the folder is
/// redirected (roaming profiles, corporate policy).
pub fn user_start_menu_programs() -> Option<PathBuf> {
    known_folder_path(&FOLDERID_Programs)
}

/// The all-users Start Menu `Programs` folder, resolved via the shell rather
/// than assumed from `%ProgramData%`.
pub fn common_start_menu_programs() -> Option<PathBuf> {
    known_folder_path(&FOLDERID_CommonPrograms)
}

fn known_folder_path(folder_id: &GUID) -> Option<PathBuf> {
    unsafe {
        let pwstr = SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None).ok()?;
        let path = pwstr.to_string().ok();
        CoTaskMemFree(Some(pwstr.0 as *const _));
        path.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_start_menu_programs_resolves_to_an_existing_directory() {
        let path = user_start_menu_programs().expect("the shell should resolve this known folder");
        assert!(path.is_dir(), "{path:?} should exist and be a directory");
    }

    #[test]
    fn common_start_menu_programs_resolves_to_an_existing_directory() {
        let path =
            common_start_menu_programs().expect("the shell should resolve this known folder");
        assert!(path.is_dir(), "{path:?} should exist and be a directory");
    }
}
