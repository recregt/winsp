use notify_debouncer_mini::notify::{self, RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use std::path::PathBuf;
use std::time::Duration;

pub enum WatchEvent {
    Changed(Vec<PathBuf>),
    Uncertain,
}

pub fn for_dirs<F>(dirs: &[PathBuf], on_event: F) -> notify::Result<Debouncer<RecommendedWatcher>>
where
    F: Fn(WatchEvent) + Send + 'static,
{
    let mut debouncer = new_debouncer(
        Duration::from_millis(750),
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                let paths = events.into_iter().map(|event| event.path).collect();
                on_event(WatchEvent::Changed(paths));
            }
            Err(_) => on_event(WatchEvent::Uncertain),
        },
    )?;

    for dir in dirs {
        if let Err(err) = debouncer.watcher().watch(dir, RecursiveMode::Recursive) {
            eprintln!("failed to watch {}: {err}", dir.display());
        }
    }

    Ok(debouncer)
}

#[cfg(windows)]
pub fn for_start_menu<F>(on_event: F) -> notify::Result<Debouncer<RecommendedWatcher>>
where
    F: Fn(WatchEvent) + Send + 'static,
{
    for_dirs(&super::apps::start_menu_dirs(), on_event)
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
        let new_app = dir.path().join("new_app.lnk");

        let _watcher = for_dirs(&[dir.path().to_path_buf()], move |event| {
            let _ = tx.send(event);
        })
        .unwrap();

        std::fs::write(&new_app, b"").unwrap();

        let event = rx
            .recv_timeout(StdDuration::from_secs(5))
            .expect("reindex callback did not fire in time");

        match event {
            WatchEvent::Changed(paths) => assert!(paths.contains(&new_app)),
            WatchEvent::Uncertain => panic!("expected a Changed event, got Uncertain"),
        }
    }
}
