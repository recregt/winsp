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
                Err(shell_execute_error_message(instance.0 as usize))
            }
        }

        fn shell_execute_error_message(code: usize) -> String {
            use windows::Win32::Foundation::ERROR_BAD_FORMAT;
            use windows::Win32::UI::Shell::{
                SE_ERR_ACCESSDENIED, SE_ERR_ASSOCINCOMPLETE, SE_ERR_DDEBUSY, SE_ERR_DDEFAIL,
                SE_ERR_DDETIMEOUT, SE_ERR_DLLNOTFOUND, SE_ERR_FNF, SE_ERR_NOASSOC, SE_ERR_OOM,
                SE_ERR_PNF, SE_ERR_SHARE,
            };
            const BAD_FORMAT: u32 = ERROR_BAD_FORMAT.0;

            match code as u32 {
                0 | SE_ERR_OOM => "The system is out of memory or resources.".into(),
                SE_ERR_FNF => "File not found.".into(),
                SE_ERR_PNF => "Path not found.".into(),
                BAD_FORMAT => "This file is not a valid Windows program.".into(),
                SE_ERR_ACCESSDENIED => "Access denied.".into(),
                SE_ERR_ASSOCINCOMPLETE | SE_ERR_NOASSOC => {
                    "No app is associated with this file type.".into()
                }
                SE_ERR_DLLNOTFOUND => "A required component is missing.".into(),
                SE_ERR_SHARE => "The file is in use by another program.".into(),
                SE_ERR_DDEBUSY | SE_ERR_DDEFAIL | SE_ERR_DDETIMEOUT => {
                    "The other application didn't respond in time.".into()
                }
                _ => format!("Failed to open this item (code {code})."),
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

#[cfg(all(test, windows))]
mod tests {
    use super::shell_execute_error_message;

    #[test]
    fn maps_documented_codes_to_readable_messages() {
        assert_eq!(shell_execute_error_message(2), "File not found.");
        assert_eq!(shell_execute_error_message(3), "Path not found.");
        assert_eq!(shell_execute_error_message(5), "Access denied.");
        assert_eq!(
            shell_execute_error_message(31),
            "No app is associated with this file type."
        );
        assert_eq!(
            shell_execute_error_message(32),
            "A required component is missing."
        );
    }

    #[test]
    fn falls_back_to_the_raw_code_for_unmapped_values() {
        assert_eq!(
            shell_execute_error_message(9),
            "Failed to open this item (code 9)."
        );
    }
}
