use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const DEFAULT_HOTKEY_VK: u16 = 0x20; // VK_SPACE

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Settings {
    #[serde(default = "default_hotkey_vk")]
    pub(super) hotkey_vk: u16,
}

fn default_hotkey_vk() -> u16 {
    DEFAULT_HOTKEY_VK
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey_vk: DEFAULT_HOTKEY_VK,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|dir| PathBuf::from(dir).join("WinSP").join("settings.msgpack"))
}

impl Settings {
    pub(super) fn load() -> Self {
        match config_path() {
            Some(path) => Self::load_from(&path),
            None => Self::default(),
        }
    }

    fn load_from(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| rmp_serde::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub(super) fn save(&self) -> io::Result<()> {
        let path = config_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is not set"))?;
        self.save_to(&path)
    }

    fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = rmp_serde::to_vec_named(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, &bytes)?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_a_missing_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");

        assert_eq!(Settings::load_from(&path), Settings::default());
    }

    #[test]
    fn load_from_corrupt_data_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        std::fs::write(&path, b"not valid msgpack").unwrap();

        assert_eq!(Settings::load_from(&path), Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.msgpack");
        let settings = Settings { hotkey_vk: 0x41 };

        settings.save_to(&path).unwrap();

        assert_eq!(Settings::load_from(&path), settings);
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");

        Settings::default().save_to(&path).unwrap();

        assert!(!path.with_extension("tmp").exists());
        assert!(path.exists());
    }
}
