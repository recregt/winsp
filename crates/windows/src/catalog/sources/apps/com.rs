use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

pub(super) struct ComGuard {
    owns_apartment: bool,
}

impl ComGuard {
    pub(super) fn new() -> Option<Self> {
        unsafe {
            match CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() {
                Ok(()) => Some(Self {
                    owns_apartment: true,
                }),
                Err(error) if error.code() == RPC_E_CHANGED_MODE => Some(Self {
                    owns_apartment: false,
                }),
                Err(_) => {
                    notify_com_init_failed();
                    None
                }
            }
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.owns_apartment {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

fn notify_com_init_failed() {
    static NOTIFIED: std::sync::Once = std::sync::Once::new();
    NOTIFIED.call_once(|| {
        crate::system::toast::show(
            "WinSP",
            "Couldn't scan Start Menu shortcuts this time. Some apps may be missing from search until WinSP restarts.",
        );
    });
}
