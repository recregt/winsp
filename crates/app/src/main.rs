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

#[cfg(windows)]
const RECONCILE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(600);
#[cfg(windows)]
const MIN_RECONCILE_GAP: std::time::Duration = std::time::Duration::from_secs(30);

#[cfg(windows)]
fn engine_from_catalog(
    catalog: &winsp_windows::catalog::sources::apps::StartMenuCatalog,
) -> Engine {
    let mut index = Engine::new();
    let mut all_items: Vec<AppItem> =
        winsp_windows::catalog::sources::apps::merge_with_built_ins(catalog.items());
    all_items.extend(winsp_windows::catalog::sources::settings::list_settings());

    index.set_items(all_items);
    index
}

#[cfg(windows)]
fn notify_if_scan_incomplete(catalog: &winsp_windows::catalog::sources::apps::StartMenuCatalog) {
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

    let start_init = std::time::Instant::now();
    let index = populate_search_index();
    let init_duration = start_init.elapsed();
    println!(
        "Indexed {} applications & settings in {:.2?}",
        index.len(),
        init_duration
    );

    let state = Arc::new(Mutex::new(AppState::new(index)));

    let _reindex_watcher = if let Some(dir) = test_watch_dir() {
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
        #[cfg(windows)]
        {
            use std::sync::mpsc;
            use std::time::Instant;
            use winsp_windows::catalog::sources::apps::StartMenuCatalog;
            use winsp_windows::catalog::sources::watcher::WatchEvent;

            let initial_catalog = StartMenuCatalog::for_start_menu();
            notify_if_scan_incomplete(&initial_catalog);
            let catalog = Arc::new(Mutex::new(initial_catalog));
            let (reconcile_tx, reconcile_rx) = mpsc::channel::<()>();

            {
                let state = Arc::clone(&state);
                let catalog = Arc::clone(&catalog);
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
                            let index = engine_from_catalog(&cat);
                            if let Ok(mut app_state) = state.lock() {
                                app_state.index = index;
                                app_state.refresh_results();
                            }
                        }
                        last_rescan = Instant::now();
                    }
                });
            }

            window::set_reconcile_hook(reconcile_tx.clone());

            let state = Arc::clone(&state);
            winsp_windows::catalog::sources::watcher::for_start_menu(move |event| match event {
                WatchEvent::Changed(paths) => {
                    if let Ok(mut cat) = catalog.lock() {
                        cat.apply_changes(&paths);
                        notify_if_scan_incomplete(&cat);
                        let index = engine_from_catalog(&cat);
                        if let Ok(mut app_state) = state.lock() {
                            app_state.index = index;
                            app_state.refresh_results();
                        }
                    }
                }
                WatchEvent::Uncertain => {
                    let _ = reconcile_tx.send(());
                }
            })
            .ok()
        }
        #[cfg(not(windows))]
        {
            None
        }
    };

    #[cfg(windows)]
    {
        println!("Press Alt+Space to toggle the search bar, Esc to dismiss.");
        window::run_app(state).map_err(|e| e.into())
    }

    #[cfg(not(windows))]
    {
        run_terminal_demo(state)
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
