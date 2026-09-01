use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const DEFAULT_VK: u16 = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct HotkeyBinding {
    #[serde(default)]
    pub(super) ctrl: bool,
    #[serde(default)]
    pub(super) shift: bool,
    #[serde(default = "default_alt")]
    pub(super) alt: bool,
    #[serde(default)]
    pub(super) win: bool,
    #[serde(default = "default_vk")]
    pub(super) vk: u16,
}

fn default_alt() -> bool {
    true
}

fn default_vk() -> u16 {
    DEFAULT_VK
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: default_alt(),
            win: false,
            vk: default_vk(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(super) struct Settings {
    #[serde(default)]
    pub(super) hotkey: HotkeyBinding,
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
        let settings = Settings {
            hotkey: HotkeyBinding {
                ctrl: true,
                shift: false,
                alt: true,
                win: false,
                vk: 0x41,
            },
        };

        settings.save_to(&path).unwrap();

        assert_eq!(Settings::load_from(&path), settings);
    }

    #[test]
    fn save_to_replaces_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        Settings {
            hotkey: HotkeyBinding {
                vk: 0x41,
                ..Default::default()
            },
        }
        .save_to(&path)
        .unwrap();

        let replacement = Settings {
            hotkey: HotkeyBinding {
                vk: 0x42,
                ..Default::default()
            },
        };
        replacement.save_to(&path).unwrap();

        assert_eq!(Settings::load_from(&path), replacement);
    }

    #[test]
    fn corrupt_settings_are_repaired_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        std::fs::write(&path, b"not valid msgpack").unwrap();

        let settings = Settings::load_from(&path);
        assert_eq!(settings, Settings::default());

        settings.save_to(&path).unwrap();

        assert_eq!(Settings::load_from(&path), Settings::default());
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");

        Settings::default().save_to(&path).unwrap();

        assert!(!path.with_extension("tmp").exists());
        assert!(path.exists());
    }

    #[test]
    fn the_default_binding_is_alt_space() {
        assert_eq!(
            HotkeyBinding::default(),
            HotkeyBinding {
                ctrl: false,
                shift: false,
                alt: true,
                win: false,
                vk: 0x20,
            }
        );
    }

    #[test]
    fn an_empty_map_with_no_hotkey_key_defaults_to_alt_space() {
        #[derive(Serialize)]
        struct Empty {}

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        let bytes = rmp_serde::to_vec_named(&Empty {}).unwrap();
        std::fs::write(&path, bytes).unwrap();

        assert_eq!(Settings::load_from(&path), Settings::default());
    }

    #[test]
    fn a_136_shaped_file_is_treated_as_having_no_hotkey_configured() {
        #[derive(Serialize)]
        struct LegacySettings {
            hotkey_vk: u16,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        let bytes = rmp_serde::to_vec_named(&LegacySettings { hotkey_vk: 0x20 }).unwrap();
        std::fs::write(&path, bytes).unwrap();

        assert_eq!(Settings::load_from(&path), Settings::default());
    }
}
