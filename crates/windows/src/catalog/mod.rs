mod apps;
mod settings;
mod start_menu;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use start_menu::ScannedShortcut;
use winsp_core::models::AppItem;

pub struct Catalog {
    dirs: Vec<PathBuf>,
    shortcuts: HashMap<PathBuf, ScannedShortcut>,
    unreadable_dirs: Vec<PathBuf>,
    builtins: Vec<AppItem>,
    settings: Vec<AppItem>,
}

impl Catalog {
    pub fn scan() -> Self {
        let (dirs, shortcuts, unreadable_dirs) =
            Self::scan_shortcuts(start_menu::start_menu_dirs());
        Self {
            dirs,
            shortcuts,
            unreadable_dirs,
            builtins: apps::built_in_tools(),
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
            if seen_ids.insert(item.id.clone()) {
                apps.push(item);
            }
        }

        apps
    }
}

pub(crate) use start_menu::start_menu_dirs;
