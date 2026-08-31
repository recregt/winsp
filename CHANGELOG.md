# Changelog

All notable changes to WinSP are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.1] - 2026-08-31

### Added
- Shunting-yard math engine with logos (#94)
- Highlight matched characters in results (#105)

### Fixed
- Make GitHub release creation idempotent (#79)
- Raise msstore publish upload timeout from the 100s default to 600s (#81)
- Use --uploadTimeout, single dash isn't a valid long option (#82)
- Don't show window on startup (#87)
- Decode UTF-16 surrogate pairs in WM_CHAR (#88)
- Dedupe start menu apps by name (#90)
- Math correctness fixes (#101)
- Harden lefthook installer robustness
- Avoid tmp file collision on install

### Changed
- Extract msstore auth/publish composite actions (#83)
- Nucleo matching, Arc<str> results, fat LTO (#93)
- Qualify public API paths (#95)
- Harden windows crate public API (#107)
- Move OS integration into windows crate (#108)

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

