#![cfg_attr(windows, windows_subsystem = "windows")]

mod catalog_sync;
mod config;
mod index;
mod state;
mod window;

use state::AppState;
use std::sync::{Arc, Mutex};

pub(crate) fn test_watch_dir() -> Option<std::path::PathBuf> {
    #[cfg(debug_assertions)]
    {
        std::env::var("WINSP_TEST_WATCH_DIR")
            .ok()
            .map(std::path::PathBuf::from)
    }
    #[cfg(not(debug_assertions))]
    {
        None
    }
}

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(_instance_mutex) = winsp_windows::system::single_instance::acquire(
        "WinSP_SingleInstance_Mutex",
        window::WINDOW_CLASS_NAME,
    ) else {
        return Ok(());
    };

    let mode = catalog_sync::startup_mode();
    let index = match &mode {
        catalog_sync::StartupMode::Real(catalog) => index::engine_from_catalog(catalog),
        catalog_sync::StartupMode::TestWatch(_) => index::populate_search_index(),
    };

    let state = Arc::new(Mutex::new(AppState::new(index)));

    let _reindex_watcher = match mode {
        catalog_sync::StartupMode::TestWatch(dir) => {
            let state = Arc::clone(&state);
            let watcher = winsp_windows::system::watcher::for_dirs(&[dir], move |_event| {
                let index = index::populate_search_index();
                if let Ok(mut app_state) = state.lock() {
                    app_state.index = index;
                    app_state.refresh_results();
                }
            });
            if watcher.is_err() {
                catalog_sync::notify_watcher_init_failed();
            }
            watcher.ok()
        }
        catalog_sync::StartupMode::Real(catalog) => {
            let catalog = Arc::new(Mutex::new(catalog));
            let reconcile_tx = catalog_sync::spawn_reconciler(Arc::clone(&catalog));
            window::set_reconcile_hook(reconcile_tx.clone());

            let watcher = winsp_windows::system::watcher::for_start_menu(move |event| {
                catalog_sync::handle_watch_event(event, &catalog, &reconcile_tx);
            });
            if watcher.is_err() {
                catalog_sync::notify_watcher_init_failed();
            }
            watcher.ok()
        }
    };

    window::run_app(state).map_err(|e| e.into())
}
