use winsp_core::{SearchIndex, SearchResult, SearchResultKind};
use winsp_indexer::launcher::launch_target;

#[derive(Debug)]
pub struct AppState {
    pub index: SearchIndex,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    #[allow(dead_code)]
    pub is_visible: bool,
}

impl AppState {
    pub fn new(index: SearchIndex) -> Self {
        let initial_results = index.search("", 6);
        Self {
            index,
            query: String::new(),
            results: initial_results,
            selected_index: 0,
            is_visible: false,
        }
    }

    #[allow(dead_code)]
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.refresh_results();
    }

    #[allow(dead_code)]
    pub fn insert_char(&mut self, c: char) {
        self.query.push(c);
        self.refresh_results();
    }

    #[allow(dead_code)]
    pub fn backspace(&mut self) {
        self.query.pop();
        self.refresh_results();
    }

    #[allow(dead_code)]
    pub fn clear_query(&mut self) {
        self.query.clear();
        self.refresh_results();
    }

    #[allow(dead_code)]
    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    #[allow(dead_code)]
    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.results.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn refresh_results(&mut self) {
        self.results = self.index.search(&self.query, 6);
        self.selected_index = 0;
    }

    #[allow(dead_code)]
    pub fn execute_selected(&mut self) -> Result<(), String> {
        if let Some(selected) = self.results.get(self.selected_index) {
            match &selected.kind {
                SearchResultKind::App(item) => {
                    launch_target(&item.target)?;
                }
                SearchResultKind::Calculation { result, .. } => {
                    println!("[WinSP] Calculation result: {}", result);
                }
                SearchResultKind::WebSearch { url, .. } => {
                    launch_target(&winsp_core::AppTarget::Path(url.clone()))?;
                }
                SearchResultKind::SystemCommand { command, .. } => {
                    launch_target(&winsp_core::AppTarget::SystemCommand(command.clone()))?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winsp_indexer::populate_search_index;

    #[test]
    fn test_app_state_navigation() {
        let index = populate_search_index();
        let mut state = AppState::new(index);

        state.set_query("calc".into());
        assert!(!state.results.is_empty());
        assert_eq!(state.selected_index, 0);

        state.select_next();
        if state.results.len() > 1 {
            assert_eq!(state.selected_index, 1);
        }

        state.select_prev();
        assert_eq!(state.selected_index, 0);

        state.backspace();
        assert_eq!(state.query, "cal");

        state.clear_query();
        assert_eq!(state.query, "");
    }
}
