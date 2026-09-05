#![cfg(windows)]

use std::sync::mpsc::{RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use winsp_core::engine::Engine;
use winsp_windows::catalog::Catalog;
use winsp_windows::system::watcher::{WatchEvent, Watcher};

const RECONCILE_INTERVAL: Duration = Duration::from_secs(600);
const MIN_RECONCILE_GAP: Duration = Duration::from_secs(30);

pub(crate) fn engine_from_catalog(catalog: &Catalog) -> Engine {
    let mut index = Engine::new();
    index.set_items(catalog.items());
    index
}

pub(crate) fn scan_catalog() -> Catalog {
    let catalog = Catalog::scan();
    notify_if_scan_incomplete(&catalog);
    catalog
}

fn notify_if_scan_incomplete(catalog: &Catalog) {
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

fn notify_watcher_init_failed() {
    winsp_windows::system::toast::show(
        "WinSP",
        "Couldn't watch the Start Menu for changes. New or removed shortcuts won't appear until WinSP restarts.",
    );
}

fn notify_watch_dirs_failed() {
    winsp_windows::system::toast::show(
        "WinSP",
        "Some folders couldn't be watched for changes. New apps there may not appear until WinSP restarts.",
    );
}

fn finish_watcher<E>(result: Result<(Watcher, Vec<std::path::PathBuf>), E>) -> Option<Watcher> {
    match result {
        Ok((watcher, failed_dirs)) => {
            if !failed_dirs.is_empty() {
                notify_watch_dirs_failed();
            }
            Some(watcher)
        }
        Err(_) => {
            notify_watcher_init_failed();
            None
        }
    }
}

pub(crate) fn start_watching(catalog: Catalog) -> (Option<Watcher>, Sender<()>) {
    let catalog = Arc::new(Mutex::new(catalog));
    let tx = spawn_reconciler(Arc::clone(&catalog));
    let reconcile_tx = tx.clone();

    let watcher = winsp_windows::system::watcher::for_start_menu(move |event| {
        handle_watch_event(event, &catalog, &tx);
    });
    (finish_watcher(watcher), reconcile_tx)
}

fn refresh_state(catalog: &Catalog) {
    crate::ui::deliver_catalog(engine_from_catalog(catalog));
}

fn next_wait(pending: bool, last_rescan: Instant) -> Duration {
    if pending {
        MIN_RECONCILE_GAP.saturating_sub(last_rescan.elapsed())
    } else {
        RECONCILE_INTERVAL
    }
}

fn spawn_reconciler(catalog: Arc<Mutex<Catalog>>) -> Sender<()> {
    let (reconcile_tx, reconcile_rx) = std::sync::mpsc::channel::<()>();

    std::thread::spawn(move || {
        let mut last_rescan = Instant::now();
        let mut pending = false;
        loop {
            let wait = next_wait(pending, last_rescan);

            match reconcile_rx.recv_timeout(wait) {
                Ok(()) => pending = true,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            loop {
                match reconcile_rx.try_recv() {
                    Ok(()) => pending = true,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }

            if pending && last_rescan.elapsed() < MIN_RECONCILE_GAP {
                continue;
            }

            if let Ok(mut cat) = catalog.lock() {
                cat.rescan();
                notify_if_scan_incomplete(&cat);
                refresh_state(&cat);
            }
            last_rescan = Instant::now();
            pending = false;
        }
    });

    reconcile_tx
}

fn handle_watch_event(event: WatchEvent, catalog: &Arc<Mutex<Catalog>>, reconcile_tx: &Sender<()>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_wait_uses_full_interval_when_nothing_pending() {
        assert_eq!(next_wait(false, Instant::now()), RECONCILE_INTERVAL);
    }

    #[test]
    fn next_wait_uses_the_remaining_cooldown_when_pending() {
        let last_rescan = Instant::now() - Duration::from_secs(10);
        let wait = next_wait(true, last_rescan);
        assert!(wait <= MIN_RECONCILE_GAP);
        assert!(wait > Duration::ZERO);
    }

    #[test]
    fn next_wait_is_zero_once_the_cooldown_has_already_elapsed() {
        let last_rescan = Instant::now() - MIN_RECONCILE_GAP - Duration::from_secs(1);
        assert_eq!(next_wait(true, last_rescan), Duration::ZERO);
    }
}
