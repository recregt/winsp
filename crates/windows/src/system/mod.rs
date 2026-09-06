mod find_exe;
mod registry;

pub mod autostart;
pub mod clipboard;
pub mod single_instance;
pub mod theme;
pub mod toast;
pub mod watcher;

pub(crate) mod com;
pub mod shortcut;
pub mod threadpool;

pub use find_exe::find_exe;
