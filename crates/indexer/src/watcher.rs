use notify_debouncer_mini::notify::{self, RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use std::time::Duration;

pub fn watch_dirs<F>(
    dirs: &[std::path::PathBuf],
    on_change: F,
) -> notify::Result<Debouncer<RecommendedWatcher>>
where
    F: Fn() + Send + 'static,
{
    let mut debouncer = new_debouncer(
        Duration::from_millis(750),
        move |res: DebounceEventResult| {
            if res.is_ok() {
                on_change();
            }
        },
    )?;

    for dir in dirs {
        let _ = debouncer.watcher().watch(dir, RecursiveMode::Recursive);
    }

    Ok(debouncer)
}

#[cfg(windows)]
pub fn watch_start_menu<F>(on_change: F) -> notify::Result<Debouncer<RecommendedWatcher>>
where
    F: Fn() + Send + 'static,
{
    watch_dirs(&crate::shell_apps::start_menu_dirs(), on_change)
}

pub fn test_watch_dir() -> Option<std::path::PathBuf> {
    std::env::var("WINSP_TEST_WATCH_DIR")
        .ok()
        .map(std::path::PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::time::Duration as StdDuration;

    #[test]
    fn reindex_fires_on_file_change() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = channel();

        let _watcher = watch_dirs(&[dir.path().to_path_buf()], move || {
            let _ = tx.send(());
        })
        .unwrap();

        std::fs::write(dir.path().join("new_app.lnk"), b"").unwrap();

        rx.recv_timeout(StdDuration::from_secs(5))
            .expect("reindex callback did not fire in time");
    }
}
