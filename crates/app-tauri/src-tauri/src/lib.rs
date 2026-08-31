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

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use winsp_windows::system::autostart;

    let toggle_item = MenuItemBuilder::with_id("toggle", "Toggle Search").build(app)?;
    let autostart_item = CheckMenuItemBuilder::with_id("autostart", "Start with Windows")
        .checked(autostart::is_enabled())
        .build(app)?;
    let exit_item = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&toggle_item)
        .item(&autostart_item)
        .separator()
        .item(&exit_item)
        .build()?;

    let autostart_item_for_event = autostart_item.clone();
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle" => toggle_main_window(app),
            "autostart" => {
                let enabled = !autostart::is_enabled();
                autostart::set_enabled(enabled);
                let _ = autostart_item_for_event.set_checked(enabled);
            }
            "exit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
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
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
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

                setup_tray(app.handle())?;
            }

            #[cfg(windows)]
            {
                let app_handle = app.handle().clone();
                let watcher = winsp_windows::catalog::sources::watcher::for_start_menu(move || {
                    let index = populate_search_index();
                    if let Some(state) = app_handle.try_state::<Mutex<AppTauriState>>() {
                        if let Ok(mut state) = state.lock() {
                            state.index = index;
                        }
                    }
                });
                if let Ok(watcher) = watcher {
                    app.manage(watcher);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![search, launch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
