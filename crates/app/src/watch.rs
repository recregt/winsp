#![cfg(windows)]

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use winsp_windows::catalog::sources::apps::StartMenuCatalog;
use winsp_windows::catalog::sources::watcher::WatchEvent;

use crate::index::engine_from_catalog;
use crate::state::AppState;

const RECONCILE_INTERVAL: Duration = Duration::from_secs(600);
const MIN_RECONCILE_GAP: Duration = Duration::from_secs(30);

pub(crate) enum StartupMode {
    TestWatch(std::path::PathBuf),
    Real(StartMenuCatalog),
}

pub(crate) fn startup_mode() -> StartupMode {
    match crate::test_watch_dir() {
        Some(dir) => StartupMode::TestWatch(dir),
        None => {
            let catalog = StartMenuCatalog::for_start_menu();
            notify_if_scan_incomplete(&catalog);
            StartupMode::Real(catalog)
        }
    }
}

pub(crate) fn notify_if_scan_incomplete(catalog: &StartMenuCatalog) {
    static NOTIFIED: std::sync::Once = std::sync::Once::new();
    if !catalog.unreadable_dirs().is_empty() {
        NOTIFIED.call_once(|| {
            winsp_windows::system::toast::show(
                "WinSP",
                "Some Start Menu folders couldn't be scanned. Results may be incomplete.",
            );
        });
    }
}

pub(crate) fn refresh_state(state: &Arc<Mutex<AppState>>, catalog: &StartMenuCatalog) {
    let index = engine_from_catalog(catalog);
    if let Ok(mut app_state) = state.lock() {
        app_state.index = index;
        app_state.refresh_results();
    }
}

pub(crate) fn spawn_reconciler(
    state: Arc<Mutex<AppState>>,
    catalog: Arc<Mutex<StartMenuCatalog>>,
) -> Sender<()> {
    let (reconcile_tx, reconcile_rx) = std::sync::mpsc::channel::<()>();

    std::thread::spawn(move || {
        let mut last_rescan = Instant::now();
        loop {
            let _ = reconcile_rx.recv_timeout(RECONCILE_INTERVAL);
            while reconcile_rx.try_recv().is_ok() {}

            if last_rescan.elapsed() < MIN_RECONCILE_GAP {
                continue;
            }

            if let Ok(mut cat) = catalog.lock() {
                cat.rescan();
                notify_if_scan_incomplete(&cat);
                refresh_state(&state, &cat);
            }
            last_rescan = Instant::now();
        }
    });

    reconcile_tx
}

pub(crate) fn handle_watch_event(
    event: WatchEvent,
    state: &Arc<Mutex<AppState>>,
    catalog: &Arc<Mutex<StartMenuCatalog>>,
    reconcile_tx: &Sender<()>,
) {
    match event {
        WatchEvent::Changed(paths) => {
            if let Ok(mut cat) = catalog.lock() {
                cat.apply_changes(&paths);
                notify_if_scan_incomplete(&cat);
                refresh_state(state, &cat);
            }
        }
        WatchEvent::Uncertain => {
            let _ = reconcile_tx.send(());
        }
    }
}
