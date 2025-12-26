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

fn print_help() {
    println!(
        r#"runa - A fast and lightweight console file browser written in Rust

USAGE:
    rn [OPTIONS]

OPTIONS:
    --help, -h            Print help information
    --init                Generate full default config at ~/.config/runa/runa.toml
    --init-minimal        Generate minimal config (overrides only)
    --config-help         Display all the configuration options

ENVIRONMENT:
    RUNA_CONFIG         Override the default config path
"#
    );
}

fn print_config_help() {
    let help_text = r##"
runa - Full Configuration Guide (runa.toml)

# General Settings
  dirs_first              (bool)  Sort directories before files
  show_hidden             (bool)  Show hidden files (dotfiles)
  show_system             (bool)  Show system/protected files
  case_insensitive        (bool)  Ignore case when searching/sorting
  always_show             (list)  List of hidden names to always show

[display]
  selection_marker        (bool)  Show the cursor marker
  dir_marker              (bool)  Show a marker for directories
  borders                 (str)   "none", "unified", or "split"
  titles                  (bool)  Show pane titles at the top
  separators              (bool)  Draw vertical lines between panes
  parent                  (bool)  Show the parent directory pane
  preview                 (bool)  Show the file preview pane
  preview_underline       (bool)  Use underline for preview selection instead of a highlighted selection
  preview_underline_color (bool)  Use underline colors instead of selection colors
  entry_padding           (usize) Entry padding for all the panes
  scroll_padding          (usize) Scroll padding of the main pane

[display.layout]
  parent                  (u16)   Width % of the parent pane
  main                    (u16)   Width % of the center pane
  preview                 (u16)   Width % of the preview pane

[theme]
  selection_icon          (str)   The cursor string (e.g., "> ")

# Theme sections          (Each supports "fg" and "bg" keys,)
[theme.selection]         Selection bar colors    fg (str), bg (str)
[theme.accent]            Border/title accents    fg (str), bg (str)
[theme.entry]             Standard entry colors   fg (str), bg (str)
[theme.directory]         Directory entry colors  fg (str), bg (str)
[theme.separator]         Vertical line colors    fg (str), bg (str)
[theme.parent]            Parent pane text        fg (str), bg (str)
[theme.preview]           Preview pane text       fg (str), bg (str)
[theme.path]              Path bar colors         fg (str), bg (str)
[theme.underline]         Underline colors        fg (str), bg (str)
[theme.widget]            Popup/widget settings:
  position                (str/list/table)  "center", "top_left", "bottom_right", [38, 32], { x = 25, y = 60 }
  size                    (str/list/table)  "small", "medium", "large", [38, 32], { w = 38, h = 32 }
[theme.widget.color]      fg/bg for popups/widgets  fg (str), bg (str)
[theme.widget.border]     fg/bg for popup borders   fg (str), bg (str)


[editor]
  cmd                    (str)   Command to open files (e.g., "nvim")

[keys]
  open_file              (list)  e.g., ["Enter"]
  go_up                  (list)  e.g., ["k", "Up"]
  go_down                (list)  e.g., ["j", "Down"]
  go_parent              (list)  e.g., ["h", "Left", "Backspace"]
  go_into_dir            (list)  e.g., ["l", "Right"]
  quit                   (list)  e.g., ["q", "Esc"]
  delete                 (list)  e.g., ["d"]
  copy                   (list)  e.g., ["y"]
  paste                  (list)  e.g., ["p"]
  rename                 (list)  e.g., ["r"]
  create                 (list)  e.g., ["n"]
  create_directory       (list)  e.g., ["Shift+n"]
  filter                 (list)  e.g., ["f"]
  toggle_marker          (list)  e.g., [" "]

Use 'Shift+x' or 'Ctrl+x' as needed. " " means space bar.

EXAMPLES:
  borders = "split"

  [display.layout]
  main = 40

  [theme.accent]
  fg = "#00ff00"
  bg = "default"
"##;

    println!("{}", help_text);
}
