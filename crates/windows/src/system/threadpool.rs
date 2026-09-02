use std::ffi::c_void;

use windows::Win32::System::Threading::{PTP_CALLBACK_INSTANCE, TrySubmitThreadpoolCallback};

unsafe extern "system" fn run_boxed_closure(
    _instance: PTP_CALLBACK_INSTANCE,
    context: *mut c_void,
) {
    let f = unsafe { Box::from_raw(context as *mut Box<dyn FnOnce() + Send>) };
    f();
}

pub fn spawn_on_threadpool(f: impl FnOnce() + Send + 'static) -> bool {
    let boxed: Box<dyn FnOnce() + Send> = Box::new(f);
    let context = Box::into_raw(Box::new(boxed)) as *mut c_void;

    let submitted =
        unsafe { TrySubmitThreadpoolCallback(Some(run_boxed_closure), Some(context), None) };

    if submitted.is_err() {
        unsafe {
            drop(Box::from_raw(context as *mut Box<dyn FnOnce() + Send>));
        }
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn runs_the_closure_on_a_different_thread() {
        let calling_thread = std::thread::current().id();
        let (tx, rx) = mpsc::channel();
        assert!(spawn_on_threadpool(move || {
            let _ = tx.send(std::thread::current().id());
        }));
        let pool_thread = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_ne!(pool_thread, calling_thread);
    }

    #[test]
    fn actually_executes_and_delivers_the_result() {
        let (tx, rx) = mpsc::channel();
        assert!(spawn_on_threadpool(move || {
            let _ = tx.send(2 + 2);
        }));
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), 4);
    }
}
