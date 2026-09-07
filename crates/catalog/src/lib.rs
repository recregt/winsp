#![cfg(windows)]
#![forbid(unsafe_code)]

mod apps;
mod builtins;
mod settings;
mod shortcut;
mod watcher;

use std::collections::HashSet;
use std::path::PathBuf;

use builtins::Builtins;
use settings::Settings;
use winsp_core::models::AppItem;

use apps::Apps;

pub use watcher::{CatalogCallback, notify_reconcile_channel_broken, start_watching};

pub(crate) struct Sources {
    apps: Apps,
    settings: Settings,
    builtins: Builtins,
}

impl Sources {
    pub(crate) fn scan() -> Self {
        Self {
            apps: Apps::scan(),
            settings: Settings,
            builtins: Builtins,
        }
    }

    pub(crate) fn items(&self) -> Vec<AppItem> {
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

    pub(crate) fn watch_dirs(&self) -> &[PathBuf] {
        self.apps.watch_dirs()
    }

    pub(crate) fn apply_changes(&mut self, changed_paths: &[PathBuf]) {
        self.apps.apply_changes(changed_paths);
    }

    pub(crate) fn rescan(&mut self) {
        self.apps.rescan();
    }

    pub(crate) fn unreadable_dirs(&self) -> &[PathBuf] {
        self.apps.unreadable_dirs()
    }
}
