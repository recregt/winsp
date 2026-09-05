#![cfg(windows)]

mod controller;
mod hotkey;
mod view;

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};

use winsp_core::engine::Engine;
use winsp_core::models::{LaunchTarget, SearchResult, SearchResultKind};
use winsp_windows::window::{Hotkey, HotkeySlot, Key, Modifiers, Window};

use crate::config::Settings;
use crate::state::AppState;
use controller::handle_event;
use view::to_anchor;

const WINDOW_WIDTH: i32 = 680;
const SEARCH_BAR_HEIGHT: i32 = 64;
const ITEM_ROW_HEIGHT: i32 = 54;
const PADDING: i32 = 12;
pub(crate) const WINDOW_CLASS_NAME: &str = "WinSP_Spotlight_Window";
const CATALOG_READY_EVENT: u32 = 1;
const MAX_RESULTS: usize = 6;

static PENDING_CATALOG: OnceLock<Mutex<Option<Engine>>> = OnceLock::new();

pub(crate) fn deliver_catalog(index: Engine) {
    if let Ok(mut slot) = PENDING_CATALOG.get_or_init(|| Mutex::new(None)).lock() {
        *slot = Some(index);
    }
    winsp_windows::window::post_event(CATALOG_READY_EVENT);
}

fn take_pending_catalog() -> Option<Engine> {
    PENDING_CATALOG
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|mut guard| guard.take())
}

#[derive(Debug)]
struct UiState {
    query: String,
    results: Vec<SearchResult>,
    selected_index: usize,
    capturing_hotkey: bool,
}

enum ExecuteOutcome {
    Copy(String),
    Launch(LaunchTarget),
    None,
}

impl UiState {
    fn new(engine: &Engine) -> Self {
        Self {
            query: String::new(),
            results: engine.search("", MAX_RESULTS),
            selected_index: 0,
            capturing_hotkey: false,
        }
    }

    fn refresh_against(&mut self, engine: &Engine) {
        self.results = engine.search(&self.query, MAX_RESULTS);
        self.selected_index = 0;
    }

    fn query(&self) -> &str {
        &self.query
    }

    fn results(&self) -> &[SearchResult] {
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

    fn stop_capturing_hotkey(&mut self, engine: &Engine) {
        self.capturing_hotkey = false;
        self.clear_query(engine);
    }

    fn insert_char(&mut self, engine: &Engine, c: char) {
        self.query.push(c);
        self.refresh_against(engine);
    }

    fn backspace(&mut self, engine: &Engine) {
        self.query.pop();
        self.refresh_against(engine);
    }

    fn clear_query(&mut self, engine: &Engine) {
        self.query.clear();
        self.refresh_against(engine);
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

    fn execute_selected(&self) -> ExecuteOutcome {
        let Some(selected) = self.results.get(self.selected_index) else {
            return ExecuteOutcome::None;
        };
        match &selected.kind {
            SearchResultKind::App(item) => ExecuteOutcome::Launch(item.target().clone()),
            SearchResultKind::Calculation { result, .. } => {
                ExecuteOutcome::Copy(result.to_string())
            }
            SearchResultKind::WebSearch { url, .. } => {
                ExecuteOutcome::Launch(LaunchTarget::WebUrl(url.clone()))
            }
            SearchResultKind::SystemCommand { command, .. } => {
                ExecuteOutcome::Launch(LaunchTarget::Command(command.clone()))
            }
        }
    }
}

struct AppContext {
    app_state: Arc<Mutex<AppState>>,
    ui_state: Mutex<UiState>,
    settings: Mutex<Settings>,
    reconcile_tx: Sender<()>,
    active_hotkey_slot: Mutex<HotkeySlot>,
}

static APP: OnceLock<AppContext> = OnceLock::new();

fn context() -> Option<&'static AppContext> {
    APP.get()
}

pub(crate) fn run(app_state: Arc<Mutex<AppState>>, reconcile_tx: Sender<()>) -> Result<(), String> {
    let settings = Settings::load();
    if !crate::config::exists() {
        if let Err(err) = settings.save() {
            eprintln!("failed to save settings: {err}");
        }
    }

    let modifiers = Modifiers {
        ctrl: settings.hotkey.ctrl,
        shift: settings.hotkey.shift,
        alt: settings.hotkey.alt,
        win: settings.hotkey.win,
    };
    let hotkey = Hotkey::new(modifiers, Key::Other(settings.hotkey.vk));
    let anchor = to_anchor(settings.position);

    let ui_state = {
        let guard = app_state.lock().map_err(|_| "app state lock poisoned")?;
        UiState::new(guard.engine())
    };

    let _ = APP.set(AppContext {
        app_state,
        ui_state: Mutex::new(ui_state),
        settings: Mutex::new(settings),
        reconcile_tx,
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

    fn sample_state() -> UiState {
        UiState::new(&Engine::new())
    }

    #[test]
    fn insert_char_appends_and_refreshes() {
        let engine = Engine::new();
        let mut state = sample_state();

        state.insert_char(&engine, 'c');
        state.insert_char(&engine, 'a');
        state.insert_char(&engine, 'l');
        state.insert_char(&engine, 'c');

        assert_eq!(state.query(), "calc");
        assert!(state.results().is_empty());
    }

    #[test]
    fn backspace_and_clear_query_refresh_results() {
        let engine = Engine::new();
        let mut state = sample_state();
        state.insert_char(&engine, 'c');
        state.insert_char(&engine, 'a');
        state.insert_char(&engine, 'l');
        state.insert_char(&engine, 'c');

        state.backspace(&engine);
        assert_eq!(state.query(), "cal");

        state.clear_query(&engine);
        assert_eq!(state.query(), "");
    }

    #[test]
    fn execute_selected_with_no_matches_returns_none() {
        let engine = Engine::new();
        let mut state = sample_state();
        for c in "zzzznomatchzzzz".chars() {
            state.insert_char(&engine, c);
        }
        assert!(state.results().is_empty());
        assert!(matches!(state.execute_selected(), ExecuteOutcome::None));
    }

    #[test]
    fn select_next_and_prev_wrap_around() {
        let mut state = sample_state();
        state.results = vec![
            SearchResult::calculation("1".into(), "1".into()),
            SearchResult::calculation("2".into(), "2".into()),
            SearchResult::calculation("3".into(), "3".into()),
        ];
        let count = state.results().len();

        state.select_next();
        assert_eq!(state.selected_index(), 1);

        state.select_prev();
        assert_eq!(state.selected_index(), 0);

        state.select_prev();
        assert_eq!(state.selected_index(), count - 1);
    }

    #[test]
    fn execute_selected_on_a_calculation_returns_a_value_to_copy_without_launching() {
        let mut state = sample_state();
        state.results = vec![SearchResult::calculation("1+1".into(), "2".into())];

        match state.execute_selected() {
            ExecuteOutcome::Copy(result) => assert_eq!(result, "2"),
            _ => panic!("expected a Copy outcome, got a different variant instead"),
        }
    }

    #[test]
    fn execute_selected_on_an_app_returns_its_target_without_launching() {
        use winsp_core::models::AppItem;

        let mut state = sample_state();
        let item = AppItem::new("id", "Calculator", LaunchTarget::Path("calc.exe".into()));
        state.results = vec![SearchResult::from_app(
            std::sync::Arc::new(item),
            0,
            Vec::new(),
        )];

        assert!(matches!(
            state.execute_selected(),
            ExecuteOutcome::Launch(_)
        ));
    }
}
