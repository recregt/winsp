cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod builtin;
        mod discovery;
        mod lnk;
        mod start_menu;
        mod url;

        pub use discovery::list_installed_apps;
        pub(crate) use start_menu::start_menu_dirs;

        fn resolve_shortcut_target(path: &std::path::Path, ext_lower: &str) -> Option<String> {
            match ext_lower {
                "lnk" => lnk::resolve_lnk_target(path),
                "url" => url::resolve_url_target(path),
                _ => None,
            }
        }
    } else {
        mod fallback;

        pub use fallback::list_installed_apps;
    }
}
