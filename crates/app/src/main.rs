#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;
mod state;
mod sync;
mod ui;

use state::AppState;
use std::sync::{Arc, Mutex};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use winsp_windows::system::single_instance::AcquireResult;

    let _instance_mutex = match winsp_windows::system::single_instance::acquire(
        "WinSP_SingleInstance_Mutex",
        ui::WINDOW_CLASS_NAME,
    ) {
        AcquireResult::Acquired(guard) => guard,
        AcquireResult::AlreadyRunning { brought_to_front } => {
            if !brought_to_front {
                winsp_windows::system::toast::show(
                    "WinSP",
                    "WinSP is already running but couldn't be brought to the front.",
                );
            }
            return Ok(());
        }
        AcquireResult::Failed => return Ok(()),
    };

    let catalog = sync::scan_catalog();
    let index = sync::engine_from_catalog(&catalog);

    let state = Arc::new(Mutex::new(AppState::new(index)));

    let (_reindex_watcher, reconcile_tx) = sync::start_watching(catalog);

    ui::run(state, reconcile_tx).map_err(|e| e.into())
}
