#![cfg_attr(windows, windows_subsystem = "windows")]

mod index;
mod state;
mod watch;
mod window;

use state::AppState;
use std::sync::{Arc, Mutex};

pub(crate) fn test_watch_dir() -> Option<std::path::PathBuf> {
    std::env::var("WINSP_TEST_WATCH_DIR")
        .ok()
        .map(std::path::PathBuf::from)
}

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _instance_mutex = match winsp_windows::system::single_instance::acquire(
        "WinSP_SingleInstance_Mutex",
        window::WINDOW_CLASS_NAME,
    ) {
        Some(mutex) => mutex,
        None => return Ok(()),
    };

    let mode = watch::startup_mode();
    let index = match &mode {
        watch::StartupMode::Real(catalog) => index::engine_from_catalog(catalog),
        watch::StartupMode::TestWatch(_) => index::populate_search_index(),
    };

    let state = Arc::new(Mutex::new(AppState::new(index)));

    let _reindex_watcher = match mode {
        watch::StartupMode::TestWatch(dir) => {
            let state = Arc::clone(&state);
            winsp_windows::catalog::sources::watcher::for_dirs(&[dir], move |_event| {
                let index = index::populate_search_index();
                if let Ok(mut app_state) = state.lock() {
                    app_state.index = index;
                    app_state.refresh_results();
                }
            })
            .ok()
        }
        watch::StartupMode::Real(catalog) => {
            let catalog = Arc::new(Mutex::new(catalog));
            let reconcile_tx = watch::spawn_reconciler(Arc::clone(&state), Arc::clone(&catalog));
            window::set_reconcile_hook(reconcile_tx.clone());

            let state = Arc::clone(&state);
            winsp_windows::catalog::sources::watcher::for_start_menu(move |event| {
                watch::handle_watch_event(event, &state, &catalog, &reconcile_tx);
            })
            .ok()
        }
    };

    window::run_app(state).map_err(|e| e.into())
}
