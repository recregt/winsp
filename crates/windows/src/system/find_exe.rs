use std::path::PathBuf;

use windows::Win32::Storage::FileSystem::SearchPathW;
use windows::core::HSTRING;

pub fn find_exe(name: &str) -> Option<PathBuf> {
    let name = HSTRING::from(name);
    let mut buffer = vec![0u16; 260];

    let len = unsafe { SearchPathW(None, &name, None, Some(&mut buffer), None) };

    (len != 0 && (len as usize) < buffer.len())
        .then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_exe_returns_none_for_a_name_that_does_not_exist_on_path() {
        assert_eq!(find_exe("definitely-not-a-real-executable.exe"), None);
    }

    #[test]
    fn find_exe_finds_a_known_system_binary() {
        assert!(find_exe("notepad.exe").is_some());
    }
}
