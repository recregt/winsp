#![cfg(windows)]
#![forbid(unsafe_code)]

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};

use winsp_core::index::{Index, Match};
use winsp_core::models::{AppItem, IconSource, LaunchTarget, SearchResult, SearchResultKind};
use winsp_windows::system::watcher::Watcher;
use winsp_windows::window::{Anchor, Key, Modifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum RowIcon {
    Path(String),
    Glyph(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultRow {
    pub title: Arc<str>,
    pub subtitle: Option<Arc<str>>,
    pub matched_char_indices: Vec<usize>,
    pub icon: Option<RowIcon>,
}

fn to_row(result: &SearchResult) -> ResultRow {
    let icon = match &result.kind {
        SearchResultKind::App(item) => item.icon().map(|icon| match icon {
            IconSource::Path(path) => RowIcon::Path(path.clone()),
            IconSource::Glyph(glyph) => RowIcon::Glyph(*glyph),
        }),
        _ => None,
    };
    ResultRow {
        title: Arc::clone(&result.title),
        subtitle: result.subtitle.clone(),
        matched_char_indices: result.matched_char_indices.clone(),
        icon,
    }
}

fn launch(target: LaunchTarget) {
    let submitted = winsp_windows::system::threadpool::spawn_on_threadpool(move || {
        let result = match target {
            LaunchTarget::Path(path) => winsp_windows::shell::open_path(&path),
            LaunchTarget::WebUrl(uri) | LaunchTarget::OsUri(uri) => {
                winsp_windows::shell::open_uri(&uri)
            }
            LaunchTarget::Command(cmd) => std::process::Command::new("cmd")
                .args(["/C", &cmd])
                .spawn()
                .map(|_| ())
                .map_err(|err| err.to_string()),
        };
        if let Err(error) = result {
            winsp_windows::system::toast::show("WinSP", &error);
        }
    });
    if !submitted {
        winsp_windows::system::toast::show(
            "WinSP",
            "Failed to launch: the system thread pool rejected the task.",
        );
    }
}

struct Inner {
    index: Index<AppItem>,
    matches: Vec<Match<AppItem>>,
    results: Vec<SearchResult>,
}

pub struct Service {
    inner: Mutex<Inner>,
    reconcile_tx: OnceLock<Sender<()>>,
    watcher: OnceLock<Option<Watcher>>,
}

impl Default for Service {
    fn default() -> Self {
        Self::new(Index::new())
    }
}

impl Service {
    pub fn new(index: Index<AppItem>) -> Self {
        Self {
            inner: Mutex::new(Inner {
                index,
                matches: Vec::new(),
                results: Vec::new(),
            }),
            reconcile_tx: OnceLock::new(),
            watcher: OnceLock::new(),
        }
    }

    pub fn start(on_index_changed: impl Fn() + Send + Sync + 'static) -> Arc<Self> {
        if !winsp_config::exists() {
            if let Err(err) = winsp_config::Settings::load().save() {
                eprintln!("failed to save settings: {err}");
            }
        }

        let service = Arc::new(Self::default());

        let on_catalog_updated = {
            let service = Arc::clone(&service);
            Arc::new(move |index| {
                service.replace_index(index);
                on_index_changed();
            })
        };
        let (watcher, reconcile_tx) = winsp_catalog::start_watching(on_catalog_updated);
        let _ = service.reconcile_tx.set(reconcile_tx);
        let _ = service.watcher.set(watcher);

        service
    }

    pub fn current_hotkey(&self) -> (Modifiers, Key) {
        winsp_config::to_hotkey_combo(winsp_config::Settings::load().hotkey)
    }

    pub fn current_position(&self) -> Anchor {
        winsp_config::to_anchor(winsp_config::Settings::load().position)
    }

    pub fn change_hotkey(&self, modifiers: Modifiers, key: Key) -> Result<(), String> {
        winsp_config::on_hotkey_changed(modifiers, key)
    }

    pub fn change_position(&self, anchor: Anchor) -> Result<(), String> {
        winsp_config::on_position_changed(anchor)
    }

    pub fn on_window_shown(&self) {
        if let Some(tx) = self.reconcile_tx.get()
            && tx.send(()).is_err()
        {
            winsp_catalog::notify_reconcile_channel_broken();
        }
    }

    fn replace_index(&self, index: Index<AppItem>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.index = index;
        }
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<ResultRow> {
        let Ok(mut inner) = self.inner.lock() else {
            return Vec::new();
        };
        let Inner {
            index,
            matches,
            results,
        } = &mut *inner;
        winsp_search::query(index, query, max_results, matches, results);
        results.iter().map(to_row).collect()
    }

    pub fn activate(&self, index: usize) {
        let Ok(inner) = self.inner.lock() else {
            return;
        };
        let Some(result) = inner.results.get(index) else {
            return;
        };
        match &result.kind {
            SearchResultKind::Calculation { result: value, .. } => {
                let value = value.to_string();
                drop(inner);
                winsp_windows::system::clipboard::copy(&value);
                winsp_windows::system::toast::show("WinSP", &format!("Copied: {value}"));
            }
            SearchResultKind::App(item) => {
                let target = item.target().clone();
                drop(inner);
                launch(target);
            }
            SearchResultKind::WebSearch { url, .. } => {
                let target = LaunchTarget::WebUrl(url.clone());
                drop(inner);
                launch(target);
            }
            SearchResultKind::SystemCommand { command, .. } => {
                let target = LaunchTarget::Command(command.clone());
                drop(inner);
                launch(target);
            }
        }
    }
}

#[cfg(feature = "test-support")]
pub mod testing {
    use super::*;

    pub fn empty_service() -> Service {
        Service::new(Index::new())
    }

    pub fn service_with_apps(apps: &[(&str, &str)]) -> Service {
        let mut index = Index::new();
        index.set_items(
            apps.iter().map(|(id, title)| {
                AppItem::new(*id, *title, LaunchTarget::Path(format!("{id}.exe")))
            }),
        );
        Service::new(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_returns_no_rows_for_an_empty_index() {
        let service = Service::new(Index::new());
        assert!(service.search("anything", 5).is_empty());
    }

    #[test]
    fn search_finds_a_matching_app_and_carries_its_display_fields() {
        let mut index = Index::new();
        index.set_items(vec![AppItem::new(
            "calc",
            "Calculator",
            LaunchTarget::Path("calc.exe".into()),
        )]);
        let service = Service::new(index);

        let rows = service.search("calc", 5);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title.as_ref(), "Calculator");
    }

    #[test]
    fn a_calculation_query_produces_a_row_with_no_icon() {
        let service = Service::new(Index::new());

        let rows = service.search("12 * 12", 5);

        assert_eq!(rows.len(), 1);
        assert!(rows[0].icon.is_none());
    }

    #[test]
    fn replace_index_changes_what_search_finds() {
        let service = Service::new(Index::new());
        assert!(service.search("calc", 5).is_empty());

        let mut index = Index::new();
        index.set_items(vec![AppItem::new(
            "calc",
            "Calculator",
            LaunchTarget::Path("calc.exe".into()),
        )]);
        service.replace_index(index);

        assert_eq!(service.search("calc", 5).len(), 1);
    }

    #[test]
    fn activating_an_out_of_range_index_does_not_panic() {
        let service = Service::new(Index::new());
        service.search("", 5);
        service.activate(0);
    }

    #[test]
    fn on_window_shown_is_a_no_op_when_the_service_was_never_started() {
        let service = Service::new(Index::new());
        service.on_window_shown();
    }
}
