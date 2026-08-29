# Changelog

All notable changes to WinSP are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] - 2026-08-29

### Added
- Hide console window and add system tray icon (#25)
- Add autostart toggle to tray menu (#26)
- Add MSIX manifest and CI packaging step (#28)
- Use Windows.ApplicationModel.StartupTask for autostart when packaged (#36)
- Go MSIX-only, drop standalone exe distribution and unpackaged autostart fallback (#62)
- Add Store badge to README and wire up direct Microsoft Store publishing on release (#64)

### Fixed
- Keep MSIX logo source out of packaged Assets dir (#30)
- Embed a real tray/window icon so it renders in the MSIX-packaged environment (#52)
- Statically link the MSVC CRT so the release binary needs no VC++ Redistributable (#54)
- Use an obvious placeholder for AppxManifest.xml version (#77)

## [nightly] - 2026-08-26

### Added
- Add single instance lock to prevent duplicate processes
- Add test-mode watch target and structured reindex logging (#20)

### Changed
- Add criterion benchmarks for search index scaling

## [0.1.0] - 2026-08-25

### Fixed
- Change lefthook install destination to cargo bin directory
- Adapt to windows-sys 0.61 HWND pointer type changes

