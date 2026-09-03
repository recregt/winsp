#![cfg(windows)]

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use winsp_core::search::Engine;
use winsp_windows::catalog::sources::apps::StartMenuCatalog;
use winsp_windows::catalog::sources::watcher::WatchEvent;

use crate::index::engine_from_catalog;

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

pub(crate) fn notify_reconcile_channel_broken() {
    static NOTIFIED: std::sync::Once = std::sync::Once::new();
    NOTIFIED.call_once(|| {
        winsp_windows::system::toast::show(
            "WinSP",
            "Background reindexing stopped responding. Restart WinSP to restore it.",
        );
    });
}

fn pending_index() -> &'static Mutex<Option<Engine>> {
    static PENDING_INDEX: OnceLock<Mutex<Option<Engine>>> = OnceLock::new();
    PENDING_INDEX.get_or_init(|| Mutex::new(None))
}

pub(crate) fn take_pending_index() -> Option<Engine> {
    pending_index().lock().ok().and_then(|mut slot| slot.take())
}

pub(crate) fn refresh_state(catalog: &StartMenuCatalog) {
    let index = engine_from_catalog(catalog);
    if let Ok(mut slot) = pending_index().lock() {
        *slot = Some(index);
    }
    winsp_windows::window::notify_catalog_ready();
}

pub(crate) fn spawn_reconciler(catalog: Arc<Mutex<StartMenuCatalog>>) -> Sender<()> {
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
                refresh_state(&cat);
            }
            last_rescan = Instant::now();
        }
    });

    reconcile_tx
}

pub(crate) fn handle_watch_event(
    event: WatchEvent,
    catalog: &Arc<Mutex<StartMenuCatalog>>,
    reconcile_tx: &Sender<()>,
) {
    match event {
        WatchEvent::Changed(paths) => {
            if let Ok(mut cat) = catalog.lock() {
                cat.apply_changes(&paths);
                notify_if_scan_incomplete(&cat);
                refresh_state(&cat);
            }
        }
        WatchEvent::Uncertain => {
            if reconcile_tx.send(()).is_err() {
                notify_reconcile_channel_broken();
            }
        }
    }
}
