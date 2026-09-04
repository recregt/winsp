# Changelog

All notable changes to WinSP are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.3.0] - 2026-09-04

### Added
- Support installing actionlint locally by @recregt in #114
- Resolve PR numbers via GitHub API by @recregt in #115
- Gate release on CHANGELOG.md version by @recregt in #116
- Dark-themed tray menu via win32-darkmode by @recregt in #118
- Add toast notifications for errors by @recregt in #125
- Surface unreadable Start Menu folders by @recregt in #132
- Add settings persistence layer by @recregt in #136
- Model hotkey modifiers as a schema by @recregt in #139
- Add tray hotkey capture flow by @recregt in #140
- Customizable search bar position by @recregt in #143
- Auto-detect wine for pre-push tests by @recregt in #153
- Show application icons in search results by @recregt in #159
- Position window on the active monitor by @recregt in #218

### Fixed
- Recover commit types from squash merges by @recregt in #113
- Add timeout to download requests by @recregt in #114
- Link bot mentions to their app page by @recregt in #115
- Recover from COM apartment conflicts by @recregt in #128
- Detect uninstallers by target by @recregt in #130
- Dedupe fallback by path not stem by @recregt in #131
- Show readable launch error messages by @recregt in #133
- Scan Start Menu once at startup by @recregt in #134
- Discard trailing char via PeekMessage by @recregt in #140
- Skip save when position is unchanged by @recregt in #143
- Relaunch no longer shows a stale window by @recregt in #148
- Surface previously-silent failures by @recregt in #150
- Add explicit check=False for ruff PLW1510 by @recregt in #153
- Evict and destroy icons past a cache size cap by @recregt in #159
- Wrap cached HICON in a Drop-based RAII guard by @recregt in #160
- Sign GitHub release MSIX for Add-AppxPackage by @recregt in #190
- Break directory cycles via file identity by @recregt in #192
- Guard BeginPaint failure in paint() by @recregt in #197
- Clear stale unreadable dirs incrementally by @recregt in #198
- Resize window for results in show_fresh by @recregt in #202
- Apply catalog refresh on the UI thread by @recregt in #203
- Use scientific notation for tiny results by @recregt in #207
- Validate percentage parsing strictly by @recregt in #206
- Saturate search score arithmetic by @recregt in #205
- Wait cooldown instead of dropping reconcile by @recregt in #204
- Scope window handler/repaint state per HWND by @recregt in #209
- Re-add tray icon after Explorer restarts by @recregt in #210
- Fold query with matcher's own casefold by @recregt in #208
- Surface autostart, watcher, and tray failures by @recregt in #211
- Give calculator score a fixed ceiling by @recregt in #215
- Flush clipboard after setting content by @recregt in #214
- Combine name and keyword match scores by @recregt in #216
- Gate test watch dir override to debug by @recregt in #217
- Use work area and handle GetCursorPos failure by @recregt in #218
- Close winsp-windows leaks and hazards by @recregt in #221
- Preserve name-match indices on keyword win by @recregt in #234
- Stop cliff splitting squash-merge bodies by @recregt in #237
- Collapse cliff entries to one per PR/type by @recregt in #238

### Changed
- Dedupe install scripts into a package by @recregt in #114
- Group verify scripts under a package by @recregt in #116
- Make read_dword safe and crate-private by @recregt in #118
- Move message dispatch into WindowHandle by @recregt in #119
- Add test-support feature to windows by @recregt in #120
- Consolidate windows-sys into windows by @recregt in #121
- Promote window to top-level module by @recregt in #123
- Rename WindowHandle to Window by @recregt in #124
- Clean up console output by @recregt in #127
- Reuse one COM shell-link object per scan by @recregt in #128
- Incrementally update Start Menu catalog by @recregt in #129
- Consolidate cfg(windows) via cfg_if by @recregt in #134
- Drop explanatory comments from #136/#139 by @recregt in #141
- Rename request_show for accuracy by @recregt in #148
- Drop unnecessary cfg guard by @recregt in #150
- Make windows crate fully windows-gated by @recregt in #153
- Drop dead startup println! calls by @recregt in #156
- Let-else for single instance check by @recregt in #157
- Use let-chains, drop nesting by @recregt in #158
- Use native thread pool for icons by @recregt in #160
- Match existing house style for FFI code by @recregt in #192
- Use MaybeUninit for FFI out-params by @recregt in #197
- Split window modules by responsibility by @recregt in #221
- Rename AppTarget to LaunchTarget by @recregt in #228
- Restructure catalog module by @recregt in #230
- Move UI side effects out of system layer by @recregt in #231
- Encapsulate AppItem, tidy search API by @recregt in #233
- Avoid re-sorting sorted search results by @recregt in #234
- Flatten core module layout by @recregt in #235
- Cut per-keystroke allocations by @recregt in #236


**Full Changelog**: https://github.com/recregt/winsp/compare/v0.2.1...v0.3.0

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
- Fixed 10,000-item index, plus a full_session case for the whole typed by [@codspeed-hq[bot]](https://github.com/apps/codspeed-hq) in #96
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

- Update GitHub Actions to v7 by [@renovate[bot]](https://github.com/apps/renovate) in #2
- Change lefthook install destination to cargo bin directory by @recregt
- Adapt to windows-sys 0.61 HWND pointer type changes by @recregt in #6
- Add minimal pull request template by @recregt in #7


