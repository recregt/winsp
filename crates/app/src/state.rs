use winsp_core::index::Index;
use winsp_core::models::AppItem;

pub(crate) type Catalog = Index<AppItem>;

#[derive(Debug)]
pub struct AppState {
    index: Catalog,
}

impl AppState {
    pub fn new(index: Catalog) -> Self {
        Self { index }
    }

    pub(crate) fn update_index(&mut self, index: Catalog) {
        self.index = index;
    }

    pub(crate) fn engine(&self) -> &Catalog {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_index_replaces_the_engine() {
        let mut state = AppState::new(Catalog::new());
        state.update_index(Catalog::new());
        assert!(state.engine().search("", 1).is_empty());
    }
}
