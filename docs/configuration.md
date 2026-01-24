# Runa Configuration Guide

runa is under active development and options may change over time.

## Contents

- [Config File Location](#config-file)
- [Quick Start](#quick-start)
- [General Settings](#general-settings)
- [Optional Tool Integration](#optional-tool-integration)
- [Theme Configuration](#theme-configuration)
- [Advanced Theme Configuration](#advanced-theme-configuration)
- [Editor](#editor)
- [Key Bindings](#key-bindings)
- [Examples](#examples)

## Config File

`runa` is configured via a TOML file. By default, it looks for your config in the following order:

1. **Custom Path Override**:
    If the `RUNA_CONFIG` environment variable is set, runa will use its value as the config file path.  
    **Example:**
    ```sh
    export RUNA_CONFIG=/path/to/runa.toml

2. **XDG Config Directory** (Linux, macOS, and supported environments):
    If `XDG_CONFIG_HOME` is set, runa looks for
    ```sh
    $XDG_CONFIG_HOME/runa/runa.toml
    ```

3. **Default Path**:
    - `~/.config/runa/runa.toml` (Linux/macOS)
    - `C:\Users\<UserName>\.config\runa\runa.toml` (Windows)

## Optional Tool Integration

Some advanced featues like `find`, require external tools to be installed.

- **[bat](https://github.com/sharkdp/bat)**:
    Used for fast, syntax-highlighted file previews when `[display.previews_options.method]` is set to `"bat"`.  
    _If `bat` is not installed, runa will fall back to the built-in preview method._

- **[fd](https://github.com/sharkdp/fd)**:
    Used for blazing fast recursive file/folder search.

**Both tools are completely optional** but highly recommended.

You can install them with your package manager:
```sh
sudo pacman -S bat fd           # On Arch
sudo apt install bat fd-find    # On Debian/Ubuntu
sudo dnf install bat fd-find    # Fedora/RHEL
```

## Quick Start

Launch `runa` by simply running `rn`. You can also specify a starting directory:

- `rn`: Opens up runa at the current directory.
- `rn /path/to/dir`: Opens runa at the specified location.

If you don't have a config file yet, you can generate one automatically:

### Configuration Setup
- `rn --init`: Generates the configuration.
- `rn --init-full`: Creates a full configuration file with all options as shown below.
- `rn --config-help`: Displays all configuration options.

### Quick Reference
- `rn --help`: Shows the standard CLI help menu
- `rn --keybinds`: Displays all default keybinds for quick reference
- `rn --version`: Displays the current installed version

## General Settings

```toml
# ANY unused / default options can be removed from runa.toml.
# Removed / unused options will default to the internal defaults as seen below.

[general]
# Sort directories before files
dirs_first = true

# Show hidden files (dotfiles)
show_hidden = true

# Show symlinks and the targets
show_symlink = true

# Show hidden system files (mostly for Windows)
show_system = false

# Ignore case sensitivity when searching or sorting
case_insensitive = true

# Always show these directories, even if 'show_hidden' is false. Example: always_show = [".config", "Downloads"]
always_show = []

# Configure the maximum number of find/search results to display.
# 2000 is the default.
# Minimum allowed: 15
# Maximum allowed: 1_000_000 (values above this will be clamped)
max_find_results = 2000

# Toggle to move deleted files to the the recycle bin instead of being permanently deleted.
# This will set the default delete key, the alternate_delete key will then alternate between the toggle.
move_to_trash = true


[display]
# Show the selection icon next to the file/directory name
selection_marker = true

# Show the default '/' symbol next to directory names
dir_marker = true

# Border style: "none", "unified", or "split"
borders = "split"

# Border shape: "square", "rounded" or "double"
border_shape = "square"

# Show pane titles at the top (e.g., "Main", "Preview")
titles = false

# Show Nerd Font icons. Requires a Nerd Font to be installed and used.
icons = false

# Draw vertical lines between panes
separators = true

# Show the parent directory pane (left)
parent = true

# Show the file preview pane (right)
preview = true

# Enable underline in the preview pane
preview_underline = true

# Use independent color for the preview underline
preview_underline_color = false

# Padding from entry to pane edge (0–4)
entry_padding = 1

# Scroll padding of the main pane
scroll_padding = 5

# Toggle if the marker selection should jump to the first entry whenever selection is at the bottom
toggle_marker_jump = false

# Toggle previews to instantly render on every selection change
# Default = false which results in pending preview when holding down a navigation key.
instant_preview = false

# Set the position or disable the entry count for the current directory
# Available options: "footer", "header" or "none" to disable
entry_count = "footer"

# Options for the preview method used by the preview pane.
# Options: "internal" and "bat". For "bat" you will need to have `bat` installed otherwise it will fallback to internal.
[display.previews_options]
method = "internal"

# Optionals for when method = "bat" otherwise these will be ignored by runa.
# Change the style of the `bat` preview method.
# Options: "plain", "numbers", "full".
style = "plain"

# Options to set the bat theme of the bat preview method
# All available bat themes are supported
theme = "default"

# Toggle wrapping the bat output to the pane width.
# If false, long lines stay on onle line and go off-screen
# If true, all the lines are wrapped to the pane width
wrap = "false"

[display.layout]
# Display ratios for panes (will be scaled to 100%)
parent = 20
main = 40
preview = 40

# Diplay the file info attributes.
[display.info]
name = true
file_type = true
size = true
modified = true
perms = true
position = "default"
```




## Theme Configuration

The easiest way to change colors:  
Just set the colors you care about directly under `[theme]`, you only need to override what you want.

```toml
[theme]
# The name of the preset themes included in runa.
# Choose a preset, or leave as "default" or omit.
name = "default"
# Available options (case-sensitive strings):
#   "gruvbox-dark"
#   "gruvbox-dark-hard"
#   "gruvbox-light"
#   "catppuccin-mocha"
#   "catppuccin-frappe"
#   "catppuccin-macchiato"
#   "catppuccin-latte"
#   "nightfox"
#   "carbonfox"
#   "tokyonight"
#   "tokyonight-storm"
#   "tokyonight-day"
#   "everforest"
#   "rose-pine"       # or "rose_pine"
#   "nord"
#   "two-dark"
#   "one-dark"
#   "solarized-dark"
#   "solarized-light"
#   "dracula"
#   "monokai"

# The symbol for the current selection. Use "" or " " to disable.
selection_icon = ">"

# Set the colors of binaries/executables only on UNIX. 
# By default LightGreen.
exe_color = "default"

# You can set each color field directly in [theme] instead.
# There is now need to create each [theme] subsection for overriding and or creating custom themes.
# Example:
selection.fg = "#EEBBAA"
directory.fg = "#7BA"
widget.border.fg = "#555555"
symlink.directory = "#8aA"
# and so on
```



## Advanced Theme Configuration

You can override all these color options for each section down below.  
These options are optional and can be omitted.  
```toml

# Color keys for most sections are always placed directly in the table:
# [theme.selection]
# fg = "yellow"
# bg = "default"
#
# For larger sections such as [theme.widget] or [theme.info], you may use either
# dot notation (e.g. color.fg, border.bg) OR define subtables like [theme.widget.color]:
#
# [theme.widget]
# color.fg = "white"
# color.bg = "black"
# border.fg = "magenta"
#
# Alternatively, this works and is equivalent:
# [theme.widget.color]
# fg = "white"
# bg = "black"
#
# [theme.widget.border]
# fg = "magenta"
#
# Theme color values can be terminal color names ("Red", "Blue", etc.), hex ("#RRGGBB"), or "default".

# The global selection coloring section.
# Can be overwritten for each pane.
[theme.selection]     # Selection bar colors
fg = "default"
bg = "default"

[theme.accent]        # Borders/titles
fg = "default"
bg = "default"

[theme.entry]         # Normal entries
fg = "default"
bg = "default"

[theme.directory]     # Directory entries
fg = "cyan"
bg = "default"

[theme.separator]     # Vertical separators
fg = "default"
bg = "default"

[theme.parent]        # Parent pane text
fg = "default"
bg = "default"

# Sets the selection coloring mode.
# If off, then selection.fg and .bg are ignored and off.
# If on, then selection.bg and .bg are set to "default" or to a specific override.
selection_mode = "on"

# Overrides the central [theme.selection] for just the parent pane
selection.fg = "default"
selection.bg = "default"

[theme.preview]       # Preview pane text
fg = "default"
bg = "default"

# Sets the selection coloring mode.
# If off, then selection.fg and .bg are ignored and off.
# If on, then selection.bg and .bg are set to "default" or to a specific override.
selection_mode = "on"

# Overrides the central [theme.selection] for just the preview pane
selection.fg = "default"
selection.bg = "default"

[theme.symlink]
# Set the symlink colors for symlinks who are linked to directories
directory = "default"

# Set color for symlink who are linked to files
file = "default"

# Set the target path color of the symlink
target = "default"

[theme.marker]        # Multi-select marker
icon = "*"
fg = "yellow"
bg = "default"
# Change the color of the clipboard when you copy a entry via multiselect or via normal yank/copy
clipboard.fg = "default"
clipboard.bg = "default"

[theme.underline]     # Underline colors (if enabled)
fg = "default"
bg = "default"

[theme.path]          # Path bar at the top
fg = "magenta"
bg = "default"

# Full widget/dialog theming: position, size, and colors

[theme.widget]
# Leave blank or omit to use the regular defaults.
# Popup position: choose one of the following styles
#   - Preset string:    "center", "top_left", "bottom_right", etc. Also possible to write "topleft", "bottomright", etc..
#   - List:             [x, y]             # percent of screen, e.g., [38, 32]
#   - Table/object:     { x = 25, y = 60 } # percent of screen
position = "center"

# Popup size: choose one of:
#   - Preset string:    "small", "medium", "large"
#   - List:             [width, height]    # **cells (columns x rows)**, e.g., [60, 12]
#   - Table/object:     { w = 60, h = 12 } # **cells**
# size arrays/tables are now always **cell-based** (not percent-based!)
size = "medium"

# Confirmation dialog size (for confirmations like deleting files):
#   - Preset string, list, or table, just like "size" above.
#   - Leave blank or omit to use the regular `size`.
confirm_size = "large"

# Move file widget size
#   - Preset string, list, or table, just like "size" above.
#   - Leave blank or omit to use the regular `size`. (default is [70, 14])
move_size = [70, 14]

# Option to specify the maximal `drawn` results of the find widget.
# Not to be confused with `max_find_results` which calculates the overall maximal results fd will generate.
find_visible_results = 5

# Option to configure the find widget widht
find_width = 40

# Coloring for the widgets
color.fg = "white"
color.bg = "black"

border.fg = "magenta"
border.bg = "default"

title.fg = "default"
title.bg = "default"

# Configuration for the status_line
[theme.status_line]
fg = "magenta"
bg = "default"

# Configuration for the File info widget
[theme.info]
color.fg = "default"
color.bg = "default"

border.fg = "default"
border.bg = "default"

title.fg = "default"
title.bg = "default"

position = "bottom_left"
```



## Editor

```toml
[editor]
# Command to open files (e.g., "nvim", "code", etc.)
# "code.cmd" on windows
cmd = "nvim"
```




## Key Bindings

All values are lists (multiple shortcuts per action). Use "Shift+x", "Ctrl+x" as needed. `" "` means space bar.

```toml
[keys]
open_file           = ["Enter"]
go_up               = ["k", "Up"]
go_down             = ["j", "Down"]
go_parent           = ["h", "Left", "Backspace"]
go_into_dir         = ["l", "Right"]
quit                = ["q", "Esc"]
delete              = ["d"]
copy                = ["y"]
paste               = ["p"]
rename              = ["r"]
create              = ["n"]
create_directory    = ["Shift+n"]
move_file           = ["m"]
filter              = ["f"]
toggle_marker       = [" "]         # space bar
info                = ["i"]
find                = ["s"]
clear_markers       = ["Ctrl+c"]
clear_filter        = ["Ctrl+f"]
alternate_delete    = ["Ctrl+d"]    # Alternates between move_to_trash and permanently delete
```

You may remove any binding to let it fall back to the default.


---


## EXAMPLES

```toml
[display]
borders = "split"

[display.layout]
main = 40

[theme]
name = "gruvbox-dark"
selection.fg = "#EBA"
entry.fg = "#333333"
directory.fg = "magenta"

[theme.accent]
fg = "#00ff00"

[theme.widget]
position = [25, 60]
size = { w = 36, h = 20 }
```
