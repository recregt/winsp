cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod com;
        mod lnk;
        mod paths;
        mod scan;
        mod url;

        pub use paths::start_menu_dirs;
        pub use scan::enumerate_installed_apps;
    } else {
        mod fallback;

        pub use fallback::enumerate_installed_apps;
    }
}
