# Runa Configuration Guide

runa is under active development and options may change over time.

## Config File

`runa` is configured via a TOML file located at:

`~/.config/runa/runa.toml` (Linux/macOS)

`C:\Users\<UserName>\.config\runa\runa.toml` (Windows)

**Override**: You can specify a custom path by setting the `RUNA_CONFIG` environment variable.

## Quick Start

If you don't have a config file yet, you can generate one automatically:

- `rn --init`: Generates the configuration.
- `rn --init-full`: Generates a full config files with all options as seen below.
- `rn --config-help`: Displays all configuration options.

## General Settings

```toml
# ANY unused / default options can be removed from runa.toml.
# Removed / unused options will default to the internal defaults as seen below.

# Sort directories before files
dirs_first = true

# Show hidden files (dotfiles)
show_hidden = false

# Show hidden system files (mostly for Windows)
show_system = false

# Ignore case sensitivity when searching or sorting
case_insensitive = true

# Always show these directories, even if 'show_hidden' is false. Example: always_show = [".config", "Downloads"]
always_show = []

[display]
# Show the selection icon next to the file/directory name
selection_marker = true

# Show the default '/' symbol next to directory names
dir_marker = true

# Border style: "none", "unified", or "split"
borders = "split"

# Show pane titles at the top (e.g., "Main", "Preview")
titles = false

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

[display.layout]
# Display ratios for panes (will be scaled to 100%)
parent = 20
main = 40
preview = 40
```




## Theme Configuration

```toml
[theme]
# The symbol for the current selection. Use "" or " " to disable.
selection_icon = ">"

# Theme color values can be terminal color names ("Red", "Blue", etc.), hex ("#RRGGBB"), or "default".

# Each [theme.*] section supports keys: fg (foreground), bg (background)
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
selection_fg = "default"
selection_bg = "default"

[theme.preview]       # Preview pane text
fg = "default"
bg = "default"
selection_fg = "default"
selection_bg = "default"

[theme.marker]        # Multi-select marker
icon = "*"
fg = "yellow"
bg = "default"

[theme.underline]     # Underline colors (if enabled)
fg = "default"
bg = "default"

[theme.path]          # Path bar at the top
fg = "magenta"
bg = "default"

# Full widget/popup theming: position, size, and colors

[theme.widget]
# Leave blank or omit to use the regular defaults.
# Popup position: choose one of the following styles
#   - Preset string:    "center", "top_left", "bottom_right", etc. Also possible to write "topleft", "bottomright", etc..
#   - List:             [x, y]             # percent of screen, e.g., [38, 32]
#   - Table/object:     { x = 25, y = 60 } # percent of screen
position = "center"

# Popup size: choose one of
#   - Preset string:    "small", "medium", "large"
#   - List:             [width, height]    # percent, e.g., [38, 32]
#   - Table/object:     { w = 38, h = 32 } # percent
size = "medium"

# Confirmation popup size (for confirmations like deleting files):
#   - Preset string, list, or table, just like "size" above.
#   - Leave blank or omit to use the regular `size`.
confirm_size = "large"

[theme.widget.color]   # Popup/widget colors
fg = "white"
bg = "black"

[theme.widget.border]  # Popup/widget border colors
fg = "magenta"
bg = "default"
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
filter              = ["f"]
toggle_marker       = [" "]     # space bar
```

You may remove any binding to let it fall back to the default.


---


## EXAMPLES

```toml
borders = "split"

[display.layout]
main = 40

[theme.accent]
fg = "#00ff00"
bg = "default"

[theme.widget]
position = [25, 60]
size = { w = 36, h = 20 }
```
