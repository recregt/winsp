use std::sync::mpsc::{RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use winsp_core::models::AppItem;
use winsp_index::Index;
use winsp_windows::system::watcher::{WatchEvent, Watcher};

use crate::Sources;

type Catalog = Index<AppItem>;
pub type CatalogCallback = Arc<dyn Fn(Catalog) + Send + Sync>;

const RECONCILE_INTERVAL: Duration = Duration::from_secs(600);
const MIN_RECONCILE_GAP: Duration = Duration::from_secs(30);

fn engine_from_sources(sources: &Sources) -> Catalog {
    let mut index = Catalog::new();
    index.set_items(sources.items());
    index
}

fn scan_sources() -> Sources {
    let sources = Sources::scan();
    notify_if_scan_incomplete(&sources);
    sources
}

fn notify_if_scan_incomplete(sources: &Sources) {
    static NOTIFIED: std::sync::Once = std::sync::Once::new();
    if !sources.unreadable_dirs().is_empty() {
        NOTIFIED.call_once(|| {
            winsp_windows::system::toast::show(
                "WinSP",
                "Some Start Menu folders couldn't be scanned. Results may be incomplete.",
            );
        });
    }
}

pub fn notify_reconcile_channel_broken() {
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

pub fn start_watching(on_catalog_updated: CatalogCallback) -> (Option<Watcher>, Sender<()>) {
    let sources = scan_sources();
    on_catalog_updated(engine_from_sources(&sources));

    let dirs = sources.watch_dirs().to_vec();
    let sources = Arc::new(Mutex::new(sources));
    let tx = spawn_reconciler(Arc::clone(&sources), Arc::clone(&on_catalog_updated));
    let reconcile_tx = tx.clone();

    let watcher = winsp_windows::system::watcher::for_dirs(&dirs, move |event| {
        handle_watch_event(event, &sources, &tx, &on_catalog_updated);
    });
    (finish_watcher(watcher), reconcile_tx)
}

fn refresh_state(sources: &Sources, on_catalog_updated: &CatalogCallback) {
    on_catalog_updated(engine_from_sources(sources));
}

fn next_wait(pending: bool, last_rescan: Instant) -> Duration {
    if pending {
        MIN_RECONCILE_GAP.saturating_sub(last_rescan.elapsed())
    } else {
        RECONCILE_INTERVAL
    }
}

fn spawn_reconciler(
    sources: Arc<Mutex<Sources>>,
    on_catalog_updated: CatalogCallback,
) -> Sender<()> {
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

            if let Ok(mut cat) = sources.lock() {
                cat.rescan();
                notify_if_scan_incomplete(&cat);
                refresh_state(&cat, &on_catalog_updated);
            }
            last_rescan = Instant::now();
            pending = false;
        }
    });

    reconcile_tx
}

fn handle_watch_event(
    event: WatchEvent,
    sources: &Arc<Mutex<Sources>>,
    reconcile_tx: &Sender<()>,
    on_catalog_updated: &CatalogCallback,
) {
    match event {
        WatchEvent::Changed(paths) => {
            if let Ok(mut cat) = sources.lock() {
                cat.apply_changes(&paths);
                notify_if_scan_incomplete(&cat);
                refresh_state(&cat, on_catalog_updated);
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
