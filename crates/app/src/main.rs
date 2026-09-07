#![cfg(windows)]
#![cfg_attr(windows, windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use winsp_service::Service;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use winsp_windows::system::single_instance::AcquireResult;

    let _instance_mutex = match winsp_windows::system::single_instance::acquire(
        "WinSP_SingleInstance_Mutex",
        winsp_ui::WINDOW_CLASS_NAME,
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

    let service = Service::start(winsp_ui::notify_index_changed);

    winsp_ui::run(service).map_err(|e| e.into())
}
