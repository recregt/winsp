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

    let reindex_callback = {
        let state = Arc::clone(&state);
        move || {
            let index = populate_search_index();
            println!("Reindex triggered: {} items", index.len());
            if let Ok(mut app_state) = state.lock() {
                app_state.index = index;
                app_state.refresh_results();
            }
        }
    };

    let _reindex_watcher = if let Some(dir) = test_watch_dir() {
        println!("Test mode: watching {} for changes", dir.display());
        winsp_windows::catalog::sources::watcher::for_dirs(&[dir], reindex_callback).ok()
    } else {
        #[cfg(windows)]
        {
            winsp_windows::catalog::sources::watcher::for_start_menu(reindex_callback).ok()
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
