# Configuration

runner configuration is under active development and options may change over time.

## Default Configuration

Default configuration file can be generated with: `rn --init`

This will create a `runner.toml` with the following contents:

```toml
# Shows directories always before files
dirs_first = true

# Shows hidden files
show_hidden = false

# Shows hidden system files (Mostly useful for windows)
show_system = false

# Option to ignore case sensitivity
case_insensitive = true

# Display section will change how the file listing is shown
[display]

# Shows the selection icon next to the file/directory name
show_selection_marker = true

# Shows the default '/' symbol next to the directory name
show_dir_marker = true

# Option to disable the border of runner
borders = true

# Theme section to customize runner (Will be expanded upon)
[theme]

# Background color
background = "default"

# Changes the color of the selection
accent_color = "default"

# Editor configuration for runner
[editor]

# Change the default editor to open files with
cmd = "nvim"

# Customize keys for navigation
[keys]

# Possible to map multiple keys to a single action
open_file   = ["Enter"]
go_up       = ["k", "Up Arrow"]
go_down     = ["j", "Down Arrow"]
go_parent   = ["h", "Left Arrow", "Backspace"]
go_into_dir = ["l", "Right Arrow"]
quit        = ["q", "Esc"]
```
