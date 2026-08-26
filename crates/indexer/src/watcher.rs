#[cfg(windows)]
use notify_debouncer_mini::notify::{self, RecommendedWatcher, RecursiveMode};
#[cfg(windows)]
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
pub fn watch_start_menu<F>(on_change: F) -> notify::Result<Debouncer<RecommendedWatcher>>
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

    for dir in crate::shell_apps::start_menu_dirs() {
        let _ = debouncer.watcher().watch(&dir, RecursiveMode::Recursive);
    }

    Ok(debouncer)
}
