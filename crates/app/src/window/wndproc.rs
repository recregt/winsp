use winsp_windows::window::{Key, Message, TrayCommand, Window};

use super::APP_STATE;
use super::geometry::{resize_window_for_results, toggle_visibility};
use super::render::render_ui;

pub(super) fn handle_message(window: &Window, message: Message) {
    match message {
        Message::Hotkey => toggle_visibility(window),
        Message::TrayRightClick => window.show_tray_menu(),
        Message::Command(cmd) => match cmd {
            TrayCommand::Toggle => toggle_visibility(window),
            TrayCommand::Autostart => {
                use winsp_windows::system::autostart;
                autostart::set_enabled(!autostart::is_enabled());
            }
            TrayCommand::Exit => window.close(),
        },
        Message::KillFocus => window.hide(),
        Message::Char(c) => {
            if let Some(state_arc) = APP_STATE.get() {
                if let Ok(mut state) = state_arc.lock() {
                    state.insert_char(c);
                    resize_window_for_results(window, state.results.len());
                }
            }
            window.invalidate();
        }
        Message::KeyDown(key) => {
            if let Some(state_arc) = APP_STATE.get() {
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
                            let _ = state.execute_selected();
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
        }
        Message::Paint => window.paint(render_ui),
    }
}
