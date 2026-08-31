use std::sync::Mutex;
use tauri::Manager;
use winsp_core::models::{AppItem, AppTarget, SearchResult, SearchResultKind};
use winsp_core::search::Engine;
use winsp_windows::catalog::launcher::run as run_target;

struct AppTauriState {
    index: Engine,
    results: Vec<SearchResult>,
}

#[derive(serde::Serialize)]
struct SearchResultDto {
    title: String,
    subtitle: Option<String>,
    matched_indices: Vec<usize>,
    kind: &'static str,
}

impl From<&SearchResult> for SearchResultDto {
    fn from(r: &SearchResult) -> Self {
        let kind = match &r.kind {
            SearchResultKind::App(_) => "app",
            SearchResultKind::Calculation { .. } => "calculation",
            SearchResultKind::WebSearch { .. } => "web_search",
            SearchResultKind::SystemCommand { .. } => "system_command",
        };
        Self {
            title: r.title.to_string(),
            subtitle: r.subtitle.as_ref().map(|s| s.to_string()),
            matched_indices: r.matched_indices.clone(),
            kind,
        }
    }
}

fn populate_search_index() -> Engine {
    let mut index = Engine::new();
    let mut all_items: Vec<AppItem> = Vec::new();

    all_items.extend(winsp_windows::catalog::sources::apps::list_installed_apps());
    all_items.extend(winsp_windows::catalog::sources::settings::list_settings());

    index.set_items(all_items);
    index
}

#[tauri::command]
fn search(query: String, state: tauri::State<Mutex<AppTauriState>>) -> Vec<SearchResultDto> {
    let mut state = state.lock().unwrap();
    let results = state.index.search(&query, 6);
    let dtos = results.iter().map(SearchResultDto::from).collect();
    state.results = results;
    dtos
}

#[tauri::command]
fn launch(index: usize, state: tauri::State<Mutex<AppTauriState>>) -> Result<(), String> {
    let state = state.lock().unwrap();
    let selected = state
        .results
        .get(index)
        .ok_or_else(|| "invalid result index".to_string())?;
    match &selected.kind {
        SearchResultKind::App(item) => run_target(&item.target),
        SearchResultKind::Calculation { .. } => Ok(()),
        SearchResultKind::WebSearch { url, .. } => run_target(&AppTarget::Path(url.clone())),
        SearchResultKind::SystemCommand { command, .. } => {
            run_target(&AppTarget::SystemCommand(command.clone()))
        }
    }
}

#[cfg(target_os = "windows")]
fn disable_window_transitions(window: &tauri::WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute};

    if let Ok(hwnd) = window.hwnd() {
        let disable: i32 = 1;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &disable as *const i32 as *const std::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let is_visible = window.is_visible().unwrap_or(false);
    if is_visible {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let index = populate_search_index();
    let initial_results = index.search("", 6);
    let state = AppTauriState {
        index,
        results: initial_results,
    };

    tauri::Builder::default()
        .manage(Mutex::new(state))
        .setup(|app| {
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                disable_window_transitions(&window);
            }

            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcuts(["alt+space"])?
                        .with_handler(|app, shortcut, event| {
                            if event.state == ShortcutState::Pressed
                                && shortcut.matches(Modifiers::ALT, Code::Space)
                            {
                                toggle_main_window(app);
                            }
                        })
                        .build(),
                )?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![search, launch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
