//! ```text
//! pub mod catalog
//! pub mod catalog::launcher
//! pub fn catalog::launcher::run(&winsp_core::models::AppTarget) -> core::result::Result<(), alloc::string::String>
//! pub mod catalog::sources
//! pub mod catalog::sources::apps
//! pub fn catalog::sources::apps::list_installed_apps() -> alloc::vec::Vec<winsp_core::models::AppItem>
//! pub mod catalog::sources::settings
//! pub fn catalog::sources::settings::list_settings() -> alloc::vec::Vec<winsp_core::models::AppItem>
//! pub mod catalog::sources::watcher
//! pub fn catalog::sources::watcher::for_dirs<F>(&[std::path::PathBuf], F) -> notify::error::Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>> where F: core::ops::function::Fn() + core::marker::Send + 'static
//! pub fn catalog::sources::watcher::for_start_menu<F>(F) -> notify::error::Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>> where F: core::ops::function::Fn() + core::marker::Send + 'static
//! pub mod system
//! pub mod system::autostart
//! pub fn system::autostart::is_enabled() -> bool
//! pub fn system::autostart::set_enabled(bool)
//! pub mod system::registry
//! pub mod system::single_instance
//! pub struct system::single_instance::InstanceGuard(_)
//! impl core::ops::drop::Drop for system::single_instance::InstanceGuard
//! pub fn system::single_instance::InstanceGuard::drop(&mut self)
//! pub fn system::single_instance::acquire(&str, &str) -> core::option::Option<system::single_instance::InstanceGuard>
//! pub mod system::theme
//! pub fn system::theme::allow_dark_mode_for_app()
//! pub enum system::FontWeight
//! pub system::FontWeight::Normal
//! pub system::FontWeight::SemiBold
//! pub enum system::Key
//! pub system::Key::Back
//! pub system::Key::Down
//! pub system::Key::Enter
//! pub system::Key::Escape
//! pub system::Key::Other(u16)
//! pub system::Key::Space
//! pub system::Key::Tab
//! pub system::Key::Up
//! pub enum system::Message
//! pub system::Message::Char(char)
//! pub system::Message::Command(system::TrayCommand)
//! pub system::Message::Hotkey
//! pub system::Message::KeyDown(system::Key)
//! pub system::Message::KillFocus
//! pub system::Message::Paint
//! pub system::Message::TrayRightClick
//! #[repr(usize)] pub enum system::TrayCommand
//! pub system::TrayCommand::Autostart = 1002
//! pub system::TrayCommand::Exit = 1003
//! pub system::TrayCommand::Toggle = 1001
//! impl core::convert::TryFrom<usize> for system::TrayCommand
//! pub type system::TrayCommand::Error = num_enum::TryFromPrimitiveError<system::TrayCommand>
//! pub fn system::TrayCommand::try_from(usize) -> core::result::Result<Self, num_enum::TryFromPrimitiveError<Self>>
//! impl num_enum::TryFromPrimitive for system::TrayCommand
//! pub type system::TrayCommand::Error = num_enum::TryFromPrimitiveError<system::TrayCommand>
//! pub type system::TrayCommand::Primitive = usize
//! pub const system::TrayCommand::NAME: &'static str
//! pub fn system::TrayCommand::try_from_primitive(Self::Primitive) -> core::result::Result<Self, num_enum::TryFromPrimitiveError<Self>>
//! pub struct system::Canvas
//! impl system::Canvas
//! pub fn system::Canvas::draw_line(&self, (i32, i32), (i32, i32), system::Color)
//! pub fn system::Canvas::draw_text(&self, &str, system::Rect)
//! pub fn system::Canvas::draw_text_measured(&self, &str, system::Rect) -> i32
//! pub fn system::Canvas::fill_rect(&self, system::Rect, system::Color)
//! pub unsafe fn system::Canvas::from_raw(windows::Win32::Graphics::Gdi::HDC) -> Self
//! pub fn system::Canvas::select_font(&self, &system::Font) -> system::FontGuard
//! pub fn system::Canvas::set_text_color(&self, system::Color)
//! pub struct system::Color(pub u32)
//! pub struct system::Font
//! impl system::Font
//! pub fn system::Font::new(&str, i32, system::FontWeight) -> Self
//! impl core::marker::Send for system::Font
//! impl core::marker::Sync for system::Font
//! impl core::ops::drop::Drop for system::Font
//! pub fn system::Font::drop(&mut self)
//! pub struct system::FontGuard
//! impl core::ops::drop::Drop for system::FontGuard
//! pub fn system::FontGuard::drop(&mut self)
//! pub struct system::Hotkey
//! impl system::Hotkey
//! pub fn system::Hotkey::alt(system::Key) -> Self
//! pub struct system::Rect
//! pub system::Rect::bottom: i32
//! pub system::Rect::left: i32
//! pub system::Rect::right: i32
//! pub system::Rect::top: i32
//! impl core::convert::From<system::Rect> for windows::Win32::Foundation::RECT
//! pub fn windows::Win32::Foundation::RECT::from(system::Rect) -> windows::Win32::Foundation::RECT
//! pub struct system::WindowHandle
//! impl system::WindowHandle
//! pub fn system::WindowHandle::add_tray_icon(&self)
//! pub fn system::WindowHandle::center(&self, i32, i32)
//! pub fn system::WindowHandle::close(&self)
//! pub fn system::WindowHandle::create(&str, &str, i32, i32, fn(&system::WindowHandle, system::Message)) -> core::result::Result<Self, core::io::error::Error>
//! pub fn system::WindowHandle::enable_dark_mode(&self)
//! pub fn system::WindowHandle::hide(&self)
//! pub fn system::WindowHandle::hwnd(&self) -> windows::Win32::Foundation::HWND
//! pub fn system::WindowHandle::invalidate(&self)
//! pub fn system::WindowHandle::is_visible(&self) -> bool
//! pub fn system::WindowHandle::new(windows::Win32::Foundation::HWND) -> Self
//! pub fn system::WindowHandle::paint(&self, impl core::ops::function::FnOnce(&system::Canvas, system::Rect))
//! pub fn system::WindowHandle::resize(&self, i32, i32)
//! pub fn system::WindowHandle::run_message_loop(&self, system::Hotkey)
//! pub fn system::WindowHandle::show(&self)
//! pub fn system::WindowHandle::show_tray_menu(&self)
//! pub fn system::register_embedded_font(&'static [u8]) -> bool
//! ```

pub mod catalog;
#[cfg(windows)]
pub mod system;
