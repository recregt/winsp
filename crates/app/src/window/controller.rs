use winsp_windows::window::{Anchor, Key, MenuItem, Modifiers, Window, WindowEvent};

use crate::config::WindowPosition;
use crate::state::ExecuteOutcome;

use super::hotkey::{self, CaptureOutcome, CommitResult};
use super::layout::{result_list_height, to_anchor};
use super::view::render_ui;
use super::{APP_STATE, CATALOG_READY_EVENT, SETTINGS};

const CMD_TOGGLE: usize = 1001;
const CMD_AUTOSTART: usize = 1002;
const CMD_EXIT: usize = 1003;
const CMD_CHANGE_HOTKEY: usize = 1004;
const CMD_POSITION_TOP: usize = 1005;
const CMD_POSITION_CENTER: usize = 1006;

const STARTUP_TASK_ID: &str = "WinSPStartup";

pub(super) fn handle_event(window: &Window, event: WindowEvent) {
    match event {
        WindowEvent::Hotkey => toggle_visibility(window),
        WindowEvent::ShowRequest => show_fresh(window),
        WindowEvent::User(id) => {
            if id == CATALOG_READY_EVENT {
                apply_catalog_ready(window);
            }
        }
        WindowEvent::TaskbarRestarted => {
            window.add_tray_icon();
        }
        WindowEvent::TrayRightClick => window.show_tray_menu(&build_tray_menu()),
        WindowEvent::TrayCommand(id) => match id {
            CMD_TOGGLE => toggle_visibility(window),
            CMD_AUTOSTART => {
                use winsp_windows::system::autostart;
                let enabled = autostart::is_enabled(STARTUP_TASK_ID);
                if let Err(error) = autostart::set_enabled(STARTUP_TASK_ID, !enabled) {
                    winsp_windows::system::toast::show("WinSP", &error.to_string());
                }
            }
            CMD_CHANGE_HOTKEY => {
                if let Some(state_arc) = APP_STATE.get()
                    && let Ok(mut state) = state_arc.lock()
                {
                    state.capturing_hotkey = true;
                }
                begin_hotkey_capture(window);
            }
            CMD_POSITION_TOP => set_position(window, WindowPosition::Top),
            CMD_POSITION_CENTER => set_position(window, WindowPosition::Center),
            CMD_EXIT => window.close(),
            _ => {}
        },
        WindowEvent::FocusLost => {
            let was_capturing = APP_STATE
                .get()
                .and_then(|state_arc| state_arc.lock().ok().map(|state| state.capturing_hotkey))
                .unwrap_or(false);
            if was_capturing {
                end_capture(window);
            } else {
                window.hide();
            }
        }
        WindowEvent::Char(c) => {
            if let Some(state_arc) = APP_STATE.get() {
                let mut results_count = None;
                if let Ok(mut state) = state_arc.lock() {
                    if !state.capturing_hotkey {
                        state.insert_char(c);
                        results_count = Some(state.results.len());
                    }
                }
                if let Some(count) = results_count {
                    resize_window_for_results(window, count);
                }
            }
            window.invalidate();
        }
        WindowEvent::KeyDown { key, modifiers } => {
            let Some(state_arc) = APP_STATE.get() else {
                return;
            };
            let capturing = state_arc
                .lock()
                .map(|state| state.capturing_hotkey)
                .unwrap_or(false);
            if capturing {
                handle_capture_key(window, key, modifiers);
                return;
            }

            let mut should_resize = false;
            let mut results_count = 0;
            let mut should_hide = false;

            if let Ok(mut state) = state_arc.lock() {
                match key {
                    Key::Back => {
                        state.backspace();
                        should_resize = true;
                        results_count = state.results.len();
                    }
                    Key::Down | Key::Tab => {
                        state.select_next();
                    }
                    Key::Up => {
                        state.select_prev();
                    }
                    Key::Enter => {
                        match state.execute_selected() {
                            ExecuteOutcome::Copy(result) => {
                                winsp_windows::system::clipboard::copy(&result);
                                winsp_windows::system::toast::show(
                                    "WinSP",
                                    &format!("Copied: {result}"),
                                );
                            }
                            ExecuteOutcome::Launch(target) => {
                                std::thread::spawn(move || {
                                    if let Err(error) = winsp_windows::shell::run(&target) {
                                        winsp_windows::system::toast::show("WinSP", &error);
                                    }
                                });
                            }
                            ExecuteOutcome::None => {}
                        }
                        should_hide = true;
                    }
                    Key::Escape => {
                        should_hide = true;
                    }
                    _ => {}
                }
            }

            if should_hide {
                window.hide();
            } else {
                if should_resize {
                    resize_window_for_results(window, results_count);
                }
                window.invalidate();
            }
        }
        WindowEvent::Redraw => window.paint(render_ui),
    }
}

pub(super) fn current_anchor() -> Anchor {
    SETTINGS
        .get()
        .and_then(|settings| settings.lock().ok().map(|settings| settings.position))
        .map(to_anchor)
        .unwrap_or(Anchor::Top)
}

fn build_tray_menu() -> Vec<MenuItem<'static>> {
    let position = current_anchor();
    vec![
        MenuItem {
            id: CMD_TOGGLE,
            label: "Toggle Search",
            checked: false,
        },
        MenuItem {
            id: CMD_AUTOSTART,
            label: "Start with Windows",
            checked: winsp_windows::system::autostart::is_enabled(STARTUP_TASK_ID),
        },
        MenuItem {
            id: CMD_CHANGE_HOTKEY,
            label: "Change Hotkey…",
            checked: false,
        },
        MenuItem {
            id: CMD_POSITION_TOP,
            label: "Position: Top",
            checked: position == Anchor::Top,
        },
        MenuItem {
            id: CMD_POSITION_CENTER,
            label: "Position: Center",
            checked: position == Anchor::Center,
        },
        MenuItem {
            id: CMD_EXIT,
            label: "Exit",
            checked: false,
        },
    ]
}

pub(super) fn show_fresh(handle: &Window) {
    handle.center(
        super::WINDOW_WIDTH,
        super::SEARCH_BAR_HEIGHT,
        current_anchor(),
    );
    if let Some(state_arc) = APP_STATE.get()
        && let Ok(mut state) = state_arc.lock()
    {
        state.clear_query();
        resize_window_for_results(handle, state.results.len());
    }
    if let Some(tx) = super::RECONCILE_TX.get() {
        if tx.send(()).is_err() {
            crate::catalog_sync::notify_reconcile_channel_broken();
        }
    }
    handle.show();
    handle.invalidate();
}

pub(super) fn toggle_visibility(handle: &Window) {
    if handle.is_visible() {
        handle.hide();
    } else {
        show_fresh(handle);
    }
}

pub(super) fn resize_window_for_results(handle: &Window, results_count: usize) {
    handle.resize(super::WINDOW_WIDTH, result_list_height(results_count));
}

pub(super) fn begin_hotkey_capture(handle: &Window) {
    handle.center(
        super::WINDOW_WIDTH,
        super::SEARCH_BAR_HEIGHT,
        current_anchor(),
    );
    resize_window_for_results(handle, 0);
    handle.show();
    handle.invalidate();
}

fn apply_catalog_ready(window: &Window) {
    let Some(index) = crate::catalog_sync::take_pending_index() else {
        return;
    };
    let Some(state_arc) = APP_STATE.get() else {
        return;
    };
    let Ok(mut state) = state_arc.lock() else {
        return;
    };
    state.index = index;
    state.refresh_results();
    let results_count = state.results.len();
    drop(state);

    resize_window_for_results(window, results_count);
    window.invalidate();
}

fn set_position(window: &Window, position: WindowPosition) {
    let Some(settings_mutex) = SETTINGS.get() else {
        return;
    };
    let Ok(mut settings) = settings_mutex.lock() else {
        return;
    };

    let previous = settings.position;
    if previous == position {
        return;
    }
    settings.position = position;
    if let Err(err) = settings.save() {
        settings.position = previous;
        winsp_windows::system::toast::show("WinSP", &format!("Failed to save position: {err}"));
        return;
    }
    drop(settings);

    if window.is_visible() {
        window.reposition(to_anchor(position));
    }
}

fn handle_capture_key(window: &Window, key: Key, modifiers: Modifiers) {
    match hotkey::evaluate(key, modifiers) {
        CaptureOutcome::Cancelled => end_capture(window),
        CaptureOutcome::Invalid => {}
        CaptureOutcome::Candidate(candidate) => {
            let Some(settings_mutex) = SETTINGS.get() else {
                return;
            };
            let Ok(mut settings) = settings_mutex.lock() else {
                return;
            };
            match hotkey::try_commit(window, &mut settings, candidate) {
                CommitResult::Committed => end_capture(window),
                CommitResult::Conflict => winsp_windows::system::toast::show(
                    "WinSP",
                    "That combination is already in use by another app.",
                ),
                CommitResult::PersistFailed(err) => winsp_windows::system::toast::show(
                    "WinSP",
                    &format!("Failed to save hotkey: {err}"),
                ),
            }
        }
    }
}

fn end_capture(window: &Window) {
    if let Some(state_arc) = APP_STATE.get()
        && let Ok(mut state) = state_arc.lock()
    {
        state.capturing_hotkey = false;
        state.clear_query();
    }
    window.discard_pending_char();
    window.hide();
}
