mod canvas;
mod event;
mod icon_cache;
mod native;
mod tray;

pub use canvas::{Canvas, Color, Font, FontGuard, FontWeight, Rect, register_embedded_font};
pub use event::{Hotkey, HotkeySlot, Key, Modifiers, WindowEvent};
pub use icon_cache::{icon_for_path, mark_paint_started};
pub use native::{Anchor, NativeWindow, post_event};
pub use tray::MenuItem;

pub(crate) use native::WM_SHOW_REQUEST;

#[cfg(feature = "test-support")]
pub use canvas::testing;
