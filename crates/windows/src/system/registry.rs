use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{HKEY, RRF_RT_REG_DWORD, RegGetValueW};

pub(super) fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

/// Reads a 32 bit integer value from the Windows registry.
///
/// # Safety
///
/// `hive` must be a valid, currently open registry key handle.
pub unsafe fn read_dword(hive: HKEY, subkey: &str, value: &str) -> Option<u32> {
    let subkey = to_wide(subkey);
    let value = to_wide(value);
    let mut data: u32 = 0;
    let mut data_size = std::mem::size_of::<u32>() as u32;

    let status = unsafe {
        RegGetValueW(
            hive,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut _,
            &mut data_size,
        )
    };

    (status == ERROR_SUCCESS).then_some(data)
}
