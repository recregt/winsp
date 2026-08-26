use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::ApplicationModel::{Package, StartupTask, StartupTaskState};
use windows::core::HSTRING;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};

const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "WinSP";
const STARTUP_TASK_ID: &str = "WinSPStartup";

fn is_packaged() -> bool {
    Package::Current().is_ok()
}

pub fn is_enabled() -> bool {
    if is_packaged() {
        startup_task_is_enabled().unwrap_or(false)
    } else {
        registry_is_enabled()
    }
}

pub fn set_enabled(enabled: bool) {
    if is_packaged() {
        let _ = startup_task_set_enabled(enabled);
    } else {
        registry_set_enabled(enabled);
    }
}

fn startup_task_is_enabled() -> windows::core::Result<bool> {
    let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.get()?;
    let state = task.State()?;
    Ok(matches!(
        state,
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
    ))
}

fn startup_task_set_enabled(enabled: bool) -> windows::core::Result<()> {
    let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.get()?;
    if enabled {
        task.RequestEnableAsync()?.get()?;
    } else {
        task.Disable()?;
    }
    Ok(())
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn current_exe_path() -> Option<Vec<u16>> {
    let mut buf = vec![0u16; 1024];
    let len =
        unsafe { GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 {
        return None;
    }
    buf.truncate(len as usize);
    buf.push(0);
    Some(buf)
}

fn registry_is_enabled() -> bool {
    unsafe {
        let path = to_wide(RUN_KEY_PATH);
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut hkey,
        ) != 0
        {
            return false;
        }
        let name = to_wide(VALUE_NAME);
        let result = RegQueryValueExW(
            hkey,
            name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        RegCloseKey(hkey);
        result == 0
    }
}

fn registry_set_enabled(enabled: bool) {
    unsafe {
        let path = to_wide(RUN_KEY_PATH);
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        ) != 0
        {
            return;
        }
        let name = to_wide(VALUE_NAME);
        if enabled {
            if let Some(exe_path) = current_exe_path() {
                let bytes =
                    std::slice::from_raw_parts(exe_path.as_ptr().cast::<u8>(), exe_path.len() * 2);
                RegSetValueExW(
                    hkey,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    bytes.as_ptr(),
                    bytes.len() as u32,
                );
            }
        } else {
            RegDeleteValueW(hkey, name.as_ptr());
        }
        RegCloseKey(hkey);
    }
}
