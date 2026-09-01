#![cfg_attr(windows, windows_subsystem = "windows")]

mod demo;
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

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        #[global_allocator]
        static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

        fn main() -> Result<(), Box<dyn std::error::Error>> {
            let _instance_mutex = match winsp_windows::system::single_instance::acquire(
                "WinSP_SingleInstance_Mutex",
                window::WINDOW_CLASS_NAME,
            ) {
                Some(mutex) => mutex,
                None => {
                    println!("WinSP is already running, focusing the existing window.");
                    return Ok(());
                }
            };

            let start_init = std::time::Instant::now();
            let mode = watch::startup_mode();
            let index = match &mode {
                watch::StartupMode::Real(catalog) => index::engine_from_catalog(catalog),
                watch::StartupMode::TestWatch(_) => index::populate_search_index(),
            };
            let init_duration = start_init.elapsed();
            println!(
                "Indexed {} applications & settings in {:.2?}",
                index.len(),
                init_duration
            );

            let state = Arc::new(Mutex::new(AppState::new(index)));

            let _reindex_watcher = match mode {
                watch::StartupMode::TestWatch(dir) => {
                    println!("Test mode: watching {} for changes", dir.display());
                    let state = Arc::clone(&state);
                    winsp_windows::catalog::sources::watcher::for_dirs(&[dir], move |_event| {
                        let index = index::populate_search_index();
                        println!("Reindex triggered: {} items", index.len());
                        if let Ok(mut app_state) = state.lock() {
                            app_state.index = index;
                            app_state.refresh_results();
                        }
                    })
                    .ok()
                }
                watch::StartupMode::Real(catalog) => {
                    let catalog = Arc::new(Mutex::new(catalog));
                    let reconcile_tx =
                        watch::spawn_reconciler(Arc::clone(&state), Arc::clone(&catalog));
                    window::set_reconcile_hook(reconcile_tx.clone());

                    let state = Arc::clone(&state);
                    winsp_windows::catalog::sources::watcher::for_start_menu(move |event| {
                        watch::handle_watch_event(event, &state, &catalog, &reconcile_tx);
                    })
                    .ok()
                }
            };

            println!("Press Alt+Space to toggle the search bar, Esc to dismiss.");
            window::run_app(state).map_err(|e| e.into())
        }
    } else {
        fn main() -> Result<(), Box<dyn std::error::Error>> {
            let index = index::populate_search_index();
            let state = Arc::new(Mutex::new(AppState::new(index)));

            let _reindex_watcher = test_watch_dir().and_then(|dir| {
                println!("Test mode: watching {} for changes", dir.display());
                let state = Arc::clone(&state);
                winsp_windows::catalog::sources::watcher::for_dirs(&[dir], move |_event| {
                    let index = index::populate_search_index();
                    println!("Reindex triggered: {} items", index.len());
                    if let Ok(mut app_state) = state.lock() {
                        app_state.index = index;
                        app_state.refresh_results();
                    }
                })
                .ok()
            });

            demo::run_terminal_demo(state)
        }
    }
}
