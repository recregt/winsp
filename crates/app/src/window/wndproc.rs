use winsp_windows::window::{Key, Message, Modifiers, TrayCommand, Window};

use super::geometry::{
    begin_hotkey_capture, current_anchor, resize_window_for_results, show_fresh, to_anchor,
    toggle_visibility,
};
use super::hotkey_capture::{self, CaptureOutcome, CommitResult};
use super::interaction;
use super::render::render_ui;
use super::settings::WindowPosition;
use super::{APP_STATE, SETTINGS};

pub(super) fn handle_message(window: &Window, message: Message) {
    match message {
        Message::Hotkey => toggle_visibility(window),
        Message::ShowRequest => show_fresh(window),
        Message::TrayRightClick => window.show_tray_menu(current_anchor()),
        Message::Command(cmd) => match cmd {
            TrayCommand::Toggle => toggle_visibility(window),
            TrayCommand::Autostart => {
                use winsp_windows::system::autostart;
                autostart::set_enabled(!autostart::is_enabled());
            }
            TrayCommand::ChangeHotkey => {
                if let Some(state_arc) = APP_STATE.get()
                    && let Ok(mut state) = state_arc.lock()
                {
                    state.capturing_hotkey = true;
                }
                begin_hotkey_capture(window);
            }
            TrayCommand::PositionTop => set_position(window, WindowPosition::Top),
            TrayCommand::PositionCenter => set_position(window, WindowPosition::Center),
            TrayCommand::Exit => window.close(),
        },
        Message::KillFocus => {
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
        Message::Char(c) => {
            if let Some(state_arc) = APP_STATE.get() {
                let mut results_count = None;
                if let Ok(mut state) = state_arc.lock() {
                    if !state.capturing_hotkey {
                        interaction::insert_char(&mut state, c);
                        results_count = Some(state.results.len());
                    }
                }
                if let Some(count) = results_count {
                    resize_window_for_results(window, count);
                }
            }
            window.invalidate();
        }
        Message::KeyDown(key, modifiers) => {
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
                        interaction::backspace(&mut state);
                        should_resize = true;
                        results_count = state.results.len();
                    }
                    Key::Down | Key::Tab => {
                        interaction::select_next(&mut state);
                    }
                    Key::Up => {
                        interaction::select_prev(&mut state);
                    }
                    Key::Enter => {
                        match interaction::execute_selected(&state) {
                            Ok(Some(result)) => {
                                winsp_windows::system::clipboard::copy(&result);
                                winsp_windows::system::toast::show(
                                    "WinSP",
                                    &format!("Copied: {result}"),
                                );
                            }
                            Ok(None) => {}
                            Err(error) => winsp_windows::system::toast::show("WinSP", &error),
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
        Message::Paint => window.paint(render_ui),
    }
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
    match hotkey_capture::evaluate(key, modifiers) {
        CaptureOutcome::Cancelled => end_capture(window),
        CaptureOutcome::Invalid => {}
        CaptureOutcome::Candidate(candidate) => {
            let Some(settings_mutex) = SETTINGS.get() else {
                return;
            };
            let Ok(mut settings) = settings_mutex.lock() else {
                return;
            };
            match hotkey_capture::try_commit(window, &mut settings, candidate) {
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
        interaction::clear_query(&mut state);
    }
    window.discard_pending_char();
    window.hide();
}
