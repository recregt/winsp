#![cfg(windows)]

use winsp_core::models::AppItem;
use winsp_core::search::Engine;

pub(crate) fn populate_search_index() -> Engine {
    let mut index = Engine::new();
    let mut all_items: Vec<AppItem> = Vec::new();

    all_items.extend(winsp_windows::catalog::sources::apps::list_installed_apps());
    all_items.extend(winsp_windows::catalog::sources::settings::list_settings());

    index.set_items(all_items);
    index
}

pub(crate) fn engine_from_catalog(
    catalog: &winsp_windows::catalog::sources::apps::StartMenuCatalog,
) -> Engine {
    let mut index = Engine::new();
    let mut all_items: Vec<AppItem> =
        winsp_windows::catalog::sources::apps::merge_with_built_ins(catalog.items());
    all_items.extend(winsp_windows::catalog::sources::settings::list_settings());

    index.set_items(all_items);
    index
}
