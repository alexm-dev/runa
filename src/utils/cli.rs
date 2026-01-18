//! Command-line argument parsing and help for runa.
//!
//! This module handles all CLI flag parsing used for config initialization and help.
//! It recognizes args/flags such: --help, --init, --init-full and --config-help
//!
//! When invoked with no args/flags (rn), runa simply launches the TUI

use crate::config::Config;

pub enum CliAction {
    RunApp,
    Exit,
}

pub fn handle_args() -> CliAction {
    let args: Vec<String> = std::env::args().collect();
    let config_path = Config::default_path();

    if args.len() <= 1 {
        return CliAction::RunApp;
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
        arg => {
            eprintln!("Unknown argument: {}", arg);
            CliAction::Exit
        }
    }
}

fn print_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(
        r#"runa - A fast and lightweight console file browser written in Rust

USAGE:
    rn [OPTIONS]

OPTIONS:
    --help, -h            Print help information
    --init                Generate a default config at ~/.config/runa/runa.toml
    --init-full           Generate the full config
    --config-help         Display all the configuration options

ENVIRONMENT:
    RUNA_CONFIG         Override the default config path
"#
    );
}

fn print_config_help() {
    let help_text = r##"
runa - Full Configuration Guide (runa.toml)

=========================
 General Settings
=========================
  dirs_first              (bool)    Sort directories before files [default: true]
  show_hidden             (bool)    Show hidden files (dotfiles)
  show_system             (bool)    Show system/protected files (mainly Windows)
  case_insensitive        (bool)    Ignore case sensitivity in search/sort [default: true]
  always_show             (list)    Hidden entries always shown, e.g. [".config", "Downloads"]
  max_find_results        (usize)   Max results for find (default: 2000, min: 15, max: 1_000_000)

=========================
 Display Settings
=========================
[display]
  selection_marker        (bool)    Show selection/cursor marker [default: true]
  dir_marker              (bool)    Show '/' or marker for directories [default: true]
  borders                 (str)     "none", "unified", or "split"
  border_shape            (str)     "square", "rounded", or "double"
  titles                  (bool)    Show pane titles at the top
  icons                   (bool)    Show Nerd Font icons
  separators              (bool)    Show vertical lines between panes
  parent                  (bool)    Show parent (left) pane [default: true]
  preview                 (bool)    Show preview (right) pane [default: true]
  preview_underline       (bool)    Underline preview selection instead of highlight
  preview_underline_color (bool)    Distinct color for preview underline
  entry_padding           (usize)   Padding (# chars) left/right (0–4)
  scroll_padding          (usize)   Reserved rows when scrolling
  toggle_marker_jump      (bool)    Toggle marker jumping to first entry
  instant_preview         (bool)    Toggle instant previews on every selection change

[display.layout]
  parent                  (u16)     Width % for parent pane
  main                    (u16)     Width % for main pane
  preview                 (u16)     Width % for preview pane

[display.info] (toggle display of file info attributes)
  name                    (bool)
  file_type               (bool)
  size                    (bool)
  modified                (bool)
  perms                   (bool)

=========================
 Theme Configuration
=========================
[theme]
  name                    (str)     Theme name, e.g. "gruvbox-dark"
  selection_icon          (str)     Symbol for selection (">" or " ")

# Each sub-table supports fg/bg colors ("Red", "Blue", hex "#RRGGBB", or "default"):
[theme.selection]                  Selection bar (fg, bg)
[theme.accent]                     Borders/titles (fg, bg)
[theme.entry]                      Normal entries (fg, bg)
[theme.directory]                  Directory entries (fg, bg)
[theme.separator]                  Vertical separators (fg, bg)
[theme.parent]                     Parent pane text (fg, bg, selection_fg, selection_bg)
[theme.preview]                    Preview pane text (fg, bg, selection_fg, selection_bg)
[theme.marker]                     Multi-select marker (icon, fg, bg, clipboard)
[theme.underline]                  Preview underline (fg, bg)
[theme.path]                       Path bar at the top (fg, bg)

[theme.status_line]                Status line color bar
  fg                      (str)  Foreground color for the status line
  bg                      (str)  Background color for the status line

[theme.widget]                     Dialog/widgets config (see docs):
  position                (str/list/table)  "center", [x, y], {x = 38, y = 32}
  size                    (str/list/table)  "small", [w, h], {w = 33, h = 15}
  confirm_size            (str/list/table)  Override size for confirmation popups
  color.fg/bg             (str)             Text/background color
  border.fg/bg            (str)
  title.fg/bg             (str)

[theme.info]              File info overlay
 color.fg/bg,             (str)
 border.fg/bg,            (str)
 title.fg/bg,             (str)
 position                 (str/list/table) "center", "top_left", [x, y], { x, y }

=========================
 Editor
=========================
[editor]
  cmd                     (str)    Command to open files (e.g., "nvim", "code")

=========================
 Key Bindings
=========================
[keys]
  open_file               (list)   e.g. ["Enter"]
  go_up                   (list)   ["k", "Up"]
  go_down                 (list)   ["j", "Down"]
  go_parent               (list)   ["h", "Left", "Backspace"]
  go_into_dir             (list)   ["l", "Right"]
  quit                    (list)   ["q", "Esc"]
  delete                  (list)   ["d"]
  copy                    (list)   ["y"]
  paste                   (list)   ["p"]
  rename                  (list)   ["r"]
  create                  (list)   ["n"]
  create_directory        (list)   ["Shift+n"]
  filter                  (list)   ["f"]
  toggle_marker           (list)   [" "]     (space bar)
  info                    (list)   ["i"]
  find                    (list)   ["s"]
  clear_markers           (list)   ["Ctrl+c]
  clear_filter            (list)   ["Ctrl+f]

    (Use "Shift+x", "Ctrl+x" as needed. " " means space bar. Omit a binding to use the default.)

=========================
 Examples
=========================
borders = "split"

[display.layout]
main = 40

[theme.accent]
fg = "#00ff00"
bg = "default"
"##;

    println!("{}", help_text);
}
