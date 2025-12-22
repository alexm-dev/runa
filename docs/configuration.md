# Runner Configuration Guide

runner is under active development and options may change over time.

## Config File

`runner` is configured via a TOML file located at:

`~/.config/runner/runner.toml` (Linux/macOS)

`C:\Users\<UserName>\.config\runner\runner.toml` (Windows)

**Override**: You can specify a custom path by setting the `RUNNER_CONFIG` environment variable.

## Quick Start

If you don't have a config file yet, you can generate one automatically:

- `rn --init`: Generates the full configuration.
- `rn --init-minimal`: Generates a minimal, clean config file, with some common overrides
- `rn --config-help`: Displays all configuration options.

## General Settings
```toml

# Default settings used by runner

# ANY unused / default options can be removed from the runner.toml
# Removed / unused options will default to the internal defaults as seen below.

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

## Border style for the application: "none", "unified", or "split"
borders = "split"

# Show pane titles at the top (e.g., "Main", "Preview")
titles = false

# Draw vertical lines between panes
separators = true

# Show the parent directory pane (left)
parent = true

# Show the file preview pane (right)
preview = true

# Option to enable the underline in the preview pane
preview_underline = true

# Enable independent color for the preview underline.
# If true, uses the color defined in [theme.underline].
# If false, the underline color is inherited from [theme.preview_underline].selection_fg and .selection_bg.
preview_underline_color = false

# Pads the gap between the entry and the pane edge.
# From 0 to 4.
entry_padding = 1

# Scroll padding of the main pane
scroll_padding = 5

# The display ratio of each pane
# Will always be automatically calculated to scale to 100%. Meaning it can be parent = 50, main = 50, preview = 50.
[display.layout]
parent = 20
main = 40
preview = 40

[theme]
# The symbol used to indicate the current selection. "" or " " to disable.
selection_icon = "> "

# Theme colors can be changed with either Hex "#RRGGBB" (or #RGB) colors, terminal colors "Red", "Blue" "etc.." or "default"
# "default" sets the colors to the internal defaults
# Sections can be removed in the config if not needed.
# Example: fg = "#FFFFFF", fg = "#FFF" or fg "cyan"

# Color pairs for the entry selection color (The cursor and not all entries)
[theme.selection]
fg = "default"
bg = "default"

# Color pair for the accents.
# Changes the coloring of the borders. If borders are "none", then this is ignored.
[theme.accent]
fg = "default"
bg = "default"

# Colors for the entries (of the main pane)
[theme.entry]
fg = "default"
bg = "default"

# Colors for the directory entries in all panes
[theme.directory]
fg = "cyan"
bg = "default"

# Color for the separator lines
# If separators are disabled, then this section is ignored.
[theme.separator]
fg = "default"
bg = "default"

# Colors for the parent pane
[theme.parent]
# Changes the pane entry color entirely
fg = "default"
bg = "default"
# Changes the selection line for the pane
# Will overwrite the [theme.selection] configuration.
selection_fg = "default"
selection_bg = "default"

# Colors for the preview pane
[theme.preview]
# Changes the pane entry color entirely
fg = "default"
bg = "default"
# Will overwrite the [theme.selection] configuration.
selection_fg = "default"
selection_bg = "default"

# These colors are used when `preview_underline_color = true`
# If preview_underline_color = true, these colors override the standard selection colors for the underline
[theme.underline]
fg = "default"
bg = "default"

# Colors for the path view at the top of the UI
[theme.path]
fg = "magenta"
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
go_parent   = ["h", "Left Arrow", "Backspace"]
go_into_dir = ["l", "Right Arrow"]
quit        = ["q", "Esc"]
```
