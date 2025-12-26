# Changelog

## [0.3.2] - 2025-12-26

### Internal
- **Terminal backend update**: Updated `ratatui` from `0.29.0` to `0.30.0`


---


## [0.3.1] - 2025-12-26

### Changed
- **README image scaling**: Fixed image sizing so screenshots render correctly for github and crates.io


---


## [0.3.0] - 2025-12-26

The first release under the name of `runa` :D
This is a big one.
`runa` is now officialy a file browser. v0.3.0 is still the beginning, there will be more good things to come...

### Added
- **File actions**: Create files, create directories, copy (yank) and paste files/directories, delete files/directories, rename files/directories
- **Filtering**: Filter through the current directory to only select what you need.
- **Customizable widgets**: Customize all the popup widgets, Customize the Multiselect marker, Customize the positions of the widgets, Customize the coloring of the widgets.
- **Multiselect**: Select and act on multiple files at once.
- **Customizable keybindings for file actions**: Improved keymapping to enable modifiers for all actions.
- **Status line**: See the applied filter, see the amount of files being yanked/copied
- **New [theme.widgets] config section.** for widget styling. Customize the entire positions and size if you desire.

### Fixed
- **Path info**: Path info is now using correct padding.

### Internal
- **Tons of refactors**: Added new modoules: app/actions, app/handlers to help seperate AppState logic.
- **Keymapping**: Removed keycode_to_str for usage of Keymap struct, that maps all the keys correctly and more efficient.
- **Widget deserialization**: Implemented a custom deserialization method for widgets to ease config verbosity.

And much more...

I wish you all a Merry Christmas!


---


## [0.2.14] - 2025-12-23

### Changed
- Finalized the change from **runner** to **runa**
- All project references now use **runa**, **runa-tui** and the binary **rn**

---

## [0.2.13] - 2025-12-23

## Changed
- Renamed the project from **runner** to **runa**
- Crate is now published as **runa-tui** (previously `runner-tui`)
- Deprecated the `runner-tui` crate on crates.io; please use `runa-tui` for new installations

---

## [0.2.12] - 2025-12-23
### Added
- **Empty Directories indicator**: Now shows `[Empty]` when a directory is empty, in the main and preview pane.

### Fixed
- **Entry coloring**: Fixed/Added a entry coloring fallback to `[theme.entry]` instead of internal default.


## [v0.2.11] - 2025-12-22

### Changed
- **Default config**: Changed the `--init` config to generate a default `runa signature` theme instead of internal defaults. Note: This is the first of many themes.

### Fixed
- **UI Stability**: Hardened the padding logic with a match guard to prevent invalid or negative spacing values.


## [v0.2.10] - 2025-12-22

### Fixed
- Selection background and foreground colors for each pane now renders correctly.
- Fixed a bug where the underline background in `[theme.underline]` would falsely overide the selection background even if false. Now correctly respects the `preview_underline_color` toggle.

### Added
- **Theme Overides**: Implemented a Global-to-Local overide system, where panes can inherit global selection styles from `[theme.selection]` or define their own.
- **Entry Padding**: Added `entry_padding` configuration to allow customization of padding between entries and the pane edge.
- **Navigation Wrapping**: Navigating past the last entry now wraps back to the top.
- **Expaned Theme Support**: Can now use 3 digit HEX colors as well.

### Changed
- **Parent pane**: Renamed the former `Origin` pane to `Parent` pane
- **Display layout**: Changed how the configuration holds the pane / ratio layouts. Now inside `[display.layout]`!
- **Defaults**: Changed the defaults of `Parent` (former `Origin`) to be enabled by default. Also the init config now comments out all the defaults except some few
- **Clean configuration init**: The `--init` command now generates a cleaner `runa.toml` by commentig out most internal defaults.

### Internal
- Optimized "Global-to-Local" theme engine to correctly resolve color overides and inheritance, improving runtime overhead.



---




## [v0.2.9] - 2025-12-22

### Fixed
- Fixed being unable to open a directory with a selected editor from the runa.toml config


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
- **Preview underline default**: Enabled preview underline to be enabled by default in the runa.toml.

## Internal
- **UI refactor**: Cleaned up the render function in ui.rs to improve readability and context.


---


## [v0.2.6] - 2025-12-22
### Fixed
- **File/Directory preview**: Fixed issue where preview did not correctly render when pane sizes where inconsistent. Now using `unicode-width` to correctly calculate pane width.
- **Pane ratios**: Pane ratios are now correctly calculated and will always internally calculate to 100%, meaning its not needed to always have 100% ratio in the runa.toml config.

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
