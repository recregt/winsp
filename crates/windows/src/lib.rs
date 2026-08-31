pub mod apps;
pub mod launcher;
pub mod settings;
pub mod watcher;

use winsp_core::models::AppItem;
use winsp_core::search::Engine;

/// Loads all installed apps and Windows Settings shortcuts into the search index.
pub fn populate_search_index() -> Engine {
    let mut index = Engine::new();
    let mut all_items: Vec<AppItem> = Vec::new();

    // 1. Enumerate Windows apps (Shell / UWP / Win32)
    let installed_apps = apps::enumerate_installed_apps();
    all_items.extend(installed_apps);

    // 2. Add Windows Settings quick items
    let settings_items = settings::get_windows_settings_items();
    all_items.extend(settings_items);

    index.set_items(all_items);
    index
}
