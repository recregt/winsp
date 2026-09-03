#![cfg(windows)]

use winsp_core::models::{AppTarget, SearchResult, SearchResultKind};
use winsp_core::search::Engine;

#[derive(Debug)]
pub struct AppState {
    pub index: Engine,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub capturing_hotkey: bool,
}

pub(crate) enum ExecuteOutcome {
    Copy(String),
    Launch(AppTarget),
    None,
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

    pub(crate) fn insert_char(&mut self, c: char) {
        self.query.push(c);
        self.refresh_results();
    }

    pub(crate) fn backspace(&mut self) {
        self.query.pop();
        self.refresh_results();
    }

    pub(crate) fn clear_query(&mut self) {
        self.query.clear();
        self.refresh_results();
    }

    pub(crate) fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    pub(crate) fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.results.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub(crate) fn execute_selected(&self) -> ExecuteOutcome {
        let Some(selected) = self.results.get(self.selected_index) else {
            return ExecuteOutcome::None;
        };
        match &selected.kind {
            SearchResultKind::App(item) => ExecuteOutcome::Launch(item.target.clone()),
            SearchResultKind::Calculation { result, .. } => ExecuteOutcome::Copy(result.clone()),
            SearchResultKind::WebSearch { url, .. } => {
                ExecuteOutcome::Launch(AppTarget::Uri(url.clone()))
            }
            SearchResultKind::SystemCommand { command, .. } => {
                ExecuteOutcome::Launch(AppTarget::SystemCommand(command.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::populate_search_index;

    fn sample_state() -> AppState {
        let index = populate_search_index();
        AppState::new(index)
    }

    #[test]
    fn refresh_results_reflects_the_current_query() {
        let mut state = sample_state();

        state.query = "calc".into();
        state.refresh_results();
        assert!(!state.results.is_empty());
        assert_eq!(state.selected_index, 0);

        state.query.clear();
        state.refresh_results();
        assert_eq!(state.query, "");
    }

    #[test]
    fn execute_selected_with_no_matches_returns_none() {
        let mut state = sample_state();
        for c in "zzzznomatchzzzz".chars() {
            state.insert_char(c);
        }
        assert!(state.results.is_empty());
        assert!(matches!(state.execute_selected(), ExecuteOutcome::None));
    }

    #[test]
    fn execute_selected_on_a_calculation_returns_a_value_to_copy_without_launching() {
        let mut state = sample_state();
        for c in "1+1".chars() {
            state.insert_char(c);
        }

        match state.execute_selected() {
            ExecuteOutcome::Copy(result) => assert_eq!(result, "2"),
            _ => panic!("expected a Copy outcome, got a different variant instead"),
        }
    }

    #[test]
    fn execute_selected_on_an_app_returns_its_target_without_launching() {
        let mut state = sample_state();
        for c in "calc".chars() {
            state.insert_char(c);
        }
        assert!(!state.results.is_empty());

        assert!(matches!(
            state.execute_selected(),
            ExecuteOutcome::Launch(_)
        ));
    }

    #[test]
    fn insert_char_appends_and_refreshes() {
        let mut state = sample_state();

        state.insert_char('c');
        state.insert_char('a');
        state.insert_char('l');
        state.insert_char('c');

        assert_eq!(state.query, "calc");
        assert!(!state.results.is_empty());
    }

    #[test]
    fn backspace_and_clear_query_refresh_results() {
        let mut state = sample_state();
        state.insert_char('c');
        state.insert_char('a');
        state.insert_char('l');
        state.insert_char('c');

        state.backspace();
        assert_eq!(state.query, "cal");

        state.clear_query();
        assert_eq!(state.query, "");
    }

    #[test]
    fn select_next_and_prev_wrap_around() {
        let mut state = sample_state();
        state.insert_char('a');
        let count = state.results.len();
        if count < 2 {
            return;
        }

        state.select_next();
        assert_eq!(state.selected_index, 1);

        state.select_prev();
        assert_eq!(state.selected_index, 0);

        state.select_prev();
        assert_eq!(state.selected_index, count - 1);
    }
}
