use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};

use windows::Win32::System::Threading::{PTP_CALLBACK_INSTANCE, TrySubmitThreadpoolCallback};

unsafe extern "system" fn run_closure<F: FnOnce() + Send>(
    _instance: PTP_CALLBACK_INSTANCE,
    context: *mut c_void,
) {
    let f = *unsafe { Box::from_raw(context as *mut F) };
    let _ = catch_unwind(AssertUnwindSafe(f));
}

pub fn spawn_on_threadpool<F: FnOnce() + Send + 'static>(f: F) -> bool {
    let context = Box::into_raw(Box::new(f)) as *mut c_void;

    let submitted =
        unsafe { TrySubmitThreadpoolCallback(Some(run_closure::<F>), Some(context), None) };

    if submitted.is_err() {
        unsafe {
            drop(Box::from_raw(context as *mut F));
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

    #[test]
    fn a_panicking_closure_does_not_abort_the_process() {
        assert!(spawn_on_threadpool(|| {
            panic!("deliberate panic to prove the pool survives it");
        }));

        let (tx, rx) = mpsc::channel();
        assert!(spawn_on_threadpool(move || {
            let _ = tx.send(());
        }));
        assert!(
            rx.recv_timeout(Duration::from_secs(2)).is_ok(),
            "the process (and this test) must still be alive after the panic to reach this point"
        );
    }
}
