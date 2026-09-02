mod builtin;
mod catalog;
mod discovery;
mod lnk;
mod start_menu;
mod url;

pub(crate) use builtin::resolve_system_exe;
pub use catalog::StartMenuCatalog;
pub use discovery::{list_installed_apps, merge_with_built_ins};
pub(crate) use start_menu::start_menu_dirs;

fn resolve_shortcut_target(path: &std::path::Path, ext_lower: &str) -> Option<String> {
    match ext_lower {
        "lnk" => lnk::resolve_lnk_target(path),
        "url" => url::resolve_url_target(path),
        _ => None,
    }
}
