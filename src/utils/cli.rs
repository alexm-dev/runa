//! Command-line argument parsing and help for runa.
//!
//! This module handles all CLI flag parsing used for config initialization and help.
//!
//! When invoked with no args/flags (rn), runa simply launches the TUI

use crate::config::Config;

pub(crate) enum CliAction {
    RunApp,
    RunAppAtPath(String),
    Exit,
}

pub(crate) fn handle_args() -> CliAction {
    let args: Vec<String> = std::env::args().collect();
    let config_path = Config::default_path();

    if args.len() < 2 {
        return CliAction::RunApp;
    }

    if args.len() > 2 {
        eprintln!("Error: runa accepts only one argument at a time.");
        eprintln!("Usage: rn [PATH] or rn [OPTION]");
        return CliAction::Exit;
    }

    match args[1].as_str() {
        "--version" | "-v" => {
            print_version();
            CliAction::Exit
        }
        "-h" | "--help" => {
            print_help();
            CliAction::Exit
        }
        "--config-help" => {
            print_config_help();
            CliAction::Exit
        }
        "--keybinds" | "--keybind" | "--key" => {
            print_keybinds();
            CliAction::Exit
        }
        "--init" => {
            if let Err(e) = Config::generate_default(&config_path, true) {
                eprintln!("Error: {}", e);
            }
            CliAction::Exit
        }
        "--init-full" => {
            if let Err(e) = Config::generate_default(&config_path, false) {
                eprintln!("Error: {}", e);
            }
            CliAction::Exit
        }
        arg if !arg.starts_with('-') && !arg.trim().is_empty() => {
            CliAction::RunAppAtPath(arg.to_string())
        }
        arg => {
            eprintln!("Unknown argument: {}", arg);
            eprintln!("Try --help for available options");
            CliAction::Exit
        }
    }
}

fn print_version() {
    println!("runa {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(
        r#"runa - A fast and lightweight terminal file manager written in Rust

USAGE:
  rn [PATH]

PATH:
  Directory to open (defaults to current directory)

OPTIONS:
      --init              Generate a minimal default configuration
      --init-full         Generate the full configuration with all options
      --config-help       Display all the configuration options
      --keybinds          Display all the default keybinds
  -h, --help              Print help information
  -v, --version           Display the current installed version of runa

ENVIRONMENT:
  RUNA_CONFIG             Override the default config path
"#
    );
}

const KEYBINDS_TEXT: &str = r##"
=========================
 Key Bindings
=========================
[keys]
  open_file                 ["enter"]
  go_up                     ["k", "up"]
  go_down                   ["j", "down"]
  go_parent                 ["h", "left", "Backspace"]
  go_into_dir               ["l", "right"]
  quit                      ["q", "esc"]
  delete                    ["d"]
  copy                      ["y"]
  paste                     ["p"]
  rename                    ["r"]
  create                    ["n"]
  create_directory          ["N"]
  move_file                 ["m"]
  filter                    ["f"]
  toggle_marker             ["space"]
  info                      ["i"]
  find                      ["s"]
  clear_markers             ["<c-c>"]
  clear_filter              ["<c-f>"]
  clear_clipboard           ["<f2>"]
  clear_all                 ["<c-l>"]
  alternate_delete          ["<m-d>"]
  go_to_bottom              ["G"]
  keybind_help              ["?"]

  go_to_top                 ["g"]     (press "g" then "g" again)
  go_to_home                ["h"]     (press "g" then "h")
  go_to_path                ["p"]     (press "g" then "p")

  tab_new                   ["<c-t>"]
  tab_close                 ["<c-w>"]
  tab_cycle                 ["<c-n>"]
  tab_next                  ["<c-n>"]
  tab_prev                  ["<c-p>"]

  scroll_up                 ["<c-d>"]   (Widget scrolling)
  scroll_down               ["<c-u>"]

  Syntax Reference:
    Modifiers: <c-x> (Ctrl), <m-x>/<a-x> (Alt/Meta), <s-x> (Shift)
    Standard:  ctrl+x, alt+x, shift+x, meta+x
    Special:   " ", "space", "back", "enter", "esc", "tab"

  Note:
    - Shorthand (c-, m-, s-) only works inside brackets <>.
    - The 'g' key is a prefix; it waits for the next key to trigger an action.
"##;

fn print_keybinds() {
    println!("{}", KEYBINDS_TEXT);
}

fn print_config_help() {
    let help_text = r##"
runa - Full Configuration Guide (runa.toml)

=========================
 General Settings
=========================
[general]
  dirs_first                 Sort directories before files [default: true]
  show_hidden                Show hidden files (dotfiles)
  show_symlink               Show symlinks
  show_system                Show system/protected files (mainly Windows)
  case_insensitive           Ignore case sensitivity in search/sort [default: true]
  always_show                Hidden entries always shown, e.g. [".config", "Downloads"]
  max_find_results           Max results for find (default: 2000, min: 15, max: 1_000_000)
  move_to_trash              Move files to trash bin instead of permanent deletion

=========================
 Display Settings
=========================
[display]
  selection_marker           Show selection/cursor marker [default: true]
  dir_marker                 Show '/' or marker for directories [default: true]
  borders                    "none", "unified", or "split"
  border_shape               "square", "rounded", or "double"
  titles                     Show pane titles at the top
  icons                      Show Nerd Font icons
  separators                 Show vertical lines between panes
  parent                     Show parent (left) pane [default: true]
  preview                    Show preview (right) pane [default: true]
  preview_underline          Underline preview selection instead of highlight
  preview_underline_color    Distinct color for preview underline
  entry_padding              Padding (# chars) left/right (0–4)
  scroll_padding             Reserved rows when scrolling
  toggle_marker_jump         Toggle marker jumping to first entry
  instant_preview            Toggle instant previews on every selection change

[display.preview_option]
  method                     Set the preview method ("bat" or "internal")
  theme                      Set the bat method theme. (only works if method = "bat")
  style                      Set the bat style. (only works if method = "bat")
  wrap                       Toggle line wrapping in the preview pane (only works if method = "bat")

[display.layout]
  parent                     Width % for parent pane
  main                       Width % for main pane
  preview                    Width % for preview pane

[display.info]               Toggle display file info attributes
  name
  file_type
  size
  modified
  perms

[display.status]             Toggle the status line options ("footer", "header, or "none" to disable)
  entry_count
  filter
  markers
  clipboard
  tasks
  tabs

=========================
 Theme Configuration
=========================
[theme]
  name                       Theme name, e.g. "gruvbox-dark"
  selection_icon             Symbol for selection (">" or " ")
  exe_color                  Coloring for executables

Each sub-table supports fg/bg colors ("Red", "Blue", hex "#RRGGBB", or "default"):

[theme.entry]                Normal entries (fg, bg)
[theme.accent]               Borders/titles (fg, bg)
[theme.selection]            Selection bar (fg, bg)
[theme.directory]            Directory entries (fg, bg)
[theme.separator]            Vertical separators (fg, bg)
[theme.parent]               Parent pane text (fg, bg, selection_fg, selection_bg)
[theme.preview]              Preview pane text (fg, bg, selection_fg, selection_bg)
[theme.underline]            Preview underline (fg, bg)
[theme.path]                 Path bar at the top (fg, bg)
[theme.symlink]              Symlink coloring (directory, file, target)
[theme.marker]               Multi-select marker (icon, fg, bg, clipboard)

[theme.status_line]          Status line color bar
  fg                         Foreground color for the status line
  bg                         Background color for the status line

[theme.widget]               Dialog/widgets config (see docs):
  position                   "center", [x, y], {x = 38, y = 32}
  size                       "small", [w, h], {w = 33, h = 15}
  confirm_size               Override size for confirmation widget
  move_size                  Override size for the move file widget
  find_size                  Override size for the find widget
  color.fg/bg                Text/background color
  border.fg/bg               Widget Border colors
  title.fg/bg                Widget Title colors
  label.fg/bg                Label colors for widgets like File Info
  value.fg/bg                Value colors for widgets like File Info
  go_to_help.size            Size of the go_to dialog when pressing the "g" prefix
  go_to_help.position        Position of the go_to dialog when pressing the "g" prefix

[theme.tab]                  Customization of the tab line
  marker                     String to set the marker. marker = ""
  separator                  String to set the separators between tabs. separator = ""
  active.fg/bg               Coloring for the active tab
  inactive.fg/bg             Coloring for the inactive tab
  line_format                Customization of the whole tab line. line_format = ["{idx}{marker}"]

[theme.info]                 File Info overlay widget
  color.fg/bg,
  border.fg/bg,
  title.fg/bg,
  position                   "center", "top_left", [x, y], { x, y }

=========================
 Editor
=========================
[editor]
  cmd                       Command to open files (e.g., "nvim", "code")
"##;

    println!("{}{}", help_text, KEYBINDS_TEXT);
}
