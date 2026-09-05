#![cfg(windows)]

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const DEFAULT_VK: u16 = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HotkeyBinding {
    #[serde(default)]
    pub(crate) ctrl: bool,
    #[serde(default)]
    pub(crate) shift: bool,
    #[serde(default = "default_alt")]
    pub(crate) alt: bool,
    #[serde(default)]
    pub(crate) win: bool,
    #[serde(default = "default_vk")]
    pub(crate) vk: u16,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum WindowPosition {
    #[default]
    Top,
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct Settings {
    #[serde(default)]
    pub(crate) hotkey: HotkeyBinding,
    #[serde(default)]
    pub(crate) position: WindowPosition,
}

#[cfg(test)]
thread_local! {
    static TEST_CONFIG_DIR: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) struct TestConfigDirGuard;

#[cfg(test)]
impl TestConfigDirGuard {
    pub(crate) fn set(dir: PathBuf) -> Self {
        TEST_CONFIG_DIR.with(|cell| *cell.borrow_mut() = Some(dir));
        Self
    }
}

#[cfg(test)]
impl Drop for TestConfigDirGuard {
    fn drop(&mut self) {
        TEST_CONFIG_DIR.with(|cell| *cell.borrow_mut() = None);
    }
}

fn config_path() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(dir) = TEST_CONFIG_DIR.with(|cell| cell.borrow().clone()) {
        return Some(dir.join("WinSP").join("settings.msgpack"));
    }

    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|dir| PathBuf::from(dir).join("WinSP").join("settings.msgpack"))
}

pub(crate) fn exists() -> bool {
    config_path().is_some_and(|path| path.exists())
}

impl Settings {
    pub(crate) fn load() -> Self {
        match config_path() {
            Some(path) => Self::load_from(&path),
            None => Self::default(),
        }
    }

    pub(crate) fn load_from(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| rmp_serde::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        let path = config_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is not set"))?;
        self.save_to(&path)
    }

    pub(crate) fn save_to(&self, path: &Path) -> io::Result<()> {
        let parent = match path.parent() {
            Some(parent) => {
                std::fs::create_dir_all(parent)?;
                parent
            }
            None => Path::new("."),
        };
        let bytes = rmp_serde::to_vec_named(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let mut tmp = tempfile::Builder::new()
            .prefix("settings.")
            .suffix(".tmp")
            .tempfile_in(parent)?;
        tmp.write_all(&bytes)?;
        persist_with_retries(tmp, path)
    }
}

const PERSIST_RETRIES: u32 = 5;
const PERSIST_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(5);

fn persist_with_retries(mut tmp: tempfile::NamedTempFile, path: &Path) -> io::Result<()> {
    let mut retries_left = PERSIST_RETRIES;
    loop {
        match tmp.persist(path) {
            Ok(_) => return Ok(()),
            Err(err) if retries_left > 0 && err.error.kind() == io::ErrorKind::PermissionDenied => {
                tmp = err.file;
                retries_left -= 1;
                std::thread::sleep(PERSIST_RETRY_DELAY);
            }
            Err(err) => return Err(err.error),
        }
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
            position: WindowPosition::Center,
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
            position: WindowPosition::Top,
        }
        .save_to(&path)
        .unwrap();

        let replacement = Settings {
            hotkey: HotkeyBinding {
                vk: 0x42,
                ..Default::default()
            },
            position: WindowPosition::Center,
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

        let leftover: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name())
            .filter(|name| *name != "settings.msgpack")
            .collect();
        assert!(
            leftover.is_empty(),
            "unexpected leftover files: {leftover:?}"
        );
        assert!(path.exists());
    }

    #[test]
    fn concurrent_saves_to_the_same_path_never_fail_or_corrupt_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");

        let a = Settings {
            hotkey: HotkeyBinding {
                vk: 0x41,
                ..Default::default()
            },
            position: WindowPosition::Top,
        };
        let b = Settings {
            hotkey: HotkeyBinding {
                vk: 0x42,
                ..Default::default()
            },
            position: WindowPosition::Center,
        };

        let (path_a, path_b) = (path.clone(), path.clone());
        let writer_a = std::thread::spawn(move || {
            for _ in 0..50 {
                a.save_to(&path_a).unwrap();
            }
            a
        });
        let writer_b = std::thread::spawn(move || {
            for _ in 0..50 {
                b.save_to(&path_b).unwrap();
            }
            b
        });

        let a = writer_a.join().unwrap();
        let b = writer_b.join().unwrap();

        let loaded = Settings::load_from(&path);
        assert!(
            loaded == a || loaded == b,
            "concurrent saves must not corrupt the file into anything other than one of the written values"
        );
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

    #[test]
    fn the_default_position_is_top() {
        assert_eq!(WindowPosition::default(), WindowPosition::Top);
    }

    #[test]
    fn a_file_with_no_position_key_defaults_to_top() {
        #[derive(Serialize)]
        struct HotkeyOnlySettings {
            hotkey: HotkeyBinding,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        let bytes = rmp_serde::to_vec_named(&HotkeyOnlySettings {
            hotkey: HotkeyBinding::default(),
        })
        .unwrap();
        std::fs::write(&path, bytes).unwrap();

        assert_eq!(Settings::load_from(&path).position, WindowPosition::Top);
    }
}
