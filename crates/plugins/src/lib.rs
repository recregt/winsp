#![cfg(windows)]
#![forbid(unsafe_code)]

mod apps;
mod builtins;
mod settings;

use std::collections::HashSet;
use std::path::PathBuf;

use winsp_core::models::AppItem;

pub use apps::{Apps, start_menu_dirs};

pub struct Plugins {
    pub apps: Apps,
}

impl Plugins {
    pub fn scan() -> Self {
        Self { apps: Apps::scan() }
    }

    pub fn items(&self) -> Vec<AppItem> {
        let mut seen_ids = HashSet::new();
        let mut items = Vec::new();

        for item in self
            .apps
            .items()
            .into_iter()
            .chain(builtins::items())
            .chain(settings::items())
        {
            if seen_ids.insert(item.id().to_string()) {
                items.push(item);
            }
        }

        items
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
