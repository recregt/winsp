#![cfg(windows)]

mod apps;
mod builtins;
mod settings;

use std::collections::HashSet;
use std::path::PathBuf;

use builtins::Builtins;
use settings::Settings;
use winsp_core::models::AppItem;

pub use apps::Apps;

pub struct Sources {
    pub apps: Apps,
    settings: Settings,
    builtins: Builtins,
}

impl Sources {
    pub fn scan() -> Self {
        Self {
            apps: Apps::scan(),
            settings: Settings,
            builtins: Builtins,
        }
    }

    pub fn items(&self) -> Vec<AppItem> {
        let mut seen_ids = HashSet::new();
        let mut items = Vec::new();

        for item in self
            .apps
            .items()
            .into_iter()
            .chain(self.builtins.items())
            .chain(self.settings.items())
        {
            if seen_ids.insert(item.id().to_string()) {
                items.push(item);
            }
        }

        items
    }

    pub fn watch_dirs(&self) -> &[PathBuf] {
        self.apps.watch_dirs()
    }

    pub fn apply_changes(&mut self, changed_paths: &[PathBuf]) {
        self.apps.apply_changes(changed_paths);
    }

    pub fn rescan(&mut self) {
        self.apps.rescan();
    }

    pub fn unreadable_dirs(&self) -> &[PathBuf] {
        self.apps.unreadable_dirs()
    }
}
