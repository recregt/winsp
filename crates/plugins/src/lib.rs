#![cfg(windows)]
#![forbid(unsafe_code)]

mod apps;
mod builtins;
mod settings;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use apps::ScannedShortcut;
use winsp_core::models::AppItem;

pub use apps::start_menu_dirs;

pub struct Catalog {
    dirs: Vec<PathBuf>,
    shortcuts: HashMap<PathBuf, ScannedShortcut>,
    unreadable_dirs: Vec<PathBuf>,
    builtins: Vec<AppItem>,
    settings: Vec<AppItem>,
}

impl Catalog {
    pub fn scan() -> Self {
        let (dirs, shortcuts, unreadable_dirs) = Self::scan_shortcuts(apps::start_menu_dirs());
        Self {
            dirs,
            shortcuts,
            unreadable_dirs,
            builtins: builtins::built_in_tools(),
            settings: settings::list_settings(),
        }
    }

    pub fn items(&self) -> Vec<AppItem> {
        let mut seen_ids = HashSet::new();
        let mut apps = Vec::new();

        for item in self
            .shortcut_items()
            .into_iter()
            .chain(self.builtins.iter().cloned())
            .chain(self.settings.iter().cloned())
        {
            if seen_ids.insert(item.id().to_string()) {
                apps.push(item);
            }
        }

        apps
    }
}
