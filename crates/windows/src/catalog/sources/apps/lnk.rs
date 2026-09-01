use std::os::windows::ffi::OsStrExt;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ,
};
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink};
use windows::core::{Interface, PCWSTR};

pub(super) struct LnkResolver {
    shell_link: IShellLinkW,
    persist_file: IPersistFile,
}

impl LnkResolver {
    pub(super) fn new() -> Option<Self> {
        unsafe {
            let shell_link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
            let persist_file: IPersistFile = shell_link.cast().ok()?;
            Some(Self {
                shell_link,
                persist_file,
            })
        }
    }

    pub(super) fn resolve(&self, path: &std::path::Path) -> Option<String> {
        unsafe {
            let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            self.persist_file
                .Load(PCWSTR(wide_path.as_ptr()), STGM_READ)
                .ok()?;

            let mut raw_path = [0u16; 260];
            self.shell_link
                .GetPath(&mut raw_path, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
                .ok()?;

            let target = expand_env_vars(&raw_path);
            if target.is_empty() {
                return None;
            }

            let mut args_buf = [0u16; 1024];
            let arguments = self
                .shell_link
                .GetArguments(&mut args_buf)
                .ok()
                .map(|()| wide_str_from_buf(&args_buf))
                .unwrap_or_default();

            Some(format!(
                "{}|{}",
                target.to_lowercase(),
                arguments.to_lowercase()
            ))
        }
    }
}

fn wide_str_from_buf(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn expand_env_vars(raw: &[u16]) -> String {
    let mut expanded = [0u16; 260];
    let written = unsafe { ExpandEnvironmentStringsW(PCWSTR(raw.as_ptr()), Some(&mut expanded)) };
    if written == 0 || written as usize > expanded.len() {
        wide_str_from_buf(raw)
    } else {
        wide_str_from_buf(&expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_environment_variables_in_lnk_target() {
        let program_files = std::env::var("ProgramFiles").unwrap();
        let raw: Vec<u16> = std::ffi::OsStr::new(r"%ProgramFiles%\App\app.exe")
            .encode_wide()
            .chain(Some(0))
            .collect();

        let expanded = expand_env_vars(&raw);

        assert_eq!(
            expanded.to_lowercase(),
            format!(r"{program_files}\App\app.exe").to_lowercase()
        );
    }
}
