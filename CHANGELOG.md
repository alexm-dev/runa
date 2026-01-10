# Changelog

## [0.5.1] - 2025-01-10

### Added
- **bat integration**: Added `bat` as an preview option to the internal preview. Can be set in `runa.toml` under `[display.preview_options]`
- **Clear Markers/Filters**: Added a `clear_filters` and `clear_markers` keybind option to clear either.
- **Icons**: Added optional nerd font icons. Is desabled by default.
- **Dynamic find widget**: Made the find widget more customizable by adding a `find_visible_results` option.

### Fixed
- **Parent pane**: Fixed the parent pane stale content update **and** optimized the parent directory selection being reset and redrawn after every directory change.
- **Directory Marker**: Fixed the `dir_marker` option to toggle the `/` for all panes.

### Breaking Changes
- **Dialog Size Configuration (`DialogSize::Custom`) now uses cells, not percentages:**
  - All custom dialog size values specified as arrays (e.g. `size = [70, 20]`) are now interpreted as exact cell (column/row) counts instead of percentages.
  - **Before:** `size = [70, 20]` meant 70% width and 20% height of the terminal area.
  - **Now:** `size = [70, 20]` means 70 columns wide and 20 rows tall.
  - Presets like `size = "small"` remain percentage-based.
  - **If you had custom dialog sizes or positions configured previously, please review your `runa.toml` and update values for your preferred appearance.**

### Changed
- **fd exclusions**: Switched to using a central exclusion map for `fd` searches. Added multiple default directories (e.g., `.git`, `node_modules`, `target`, `venv`, etc.) to reduce noise and improve relevance when searching. This also makes it easier to maintain and update the exclusion list.


### Internal
- **Core refactor**: Moved `formatter.rs` from utils/ to core/, since formatter now handles all core formatting logic of multiple functions.
- **Renamed file_manager.rs**: `file_manager.rs` is renamed to `rm.rs` to keep it more simple :)
- **Renamed find**: `find.rs` is renamed to `proc.rs` since it now handles all subprocesses spawned by runa.


---


## [0.5.0] - 2025-01-08

UI related additions and more.

### Added
- **Marker coloring**: Added a new `clipboard` config option to color the yanked and selected entries with a different marker color. Now its easier to indicate which entry was yanked.
- **Pane markers**: Added markers to panes. Now persistent markers will be shown/rendered in each preview and parent pane.
- **Toggle Marker jump**: Added `toggle_marker_jump` configuration option to theme.display to toggle if multiselect should jump to the first entry when the selection is at the last entry.
- **Instant Preview**: Added `instant_preview` configuration option to toggle between instant preview (requesting previews on every selection change) and pending previews. Off by default.
- **Empty filter**: Added a `No results for this filter` message when a filter applied shows no entries.

### Breaking Changes
- The keys `selection_fg` / `selection_bg` from `[theme.preview]` / `[theme.parent]` are now replaced by `selection.fg` / `selection.bg` or `[theme.preview.selection]` / `[theme.parent.selection]`
    - **New keys**:
    ```toml
    [theme.preview]
    fg = "default"
    bg = "default"
    selection.fg = "default"
    selection.bg = "default"

    ## Sub tables:

    [theme.preview.selection]
    fg = "default"
    bg = "default"
    ```
- If you set the selection colors for each pane, then these changes are breaking changes for you config.

### Fixed
- **Directory copy**: Fixed directories being unable to be copied.
- **Filter preview update**: Fixed preview data not being cleared when a filter shows no entries.
- **Config defaults:** `RawConfig` now explicitly sets default values for all fields, ensuring core options like `dirs_first` and `show_hidden` are enabled when configuration is omitted.

### Internal
- **Find**: Optimization for the find feature. Now lists the results faster.
- **Cargo update**: Dependencies updated to patch crates.
- **Worker thread API**: Worker thread spawnm functions are now private functions since worker threads are now spawned through `Workers` struct.
- **Preview constants**: Moved preview byte size and line count checks to the top of `core/workers.rs` for better clarity and maintainability.


---


## [0.4.0] - 2025-01-06

New feature update: The fuzzy finder.

### Added
- **Find function**: Added a new (fuzzy) find function to quickly search your directories and files.
    - Note: This feature optionally leverages the external tool [fd](https://github.com/sharkdp/fd) for high-performance recursive traversal.
- **Scrollable find results**: Added a scroll able find result list to the new find function to scroll through the results in the widget.
- **Persistent filters**: Made filters persist for each directory it is applied to.
- **Configurable maximum find results**: Added a new configuration to change the internal defaults for the new find function. (Internal default is 2000).
- **Internal themes**: Added internal themes which can be set in the `runa.toml` config.

### Fixed
- **Parent pane**: Fixed a stale parent content request after initial startup of runa.
- **Pane Requests**: Improved ID handling for pane requests, making request IDs more robust and reliable.

### Internal
- **Code file structure**: Refactored modules and sub-modules for better maintainability. In example: Moved **core** runa modules, like `file_manager`, `worker`, etc. into `core`.
- **Worker thread separation**: Separated worker threads to individual lines for better performance. FileOp, Nav, Find and IO have each their own worker now.
- **External binary detection**: Integrated which for graceful detection of the fd search backend, providing user notifications if the tool is missing.
- **Tests**: Added new `find` related tests.

---


## [0.3.10] - 2025-12-30

Quick configuration generation patch when `runa.toml` is generated with `rn --init-full`

### Fixed
- **Initial configuration**: Fixed full `runa.toml` initial config generation made by cli arg `rn --init-full`. Now generates the correct configuration and honors the internal defaults.


---


## [0.3.9] - 2025-12-30

### Added
- **Path string**: Display `~` for the home directory at the top of the TUI instead of the full absolute path.

### Changed
- **Internal default colors**: Changed internal default colors of `border`, `selection`, `titles` and `directory`.
**show_hidden**: Enabled `show_hidden` by default and is set to enabled internally.
- **Parent Pane**: Removed root indicator `/` in the parent pane, since `path_str` handles that in `ui.rs`.


---


### Changed

## [0.3.8] - 2025-12-30

Quick integration test patch for a more robust testing of runa with `cargo test`.

### Internal
**Testing**: Improved error handling and sandboxing of `nav_tests`, `utils_tests`, `worker_tests`, and `ui_tests`


---


## [0.3.7] - 2025-12-30

Quick patch to fix the cli message to show the correct cli args.

### Fixed
- **CLI message**: Fixed cli initial message to correctly show the cli args. `--init-minimal` to `--init-full`
- **Formatter** Fixed `formatter.rs` warning showing unused import for unix in `format_attributes`.

### Changed
- **Widget Size Default**: Adjusted the default for dialog widgets to DialogSize::Small.


---


## [0.3.6] - 2025-12-30

### Added
- **Overlay widgets**: Added support for overlay widgets to dynamically toggle between widgets. Implemented the ShowInfo overlay as the first one.
- **ShowInfo**: Implemented the new showinfo overlay for file information.
- **Toggle Marker Advance**: Improved marker toggle logic. Now jumps to the next entry to make marking more seamless and easier.
- **Border shape**: Added border shapes to configuration. "square", "rounded" or "double".
- **Status line configuration**: Added status line configuration options.

### Changed
- Refactored dialog position logic for all widgets: Dialogs that use `TopLeft`, `TopRight`, or `Custom()` now appear a few rows lower in unified border mode so they never cover status or title lines

### Fixed
- **Input fields**: Input widgets now dynamically crop/scroll horizontally and keep the widget size during terminal resize.

### Internal
- **Theme/Config Consistency**: Dialog style, position, and size are now fully driven from theme/configuration.
- New `dialog_position_unified` and `adjusted_dialog_position` helpers to help with the widget drawing modes for each border mode.
- **Dependencies**: Added `humansize` and `chrono` crates for ShowInfo overlay widget.


---


## [0.3.5] - 2025-12-28

### Fixed
- **Preview Pane**: Resolved a race condition that caused a brief flash of old directory entries when rapidly navigating between folders immediately after startup.

### Internal
- **Allocation Optimization**: Optimized `tick()` in `app.rs` to pre-calculate the selected path, reducing the times `PathBuf` and string joins are done during the tick loop.


---


## [0.3.4] - 2025-12-28

### Added
- **Input polish**: Added cursor movement within input fields.
- **File collision**: Added `get_unused_path` to utils.rs. It now becomes `test_1.txt` instead of colliding.

### Fixed
- **Nav persistence**: The cursor now follows the file name when filtering. No more jumping back to the top when you type.

### Internal
- **Hardened NavState**: Threw 1 million iterations at the navigation math. It’s rock solid now.
- **Refactoring**: Renamed `popup` to `dialog` across codebase for better naming. (Just feels better).
- **Testing**: Added a bunch more of unit tests.
- **Documentation**: Added some documentation of code and modules. Still working on more...

---


## [0.3.3] - 2025-12-26

### Internal
- **Dependency update**: Updated `unicode-width` from `0.2.0` to `0.2.2`


---


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
