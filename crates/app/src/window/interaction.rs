use winsp_core::models::{AppTarget, SearchResultKind};

use crate::state::AppState;

pub(super) fn insert_char(state: &mut AppState, c: char) {
    state.query.push(c);
    state.refresh_results();
}

pub(super) fn backspace(state: &mut AppState) {
    state.query.pop();
    state.refresh_results();
}

pub(super) fn clear_query(state: &mut AppState) {
    state.query.clear();
    state.refresh_results();
}

pub(super) fn select_next(state: &mut AppState) {
    if !state.results.is_empty() {
        state.selected_index = (state.selected_index + 1) % state.results.len();
    }
}

pub(super) fn select_prev(state: &mut AppState) {
    if !state.results.is_empty() {
        if state.selected_index == 0 {
            state.selected_index = state.results.len() - 1;
        } else {
            state.selected_index -= 1;
        }
    }
}

pub(super) enum ExecuteOutcome {
    Copy(String),
    Launch(AppTarget),
    None,
}

pub(super) fn execute_selected(state: &AppState) -> ExecuteOutcome {
    let Some(selected) = state.results.get(state.selected_index) else {
        return ExecuteOutcome::None;
    };
    match &selected.kind {
        SearchResultKind::App(item) => ExecuteOutcome::Launch(item.target.clone()),
        SearchResultKind::Calculation { result, .. } => ExecuteOutcome::Copy(result.clone()),
        SearchResultKind::WebSearch { url, .. } => {
            ExecuteOutcome::Launch(AppTarget::Path(url.clone()))
        }
        SearchResultKind::SystemCommand { command, .. } => {
            ExecuteOutcome::Launch(AppTarget::SystemCommand(command.clone()))
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
    fn execute_selected_with_no_matches_returns_none() {
        let mut state = sample_state();
        for c in "zzzznomatchzzzz".chars() {
            insert_char(&mut state, c);
        }
        assert!(state.results.is_empty());
        assert!(matches!(execute_selected(&state), ExecuteOutcome::None));
    }

    #[test]
    fn execute_selected_on_a_calculation_returns_a_value_to_copy_without_launching() {
        let mut state = sample_state();
        for c in "1+1".chars() {
            insert_char(&mut state, c);
        }

        match execute_selected(&state) {
            ExecuteOutcome::Copy(result) => assert_eq!(result, "2"),
            _ => panic!("expected a Copy outcome, got a different variant instead"),
        }
    }

    #[test]
    fn execute_selected_on_an_app_returns_its_target_without_launching() {
        let mut state = sample_state();
        for c in "calc".chars() {
            insert_char(&mut state, c);
        }
        assert!(!state.results.is_empty());

        assert!(matches!(
            execute_selected(&state),
            ExecuteOutcome::Launch(_)
        ));
    }

    #[test]
    fn insert_char_appends_and_refreshes() {
        let mut state = sample_state();

        insert_char(&mut state, 'c');
        insert_char(&mut state, 'a');
        insert_char(&mut state, 'l');
        insert_char(&mut state, 'c');

        assert_eq!(state.query, "calc");
        assert!(!state.results.is_empty());
    }

    #[test]
    fn backspace_and_clear_query_refresh_results() {
        let mut state = sample_state();
        insert_char(&mut state, 'c');
        insert_char(&mut state, 'a');
        insert_char(&mut state, 'l');
        insert_char(&mut state, 'c');

        backspace(&mut state);
        assert_eq!(state.query, "cal");

        clear_query(&mut state);
        assert_eq!(state.query, "");
    }

    #[test]
    fn select_next_and_prev_wrap_around() {
        let mut state = sample_state();
        insert_char(&mut state, 'a');
        let count = state.results.len();
        if count < 2 {
            return;
        }

        select_next(&mut state);
        assert_eq!(state.selected_index, 1);

        select_prev(&mut state);
        assert_eq!(state.selected_index, 0);

        select_prev(&mut state);
        assert_eq!(state.selected_index, count - 1);
    }
}
