# Runner Configuration Guide

runner is under active development and options may change over time.

## Config File

`runner` is configured via a TOML file located at:

`~/.config/runner/runner.toml` (Linux/macOS)

`C:\Users\<UserName>\.config\runner\runner.toml` (Windows)

**Override**: You can specify a custom path by setting the `RUNNER_CONFIG` environment variable.

## Quick Start

If you don't have a config file yet, you can generate one automatically:

- `rn --init`: Generates the full configuration with all default options.
- `rn --init-minimal`: Generates a minimal, clean config file, with some common overrides
- `rn --config-help`: Displays all configuration options.

## General Settings
```toml

# Default settings used by runner

# Sort directories before files
dirs_first = true

# Shows hidden files (dotfiles)
show_hidden = false

# Shows hidden system files (Mostly useful for Windows)
show_system = false

# Ignore case sensitivity when searching or sorting
case_insensitive = true

# Set directories to be always shown, ignoring the show_hidden option. Example: always_show = [".config", "..."]
always_show = []

[display]
# Shows the selection icon next to the file/directory name
selection_marker = true

# Shows the default '/' symbol next to the directory name
dir_marker = true

# Border style for the application: "none", "unified", or "split"
borders = "unified"

# Show pane titles at the top (e.g., "Main", "Preview")
titles = false

# Draw vertical lines between panes
separators = false

# Show the parent directory pane (left)
origin = false

# Show the file preview pane (right)
preview = true

# Pane width ratios (relative to each other)
origin_ratio = 30
main_ratio = 40
preview_ratio = 30

# Option to enable the underline in the preview pane
preview_underline = true

# Enable independent color for the preview underline.
# If true, uses the color defined in [theme.underline].
# If false, the underline color is inherited from [theme.selection].
preview_underline_color = false

# Scroll padding of the main pane
scroll_padding = 5

# Theme colors can be changed with either Hex "#RRGGBB" colors or terminal colors
# Any key can be ommited and does not have to be inside the runner.toml.
# Example
# [theme.selection]
# fg = "#FFFFFF"
[theme]
# The symbol used to indicate the current selection. "" or " " to disable.
selection_icon = "> "

# Color pairs for the entry selection color (The cursor and not all entries)
[theme.selection]
fg = "default"
bg = "default"

# These colors are used when `preview_underline_color = true`
# If preview_underline_color = true, these colors overlice the standard selection colors
[theme.underline]
fg = "default"
bg = "default"

# Color pair for the accents (of the main pane)
[theme.accent]
fg = "default"
bg = "default"

# Colors for the entries (of the main pane)
[theme.entry]
fg = "default"
bg = "default"

# Colors for the directory entries in all panes
[theme.directory]
fg = "default"
bg = "default"

# Color for the separator lines
[theme.separator]
fg = "default"
bg = "default"

# Colors for the origin (parent) pane
[theme.origin]
fg = "default"
bg = "default"
selection_fg = "default"
selection_bg = "default"

# Colors for the preview pane
[theme.preview]
fg = "default"
bg = "default"
selection_fg = "default"
selection_bg = "default"

# Colors for the path view at the top of the UI
[theme.path]
fg = "cyan"
bg = "default"

[editor]
# Change the default editor to open files with
# "code" ("code.cmd" on windows)
cmd = "nvim"

[keys]
# List of keys mapped to specific actions
open_file   = ["Enter"]
go_up       = ["k", "Up Arrow"]
go_down     = ["j", "Down Arrow"]
go_origin   = ["h", "Left Arrow", "Backspace"]
go_into_dir = ["l", "Right Arrow"]
quit        = ["q", "Esc"]
```
