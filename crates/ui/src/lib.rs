#![cfg(windows)]
#![forbid(unsafe_code)]

mod controller;
mod hotkey;
mod view;

use std::sync::{Arc, Mutex, OnceLock};

use winsp_service::{ResultRow, Service};
use winsp_windows::window::{Hotkey, HotkeySlot, Window};

use controller::handle_event;

const WINDOW_WIDTH: i32 = 680;
const SEARCH_BAR_HEIGHT: i32 = 64;
const ITEM_ROW_HEIGHT: i32 = 54;
const PADDING: i32 = 12;
pub const WINDOW_CLASS_NAME: &str = "WinSP_Spotlight_Window";
const REFRESH_EVENT: u32 = 1;
const MAX_RESULTS: usize = 6;

pub fn notify_index_changed() {
    winsp_windows::window::post_event(REFRESH_EVENT);
}

struct UiState {
    query: String,
    results: Vec<ResultRow>,
    selected_index: usize,
    capturing_hotkey: bool,
    stale: bool,
}

impl UiState {
    fn new(service: &Service) -> Self {
        let results = service.search("", MAX_RESULTS);
        Self {
            query: String::new(),
            results,
            selected_index: 0,
            capturing_hotkey: false,
            stale: false,
        }
    }

    fn refresh_against(&mut self, service: &Service) {
        self.results = service.search(&self.query, MAX_RESULTS);
        self.selected_index = 0;
        self.stale = false;
    }

    fn settle(&mut self, service: &Service) -> bool {
        if !self.stale {
            return false;
        }
        self.refresh_against(service);
        true
    }

    fn query(&self) -> &str {
        &self.query
    }

    fn results(&self) -> &[ResultRow] {
        &self.results
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn is_capturing_hotkey(&self) -> bool {
        self.capturing_hotkey
    }

    fn start_capturing_hotkey(&mut self) {
        self.capturing_hotkey = true;
    }

    fn stop_capturing_hotkey(&mut self, service: &Service) {
        self.capturing_hotkey = false;
        self.clear_query();
        self.settle(service);
    }

    fn insert_char(&mut self, c: char) {
        self.query.push(c);
        self.stale = true;
    }

    fn insert_text(&mut self, text: &str) {
        let filtered = text.chars().filter(|c| !c.is_control());
        let before = self.query.len();
        self.query.extend(filtered);
        self.stale = self.stale || self.query.len() != before;
    }

    fn backspace(&mut self) {
        if self.query.pop().is_some() {
            self.stale = true;
        }
    }

    fn clear_query(&mut self) {
        if self.query.is_empty() {
            self.selected_index = 0;
            return;
        }
        self.query.clear();
        self.stale = true;
    }

    fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.results.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }
}

struct AppContext {
    service: Arc<Service>,
    ui_state: Mutex<UiState>,
    active_hotkey_slot: Mutex<HotkeySlot>,
}

static APP: OnceLock<AppContext> = OnceLock::new();

fn context() -> Option<&'static AppContext> {
    APP.get()
}

pub fn run(service: Arc<Service>) -> Result<(), String> {
    let (modifiers, key) = service.current_hotkey();
    let anchor = service.current_position();
    let hotkey = Hotkey::new(modifiers, key);

    let ui_state = UiState::new(&service);

    let _ = APP.set(AppContext {
        service,
        ui_state: Mutex::new(ui_state),
        active_hotkey_slot: Mutex::new(HotkeySlot::Primary),
    });

    winsp_windows::system::theme::allow_dark_mode_for_app();

    let window_handle = Window::create(
        WINDOW_CLASS_NAME,
        "WinSP",
        WINDOW_WIDTH,
        SEARCH_BAR_HEIGHT,
        handle_event,
    )
    .map_err(|e| format!("failed to create window: {e}"))?;
    window_handle.enable_dark_mode();

    window_handle.center(WINDOW_WIDTH, SEARCH_BAR_HEIGHT, anchor);
    if !window_handle.add_tray_icon() {
        winsp_windows::system::toast::show(
            "WinSP",
            "Couldn't add the tray icon. Use the hotkey to open WinSP.",
        );
    }
    if !window_handle.register_hotkey(HotkeySlot::Primary, hotkey) {
        winsp_windows::system::toast::show(
            "WinSP",
            &format!(
                "Failed to register global hotkey: {}",
                std::io::Error::last_os_error()
            ),
        );
    }
    window_handle.run_message_loop();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use winsp_service::testing::{empty_service, service_with_apps};

    fn sample_service() -> Service {
        service_with_apps(&[("calc", "Calculator"), ("cal", "Calendar")])
    }

    fn type_query(state: &mut UiState, service: &Service, query: &str) {
        for c in query.chars() {
            state.insert_char(c);
        }
        state.settle(service);
    }

    #[test]
    fn insert_char_appends_and_refreshes() {
        let service = empty_service();
        let mut state = UiState::new(&service);

        type_query(&mut state, &service, "calc");

        assert_eq!(state.query(), "calc");
        assert!(state.results().is_empty());
    }

    #[test]
    fn insert_text_appends_pasted_text_and_refreshes() {
        let service = empty_service();
        let mut state = UiState::new(&service);

        state.insert_text("calc");
        state.settle(&service);

        assert_eq!(state.query(), "calc");
    }

    #[test]
    fn insert_text_strips_control_characters() {
        let service = empty_service();
        let mut state = UiState::new(&service);

        state.insert_text("ca\nlc\t");
        state.settle(&service);

        assert_eq!(state.query(), "calc");
    }

    #[test]
    fn insert_text_with_only_control_characters_does_not_mark_state_stale() {
        let service = empty_service();
        let mut state = UiState::new(&service);
        state.settle(&service);

        state.insert_text("\n\t");

        assert!(!state.settle(&service), "an empty paste should not refresh");
    }

    #[test]
    fn backspace_and_clear_query_refresh_results() {
        let service = empty_service();
        let mut state = UiState::new(&service);
        type_query(&mut state, &service, "calc");

        state.backspace();
        state.settle(&service);
        assert_eq!(state.query(), "cal");

        state.clear_query();
        state.settle(&service);
        assert_eq!(state.query(), "");
    }

    #[test]
    fn backspace_on_an_empty_query_is_a_no_op() {
        let service = empty_service();
        let mut state = UiState::new(&service);

        state.backspace();
        state.settle(&service);

        assert_eq!(state.query(), "");
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn clear_query_on_an_already_empty_query_still_resets_the_selection() {
        let service = empty_service();
        let mut state = UiState::new(&service);
        state.results = vec![
            ResultRow {
                title: "1".into(),
                subtitle: None,
                matched_char_indices: Vec::new(),
                icon: None,
            },
            ResultRow {
                title: "2".into(),
                subtitle: None,
                matched_char_indices: Vec::new(),
                icon: None,
            },
        ];
        state.selected_index = 1;

        state.clear_query();

        assert_eq!(state.query(), "");
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn an_edit_left_unsettled_is_searched_the_next_time_results_are_read() {
        let service = sample_service();
        let mut state = UiState::new(&service);

        for c in "calc".chars() {
            state.insert_char(c);
        }
        assert_eq!(state.query(), "calc");
        assert_eq!(
            state.results().len(),
            2,
            "still the results of the query before"
        );

        state.settle(&service);
        assert_eq!(
            state.results().first().map(|result| result.title.as_ref()),
            Some("Calculator")
        );

        for _ in 0..4 {
            state.backspace();
        }
        state.settle(&service);
        assert_eq!(state.query(), "");
        assert_eq!(state.results().len(), 2);
    }

    #[test]
    fn settling_without_a_pending_edit_keeps_the_selection() {
        let service = sample_service();
        let mut state = UiState::new(&service);
        type_query(&mut state, &service, "cal");

        state.select_next();
        let selected = state.selected_index();
        assert_eq!(selected, 1);

        state.settle(&service);

        assert_eq!(state.selected_index(), selected);
    }

    #[test]
    fn select_next_and_prev_wrap_around() {
        let service = empty_service();
        let mut state = UiState::new(&service);
        state.results = vec![
            ResultRow {
                title: "1".into(),
                subtitle: None,
                matched_char_indices: Vec::new(),
                icon: None,
            },
            ResultRow {
                title: "2".into(),
                subtitle: None,
                matched_char_indices: Vec::new(),
                icon: None,
            },
            ResultRow {
                title: "3".into(),
                subtitle: None,
                matched_char_indices: Vec::new(),
                icon: None,
            },
        ];
        let count = state.results().len();

        state.select_next();
        assert_eq!(state.selected_index(), 1);

        state.select_prev();
        assert_eq!(state.selected_index(), 0);

        state.select_prev();
        assert_eq!(state.selected_index(), count - 1);
    }
}
