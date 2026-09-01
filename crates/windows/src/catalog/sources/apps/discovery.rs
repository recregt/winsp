use winsp_core::models::AppItem;

use super::builtin::built_in_tools;
use super::catalog::StartMenuCatalog;

pub fn list_installed_apps() -> Vec<AppItem> {
    merge_with_built_ins(StartMenuCatalog::for_start_menu().items())
}

pub fn merge_with_built_ins(mut apps: Vec<AppItem>) -> Vec<AppItem> {
    let mut seen_ids: std::collections::HashSet<String> =
        apps.iter().map(|item| item.id.clone()).collect();

    for item in built_in_tools() {
        if seen_ids.insert(item.id.clone()) {
            apps.push(item);
        }
    }

    apps
}
