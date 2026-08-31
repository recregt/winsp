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
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::UI::Shell::ShellExecuteW;
            use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

            let path_wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
            let op_wide: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();

            unsafe {
                let instance = ShellExecuteW(
                    std::ptr::null_mut(),
                    op_wide.as_ptr(),
                    path_wide.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    SW_SHOWNORMAL,
                );

                if (instance as usize) > 32 {
                    Ok(())
                } else {
                    Err(format!("Failed to execute path with code: {:?}", instance))
                }
            }
        }

        fn launch_aumid(aumid: &str) -> Result<(), String> {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::UI::Shell::ShellExecuteW;
            use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

            let explorer_wide: Vec<u16> = OsStr::new("explorer.exe")
                .encode_wide()
                .chain(Some(0))
                .collect();
            let param = format!("shell:AppsFolder\\{}", aumid);
            let param_wide: Vec<u16> = OsStr::new(&param).encode_wide().chain(Some(0)).collect();
            let op_wide: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();

            unsafe {
                let instance = ShellExecuteW(
                    std::ptr::null_mut(),
                    op_wide.as_ptr(),
                    explorer_wide.as_ptr(),
                    param_wide.as_ptr(),
                    std::ptr::null(),
                    SW_SHOWNORMAL,
                );

                if (instance as usize) > 32 {
                    Ok(())
                } else {
                    Err(format!("Failed to launch UWP app: {}", aumid))
                }
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
