#[cfg(feature = "test-support")]
pub mod testing {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    pub struct ComGuard;

    impl ComGuard {
        pub fn new() -> Self {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
            }
            Self
        }
    }

    impl Default for ComGuard {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub fn create_test_lnk(dir: &Path, name: &str, target: &str, args: &str) {
        unsafe {
            let shell_link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();

            shell_link.SetPath(PCWSTR(wide(target).as_ptr())).unwrap();
            if !args.is_empty() {
                shell_link
                    .SetArguments(PCWSTR(wide(args).as_ptr()))
                    .unwrap();
            }

            let persist_file: IPersistFile = shell_link.cast().unwrap();
            let lnk_path = dir.join(format!("{name}.lnk"));
            persist_file
                .Save(PCWSTR(wide(&lnk_path.to_string_lossy()).as_ptr()), true)
                .unwrap();
        }
    }
}
