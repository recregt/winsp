#[cfg_attr(not(windows), allow(dead_code))]
mod lnk;
#[cfg_attr(not(windows), allow(dead_code))]
mod url;

#[cfg(windows)]
fn resolve_shortcut_target(path: &std::path::Path, ext_lower: &str) -> Option<String> {
    match ext_lower {
        "lnk" => lnk::resolve_lnk_target(path),
        "url" => url::resolve_url_target(path),
        _ => None,
    }
}

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod builtin;
        mod discovery;
        mod start_menu;

        pub use discovery::list_installed_apps;
        pub(crate) use start_menu::start_menu_dirs;
    } else {
        mod fallback;

        pub use fallback::list_installed_apps;
    }
}
