# Changelog

All notable changes to WinSP are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.1] - 2026-08-31

### Added
- Expand imperative-mood word list by @recregt in #86
- Shunting-yard math engine with logos by @recregt in #94
- Highlight matched characters in results by @recregt in #105

### Fixed
- Make GitHub release creation idempotent by @recregt in #79
- Route workflow_dispatch tag input through env, not template expansion by @recregt in #80
- Raise msstore publish upload timeout from the 100s default to 600s by @recregt in #81
- Use --uploadTimeout, single dash isn't a valid long option by @recregt in #82
- Use named groups, avoid IndexError by @recregt in #86
- Strip comments, require blank line by @recregt in #86
- Treat whitespace-only line as blank by @recregt in #86
- Always read files as utf-8 by @recregt in #86
- Close trailing-period and auto gaps by @recregt in #86
- Don't count Cargo.lock as a scope by @recregt in #86
- Don't show window on startup by @recregt in #87
- Decode UTF-16 surrogate pairs in WM_CHAR by @recregt in #88
- Dedupe start menu apps by name by @recregt in #90
- Dedupe by resolved shortcut target by @recregt in #90
- Fix COM lifecycle, dedupe identity by @recregt in #90
- Fail closed on COM init failure by @recregt in #90
- Needle case-folding and frecency parity by @recregt in #93
- Cap query length, rename to math by @recregt in #93
- Lowercase sci notation, tan domain by @recregt in #101
- Clamp segment rects, add highlight tests by @recregt in #105
- Remove comments, fix too-many-arguments by @recregt in #106
- Fixed 10,000-item index, plus a full_session case for the whole typed by @codspeed-hq[bot] in #96
- Harden lefthook installer robustness by @recregt in #111
- Avoid tmp file collision on install by @recregt in #111

### Changed
- Extract msstore auth/publish composite actions by @recregt in #83
- Split window.rs into window/ by @recregt in #88
- Move window logic into window/ by @recregt in #88
- Split shell_apps into apps/ by @recregt in #90
- Nucleo matching, Arc<str> results, fat LTO by @recregt in #93
- Split search orchestrator, index, math by @recregt in #94
- Fold search() into a SearchIndex method by @recregt in #94
- Qualify public API paths by @recregt in #95
- Rename indexer crate to windows by @recregt in #107
- Harden windows crate public API by @recregt in #107
- Unify windows crate into sources by @recregt in #107
- Unflatten sources public API by @recregt in #107
- Move OS integration into windows crate by @recregt in #108
- Use cfg_if in launcher by @recregt in #108


**Full Changelog**: https://github.com/recregt/winsp/compare/v0.2.0...v0.2.1

## [0.2.0] - 2026-08-29

### Added
- Add single instance lock to prevent duplicate processes
- Add test-mode watch target and structured reindex logging by @recregt in #20
- Hide console window and add system tray icon by @recregt in #25
- Add autostart toggle to tray menu by @recregt in #26
- Add MSIX manifest and CI packaging step by @recregt in #28
- Use Windows.ApplicationModel.StartupTask for autostart when packaged by @recregt in #36
- Go MSIX-only, drop standalone exe distribution and unpackaged autostart fallback by @recregt in #62
- Add Store badge to README and wire up direct Microsoft Store publishing on release by @recregt in #64

### Fixed
- Keep MSIX logo source out of packaged Assets dir by @recregt in #30
- Accept semver pre-release versions, clean up staging dir, and verify release tag matches Cargo version by @recregt in #47
- Embed a real tray/window icon so it renders in the MSIX-packaged environment by @recregt in #52
- Statically link the MSVC CRT so the release binary needs no VC++ Redistributable by @recregt in #54
- Pin all third-party actions to commit hashes and harden checkout/cache steps by @recregt in #60
- Refresh taiki-e/install-action pins to their current commits by @recregt in #70
- Surface msstore stderr output in store-auth-check by @recregt in #72
- Surface verbose output for msstore submission get by @recregt in #73
- Use an obvious placeholder for AppxManifest.xml version by @recregt in #77

### Changed
- Add criterion benchmarks for search index scaling
- Extract MSIX packaging into a shared composite action by @recregt in #47


**Full Changelog**: https://github.com/recregt/winsp/compare/v0.1.0...v0.2.0

## [0.1.0] - 2026-08-25

- Update GitHub Actions to v7 by @renovate[bot] in #2
- Change lefthook install destination to cargo bin directory by @recregt
- Adapt to windows-sys 0.61 HWND pointer type changes by @recregt in #6
- Add minimal pull request template by @recregt in #7


