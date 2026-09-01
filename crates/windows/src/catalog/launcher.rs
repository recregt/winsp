use winsp_core::models::AppTarget;

pub fn run(target: &AppTarget) -> Result<(), String> {
    match target {
        AppTarget::Path(path) => launch_path(path),
        AppTarget::Aumid(aumid) => launch_aumid(aumid),
        AppTarget::SettingUri(uri) => launch_uri(uri),
        AppTarget::SystemCommand(cmd) => launch_command(cmd),
    }
}

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        fn launch_path(path: &str) -> Result<(), String> {
            use windows::Win32::UI::Shell::ShellExecuteW;
            use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
            use windows::core::{HSTRING, w};

            let path = HSTRING::from(path);

            let instance =
                unsafe { ShellExecuteW(None, w!("open"), &path, None, None, SW_SHOWNORMAL) };

            if instance.0 as usize > 32 {
                Ok(())
            } else {
                Err(format!("Failed to execute path with code: {instance:?}"))
            }
        }

        fn launch_aumid(aumid: &str) -> Result<(), String> {
            use windows::Win32::UI::Shell::ShellExecuteW;
            use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
            use windows::core::{HSTRING, w};

            let param = HSTRING::from(format!("shell:AppsFolder\\{aumid}"));

            let instance = unsafe {
                ShellExecuteW(
                    None,
                    w!("open"),
                    w!("explorer.exe"),
                    &param,
                    None,
                    SW_SHOWNORMAL,
                )
            };

            if instance.0 as usize > 32 {
                Ok(())
            } else {
                Err(format!("Failed to launch UWP app: {aumid}"))
            }
        }

        fn launch_uri(uri: &str) -> Result<(), String> {
            launch_path(uri)
        }

        fn launch_command(cmd: &str) -> Result<(), String> {
            std::process::Command::new("cmd")
                .args(["/C", cmd])
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    } else {
        fn launch_path(path: &str) -> Result<(), String> {
            println!("[WinSP Stub Launch] Launching path: {}", path);
            Ok(())
        }

        fn launch_aumid(aumid: &str) -> Result<(), String> {
            println!("[WinSP Stub Launch] Launching AUMID: {}", aumid);
            Ok(())
        }

        fn launch_uri(uri: &str) -> Result<(), String> {
            println!("[WinSP Stub Launch] Launching URI: {}", uri);
            Ok(())
        }

        fn launch_command(cmd: &str) -> Result<(), String> {
            println!("[WinSP Stub Launch] Launching command: {}", cmd);
            Ok(())
        }
    }
}
