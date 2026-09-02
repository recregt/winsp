use winsp_core::models::SearchResult;
use winsp_core::search::Engine;

#[derive(Debug)]
pub struct AppState {
    pub index: Engine,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub capturing_hotkey: bool,
}

impl AppState {
    pub fn new(index: Engine) -> Self {
        let initial_results = index.search("", 6);
        Self {
            index,
            query: String::new(),
            results: initial_results,
            selected_index: 0,
            capturing_hotkey: false,
        }
    }

    pub fn refresh_results(&mut self) {
        self.results = self.index.search(&self.query, 6);
        self.selected_index = 0;
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::index::populate_search_index;

    #[test]
    fn refresh_results_reflects_the_current_query() {
        let index = populate_search_index();
        let mut state = AppState::new(index);

        state.query = "calc".into();
        state.refresh_results();
        assert!(!state.results.is_empty());
        assert_eq!(state.selected_index, 0);

        state.query.clear();
        state.refresh_results();
        assert_eq!(state.query, "");
    }
}
