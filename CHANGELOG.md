# Changelog

## [v0.2.9] - 2025-12-22

### Fixed
- Fixed being unable to open a directory with a selected editor from the runner.toml config


---



## [v0.2.8] - 2025-12-22
## Changed
- Updated Cargo release profiles in `Cargo.toml` for optimized builds.



---


## [v0.2.7] - 2025-12-22
## Added
- **Preview underline theming**: Added `[theme.underline]` section to customize underline colors.
- **Preview color source toggle**: New `preview_underline_color` setting to choose between `[theme.underline]` or the standard `[theme.selection]` colors for the underline.

## Changed
- **Preview underline default**: Enabled preview underline to be enabled by default in the runner.toml.

## Internal
- **UI refactor**: Cleaned up the render function in ui.rs to improve readability and context.


---


## [v0.2.6] - 2025-12-22
### Fixed
- **File/Directory preview**: Fixed issue where preview did not correctly render when pane sizes where inconsistent. Now using `unicode-width` to correctly calculate pane width.
- **Pane ratios**: Pane ratios are now correctly calculated and will always internally calculate to 100%, meaning its not needed to always have 100% ratio in the runner.toml config.

### Added
- **Preview underline**: Toggle to enable a underline for the preview pane.
- **Directory colors**: Colors for directories are now independent of entry colors.

### Internal
- **Massive refactor**:  
    - Refactored `AppState` with modular app sub-modules: `app/nav.rs`, `app/parent.rs` and `app/preview.rs`
    - Refactored `Config` to make it more maintainable `config/themes`, `config/display`, `config/input`.
    - Separated UI functions from `terminal.rs` and added UI specific modules: `ui.rs`, `ui/panes.rs`, `ui/widgets.rs`
    - `worker.rs`: Ensured preview lines properly account for Unicode width and ignore control characters and tabs, preserving visual alignment in the TUI.
    - All worker responses and previews now produce strings of the exact pane_width, so TUI rendering remains stable regardless of file names or content length.

- **Tests**: Added formatting and worker tests

---

## [v0.2.5] - 2025-12-21
### Fixed
- **File Preview:** Fixed an issue where files without extensions and with very short names (≤ 3 characters, for example `ht`, `xy`) were incorrectly shown in the preview pane.
- **Preview Bleed/Race:** Hardened preview logic to ensure only the freshest preview request result is ever shown, preventing bleed from stale async worker responses during very fast navigation.

### Changed
- Reduced the default maximum number of previewed lines from 60 to 50 for better fit across a variety of terminal sizes. (Will make it configurable in following releases)
- Increased tick debounce from 15 to 75 milliseconds to reduce excessive preview requests during very fast navigation.

### Internal
- Improved worker-response by relying on request IDs to always honor the latest directory or preview pane update, eliminating edge cases with rapid async requests.
- Clarified and strengthened file preview and worker-response logic, including improved state handling and fewer UI edge case bugs.

---

## [v0.2.4] - 2025-12-20
### Changed
- Switched `always_show` config to use `HashSet<OsString>` for much faster and efficient lookups.
- Set default value of `always_show` to empty for a saner default config.

### Internal
- Refactored config module for better maintainability: introduced `RawConfig` for deserialization, and `Config` for runtime usage.

---

## What's New in v0.2.1 and v0.2.2

## UI improvements

- Improved pane customization by adding pane specific selection cache.
- Custom Path Styling (v0.2.2): Addded a dedicated theme.path configuration to customize the path at the top of the UI.
- Pane-Specific Selections (v0.2.2): Added selection foreground and background colors for each pane for more customization.

## Performance

- Switched to crossbeam-channel for better thread sync.
- Performance improvements by reducing event_loop string creation.
- Optimized always_show and other flags by using Atomics.
- Migrated text rendering to use Line and Span for better performance and future-proofing.

## Fixes
- Fixed preview sorting issue. Now shows the directories in the correct oder.
