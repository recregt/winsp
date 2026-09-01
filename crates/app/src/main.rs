#![cfg_attr(windows, windows_subsystem = "windows")]

mod state;
mod window;

use state::AppState;
#[cfg(not(windows))]
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use winsp_core::models::AppItem;
use winsp_core::search::Engine;

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn populate_search_index() -> Engine {
    let mut index = Engine::new();
    let mut all_items: Vec<AppItem> = Vec::new();

    all_items.extend(winsp_windows::catalog::sources::apps::list_installed_apps());
    all_items.extend(winsp_windows::catalog::sources::settings::list_settings());

    index.set_items(all_items);
    index
}

fn test_watch_dir() -> Option<std::path::PathBuf> {
    std::env::var("WINSP_TEST_WATCH_DIR")
        .ok()
        .map(std::path::PathBuf::from)
}

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        use winsp_windows::catalog::sources::apps::StartMenuCatalog;
        use winsp_windows::catalog::sources::watcher::WatchEvent;

        const RECONCILE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(600);
        const MIN_RECONCILE_GAP: std::time::Duration = std::time::Duration::from_secs(30);

        fn engine_from_catalog(catalog: &StartMenuCatalog) -> Engine {
            let mut index = Engine::new();
            let mut all_items: Vec<AppItem> =
                winsp_windows::catalog::sources::apps::merge_with_built_ins(catalog.items());
            all_items.extend(winsp_windows::catalog::sources::settings::list_settings());

            index.set_items(all_items);
            index
        }

        fn notify_if_scan_incomplete(catalog: &StartMenuCatalog) {
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

        fn refresh_state(state: &Arc<Mutex<AppState>>, catalog: &StartMenuCatalog) {
            let index = engine_from_catalog(catalog);
            if let Ok(mut app_state) = state.lock() {
                app_state.index = index;
                app_state.refresh_results();
            }
        }

        fn build_initial_catalog(test_watch_mode: bool) -> Option<StartMenuCatalog> {
            if test_watch_mode {
                return None;
            }
            let catalog = StartMenuCatalog::for_start_menu();
            notify_if_scan_incomplete(&catalog);
            Some(catalog)
        }

        fn spawn_reconciler(
            state: Arc<Mutex<AppState>>,
            catalog: Arc<Mutex<StartMenuCatalog>>,
        ) -> std::sync::mpsc::Sender<()> {
            let (reconcile_tx, reconcile_rx) = std::sync::mpsc::channel::<()>();

            std::thread::spawn(move || {
                let mut last_rescan = std::time::Instant::now();
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
                    last_rescan = std::time::Instant::now();
                }
            });

            reconcile_tx
        }

        fn handle_watch_event(
            event: WatchEvent,
            state: &Arc<Mutex<AppState>>,
            catalog: &Arc<Mutex<StartMenuCatalog>>,
            reconcile_tx: &std::sync::mpsc::Sender<()>,
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
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
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

    let watch_dir = test_watch_dir();

    let start_init = std::time::Instant::now();
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            let initial_catalog = build_initial_catalog(watch_dir.is_some());
            let index = match &initial_catalog {
                Some(catalog) => engine_from_catalog(catalog),
                None => populate_search_index(),
            };
        } else {
            let index = populate_search_index();
        }
    }
    let init_duration = start_init.elapsed();
    println!(
        "Indexed {} applications & settings in {:.2?}",
        index.len(),
        init_duration
    );

    let state = Arc::new(Mutex::new(AppState::new(index)));

    let _reindex_watcher = if let Some(dir) = watch_dir {
        println!("Test mode: watching {} for changes", dir.display());
        let state = Arc::clone(&state);
        winsp_windows::catalog::sources::watcher::for_dirs(&[dir], move |_event| {
            let index = populate_search_index();
            println!("Reindex triggered: {} items", index.len());
            if let Ok(mut app_state) = state.lock() {
                app_state.index = index;
                app_state.refresh_results();
            }
        })
        .ok()
    } else {
        cfg_if::cfg_if! {
            if #[cfg(windows)] {
                let catalog = Arc::new(Mutex::new(
                    initial_catalog.expect("built above when not running in test-watch mode"),
                ));
                let reconcile_tx = spawn_reconciler(Arc::clone(&state), Arc::clone(&catalog));
                window::set_reconcile_hook(reconcile_tx.clone());

                let state = Arc::clone(&state);
                winsp_windows::catalog::sources::watcher::for_start_menu(move |event| {
                    handle_watch_event(event, &state, &catalog, &reconcile_tx);
                })
                .ok()
            } else {
                None
            }
        }
    };

    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            println!("Press Alt+Space to toggle the search bar, Esc to dismiss.");
            window::run_app(state).map_err(|e| e.into())
        } else {
            run_terminal_demo(state)
        }
    }
}

#[cfg(not(windows))]
fn run_terminal_demo(state: Arc<Mutex<AppState>>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Cross-platform interactive demo mode.");
    println!("Type a query to search, '=expr' for math, or ':q' to quit.");

    loop {
        print!("Spotlight > ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break;
        }

        let query = input.trim();
        if query == ":q" || query == "exit" {
            break;
        }

        let start_search = std::time::Instant::now();
        let mut app_state = state.lock().unwrap();
        app_state.set_query(query.to_string());
        let search_duration = start_search.elapsed();

        println!(
            "Found {} in {:.3?}",
            app_state.results.len(),
            search_duration
        );

        if app_state.results.is_empty() {
            println!("  No matching applications or calculations found.");
        } else {
            for (i, res) in app_state.results.iter().enumerate() {
                let sub = res.subtitle.as_deref().unwrap_or("");
                println!("  [{}] {:<25} | {}", i + 1, res.title, sub);
            }
        }
        println!();
    }

    Ok(())
}
