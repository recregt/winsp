#![cfg(windows)]

use winsp_core::engine::Engine;
use winsp_windows::catalog::Catalog;

pub(crate) fn populate_search_index() -> Engine {
    let mut index = Engine::new();
    index.set_items(Catalog::scan().items());
    index
}

pub(crate) fn engine_from_catalog(catalog: &Catalog) -> Engine {
    let mut index = Engine::new();
    index.set_items(catalog.items());
    index
}
