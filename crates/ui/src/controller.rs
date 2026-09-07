use winsp_windows::window::{Anchor, Key, MenuItem, Modifiers, Window, WindowEvent};

use super::hotkey::{self, CaptureOutcome, CommitResult};
use super::view::{self, render, result_list_height};
use super::{REFRESH_EVENT, context};

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
            if id == REFRESH_EVENT {
                refresh_from_service(window);
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
            CMD_POSITION_TOP => set_position(window, Anchor::Top),
            CMD_POSITION_CENTER => set_position(window, Anchor::Center),
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
            let more_typing = window.has_pending_keystroke();
            if let Some(ctx) = context() {
                let mut results_count = None;
                if let Ok(mut ui_state) = ctx.ui_state.lock() {
                    if !ui_state.is_capturing_hotkey() {
                        ui_state.insert_char(c);
                        if !more_typing && ui_state.settle(&ctx.service) {
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
            let more_typing = window.has_pending_keystroke();

            if let Ok(mut ui_state) = ctx.ui_state.lock() {
                let mut settled = false;
                match key {
                    Key::Back => {
                        ui_state.backspace();
                        settled = !more_typing && ui_state.settle(&ctx.service);
                    }
                    Key::Down | Key::Tab => {
                        settled = ui_state.settle(&ctx.service);
                        ui_state.select_next();
                    }
                    Key::Up => {
                        settled = ui_state.settle(&ctx.service);
                        ui_state.select_prev();
                    }
                    Key::Enter => {
                        ui_state.settle(&ctx.service);
                        ctx.service.activate(ui_state.selected_index());
                        should_hide = true;
                    }
                    Key::Escape => {
                        should_hide = true;
                    }
                    _ => {
                        settled = !more_typing && ui_state.settle(&ctx.service);
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
            let state = context().and_then(|ctx| {
                let mut ui_state = ctx.ui_state.lock().ok()?;
                ui_state.settle(&ctx.service);
                Some(ui_state)
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
        .map(|ctx| ctx.service.current_position())
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
        if let Ok(mut ui_state) = ctx.ui_state.lock() {
            ui_state.clear_query();
            ui_state.settle(&ctx.service);
            resize_window_for_results(handle, ui_state.results().len());
        }
        ctx.service.on_window_shown();
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

fn refresh_from_service(window: &Window) {
    let Some(ctx) = context() else {
        return;
    };
    let Ok(mut ui_state) = ctx.ui_state.lock() else {
        return;
    };
    ui_state.refresh_against(&ctx.service);
    let results_count = ui_state.results().len();
    drop(ui_state);

    resize_window_for_results(window, results_count);
    window.invalidate();
}

fn set_position(window: &Window, position: Anchor) {
    let Some(ctx) = context() else {
        return;
    };
    let current = ctx.service.current_position();
    let outcome = update_position(current, position, |p| ctx.service.change_position(p));

    apply_position_outcome(window, position, outcome);
}

fn apply_position_outcome(window: &Window, position: Anchor, outcome: Result<bool, String>) {
    match outcome {
        Ok(true) => {
            if window.is_visible() {
                window.reposition(position);
            }
        }
        Ok(false) => {}
        Err(err) => {
            winsp_windows::system::toast::show("WinSP", &format!("Failed to save position: {err}"));
        }
    }
}

fn update_position(
    current: Anchor,
    position: Anchor,
    persist: impl FnOnce(Anchor) -> Result<(), String>,
) -> Result<bool, String> {
    if current == position {
        return Ok(false);
    }
    persist(position)?;
    Ok(true)
}

fn handle_capture_key(window: &Window, key: Key, modifiers: Modifiers) {
    match hotkey::evaluate(key, modifiers) {
        CaptureOutcome::Cancelled => end_capture(window),
        CaptureOutcome::Invalid => {}
        CaptureOutcome::Candidate(modifiers, key) => {
            let Some(ctx) = context() else {
                return;
            };
            let current = ctx.service.current_hotkey();
            let Ok(mut active_slot) = ctx.active_hotkey_slot.lock() else {
                return;
            };
            match hotkey::try_commit(
                window,
                current,
                &mut active_slot,
                (modifiers, key),
                |m, k| ctx.service.change_hotkey(m, k),
            ) {
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
        && let Ok(mut ui_state) = ctx.ui_state.lock()
    {
        ui_state.stop_capturing_hotkey(&ctx.service);
    }
    window.discard_pending_char();
    window.hide();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unchanged_position_is_a_no_op() {
        let result = update_position(Anchor::Top, Anchor::Top, |_| {
            panic!("save should not be called when the position does not change")
        });

        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn a_changed_position_is_saved() {
        let result = update_position(Anchor::Top, Anchor::Center, |_| Ok(()));

        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn a_persist_failure_is_reported_as_an_error() {
        let result = update_position(
            Anchor::Top,
            Anchor::Center,
            |_| Err("disk full".to_string()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn a_visible_window_is_repositioned_when_the_position_changes() {
        let window = Window::create("WinSpTest_ApplyPositionVisible", "t", 40, 40, |_, _| {})
            .expect("window creation should succeed");
        window.center(40, 40, Anchor::Top);
        window.show();
        let before = window.outer_position();

        apply_position_outcome(&window, Anchor::Center, Ok(true));

        assert_ne!(
            window.outer_position(),
            before,
            "a visible window should move to the new anchor"
        );

        window.close();
    }

    #[test]
    fn a_hidden_window_is_not_repositioned_when_the_position_changes() {
        let window = Window::create("WinSpTest_ApplyPositionHidden", "t", 40, 40, |_, _| {})
            .expect("window creation should succeed");
        window.center(40, 40, Anchor::Top);
        assert!(!window.is_visible(), "the window should start out hidden");
        let before = window.outer_position();

        apply_position_outcome(&window, Anchor::Center, Ok(true));

        assert_eq!(
            window.outer_position(),
            before,
            "a hidden window must not be moved on screen"
        );

        window.close();
    }

    #[test]
    fn an_unchanged_outcome_leaves_a_visible_window_where_it_is() {
        let window = Window::create("WinSpTest_ApplyPositionNoOp", "t", 40, 40, |_, _| {})
            .expect("window creation should succeed");
        window.center(40, 40, Anchor::Top);
        window.show();
        let before = window.outer_position();

        apply_position_outcome(&window, Anchor::Center, Ok(false));

        assert_eq!(window.outer_position(), before);

        window.close();
    }

    #[test]
    fn a_persist_failure_reports_the_error_without_moving_the_window() {
        let window = Window::create("WinSpTest_ApplyPositionError", "t", 40, 40, |_, _| {})
            .expect("window creation should succeed");
        window.center(40, 40, Anchor::Top);
        window.show();
        let before = window.outer_position();

        apply_position_outcome(&window, Anchor::Center, Err("disk full".to_string()));

        assert_eq!(
            window.outer_position(),
            before,
            "a save failure must not move the window, only report the error"
        );

        window.close();
    }
}
