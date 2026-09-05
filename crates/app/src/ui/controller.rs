use winsp_windows::window::{Anchor, Key, MenuItem, Modifiers, Window, WindowEvent};

use crate::config::WindowPosition;

use super::ExecuteOutcome;
use super::hotkey::{self, CaptureOutcome, CommitResult};
use super::view::{self, render, result_list_height, to_anchor};
use super::{CATALOG_READY_EVENT, context};

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
                if let Some(ctx) = context()
                    && let Ok(mut ui_state) = ctx.ui_state.lock()
                {
                    ui_state.start_capturing_hotkey();
                }
                begin_hotkey_capture(window);
            }
            CMD_POSITION_TOP => set_position(window, WindowPosition::Top),
            CMD_POSITION_CENTER => set_position(window, WindowPosition::Center),
            CMD_EXIT => window.close(),
            _ => {}
        },
        WindowEvent::FocusLost => {
            let was_capturing = context()
                .and_then(|ctx| {
                    ctx.ui_state
                        .lock()
                        .ok()
                        .map(|ui_state| ui_state.is_capturing_hotkey())
                })
                .unwrap_or(false);
            if was_capturing {
                end_capture(window);
            } else {
                window.hide();
            }
        }
        WindowEvent::Char(c) => {
            // Asked before the state is locked: peeking the queue lets the
            // window procedure run for messages other threads sent, and that
            // takes the very locks held here.
            let more_typing = window.has_pending_keystroke();
            if let Some(ctx) = context() {
                let mut results_count = None;
                if let (Ok(app_state), Ok(mut ui_state)) =
                    (ctx.app_state.lock(), ctx.ui_state.lock())
                {
                    if !ui_state.is_capturing_hotkey() {
                        ui_state.insert_char(c);
                        // The keystroke already queued behind this one replaces
                        // these results before anything can show them, so the
                        // search for them is left to it.
                        if !more_typing && ui_state.settle(app_state.engine()) {
                            results_count = Some(ui_state.results().len());
                        }
                    }
                }
                if let Some(count) = results_count {
                    resize_window_for_results(window, count);
                }
            }
            window.invalidate();
        }
        WindowEvent::KeyDown { key, modifiers } => {
            let Some(ctx) = context() else {
                return;
            };
            let capturing = ctx
                .ui_state
                .lock()
                .map(|ui_state| ui_state.is_capturing_hotkey())
                .unwrap_or(false);
            if capturing {
                handle_capture_key(window, key, modifiers);
                return;
            }

            let mut should_resize = false;
            let mut results_count = 0;
            let mut should_hide = false;
            // See the `Char` arm: peeked before the state is locked.
            let more_typing = window.has_pending_keystroke();

            if let (Ok(app_state), Ok(mut ui_state)) = (ctx.app_state.lock(), ctx.ui_state.lock()) {
                // A search a burst of typing deferred is owed by the first key
                // press that reads the results, acts on them, or simply ends
                // the burst.
                let mut settled = false;
                match key {
                    Key::Back => {
                        ui_state.backspace();
                        settled = !more_typing && ui_state.settle(app_state.engine());
                    }
                    Key::Down | Key::Tab => {
                        settled = ui_state.settle(app_state.engine());
                        ui_state.select_next();
                    }
                    Key::Up => {
                        settled = ui_state.settle(app_state.engine());
                        ui_state.select_prev();
                    }
                    Key::Enter => {
                        ui_state.settle(app_state.engine());
                        match ui_state.execute_selected() {
                            ExecuteOutcome::Copy(result) => {
                                winsp_windows::system::clipboard::copy(&result);
                                winsp_windows::system::toast::show(
                                    "WinSP",
                                    &format!("Copied: {result}"),
                                );
                            }
                            ExecuteOutcome::Launch(target) => {
                                let submitted =
                                    winsp_windows::system::threadpool::spawn_on_threadpool(
                                        move || {
                                            if let Err(error) = winsp_windows::shell::run(&target) {
                                                winsp_windows::system::toast::show("WinSP", &error);
                                            }
                                        },
                                    );
                                if !submitted {
                                    winsp_windows::system::toast::show(
                                        "WinSP",
                                        "Failed to launch: the system thread pool rejected the task.",
                                    );
                                }
                            }
                            ExecuteOutcome::None => {}
                        }
                        should_hide = true;
                    }
                    Key::Escape => {
                        should_hide = true;
                    }
                    _ => {
                        settled = !more_typing && ui_state.settle(app_state.engine());
                    }
                }

                if settled {
                    should_resize = true;
                    results_count = ui_state.results().len();
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
        WindowEvent::Redraw => window.paint(|canvas, rect| {
            // Painting is the last place a deferred search can still be owed,
            // so the results are settled before they are drawn.
            let state =
                context().and_then(|ctx| match (ctx.app_state.lock(), ctx.ui_state.lock()) {
                    (Ok(app_state), Ok(mut ui_state)) => {
                        ui_state.settle(app_state.engine());
                        Some(ui_state)
                    }
                    _ => None,
                });
            match state {
                Some(ui_state) => render(canvas, &ui_state, rect),
                None => canvas.fill_rect(rect, view::BACKGROUND_COLOR),
            }
        }),
    }
}

fn current_anchor() -> Anchor {
    context()
        .and_then(|ctx| ctx.settings.lock().ok().map(|settings| settings.position))
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

fn show_fresh(handle: &Window) {
    handle.center(
        super::WINDOW_WIDTH,
        super::SEARCH_BAR_HEIGHT,
        current_anchor(),
    );
    if let Some(ctx) = context() {
        if let (Ok(app_state), Ok(mut ui_state)) = (ctx.app_state.lock(), ctx.ui_state.lock()) {
            ui_state.clear_query();
            ui_state.settle(app_state.engine());
            resize_window_for_results(handle, ui_state.results().len());
        }
        if ctx.reconcile_tx.send(()).is_err() {
            crate::sync::notify_reconcile_channel_broken();
        }
    }
    handle.show();
    handle.invalidate();
}

fn toggle_visibility(handle: &Window) {
    if handle.is_visible() {
        handle.hide();
    } else {
        show_fresh(handle);
    }
}

fn resize_window_for_results(handle: &Window, results_count: usize) {
    handle.resize(super::WINDOW_WIDTH, result_list_height(results_count));
}

fn begin_hotkey_capture(handle: &Window) {
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
    let Some(index) = super::take_pending_catalog() else {
        return;
    };
    let Some(ctx) = context() else {
        return;
    };
    let Ok(mut app_state) = ctx.app_state.lock() else {
        return;
    };
    app_state.update_index(index);
    let Ok(mut ui_state) = ctx.ui_state.lock() else {
        return;
    };
    ui_state.refresh_against(app_state.engine());
    let results_count = ui_state.results().len();
    drop(ui_state);
    drop(app_state);

    resize_window_for_results(window, results_count);
    window.invalidate();
}

fn set_position(window: &Window, position: WindowPosition) {
    let Some(ctx) = context() else {
        return;
    };
    let Ok(mut settings) = ctx.settings.lock() else {
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
            let Some(ctx) = context() else {
                return;
            };
            let Ok(mut settings) = ctx.settings.lock() else {
                return;
            };
            let Ok(mut active_slot) = ctx.active_hotkey_slot.lock() else {
                return;
            };
            match hotkey::try_commit(window, &mut settings, &mut active_slot, candidate) {
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
    if let Some(ctx) = context()
        && let (Ok(app_state), Ok(mut ui_state)) = (ctx.app_state.lock(), ctx.ui_state.lock())
    {
        ui_state.stop_capturing_hotkey(app_state.engine());
    }
    window.discard_pending_char();
    window.hide();
}
