//! Key mapping and action dispatch system for runa
//!
//! Defines key to an action, parsing from the config, and enum variants
//! for all navigation, file and actions used by runa.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

/// Represents any action in the app: navigation, file, or system.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Action {
    Nav(NavAction),
    File(FileAction),
    System(SystemAction),
}

/// Navigation actions (move, into_parent, markers, etc.)
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum NavAction {
    GoParent,
    GoIntoDir,
    GoUp,
    GoDown,
    ToggleMarker,
}

/// File actions (delete, copy, open, paste, etc.)
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileAction {
    Delete,
    Copy,
    Open,
    Paste,
    Rename,
    Create,
    CreateDirectory,
    Filter,
    ShowInfo,
    FuzzyFind,
}

/// System actions (quit)
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SystemAction {
    Quit,
}

/// Key + modifiers as used in keybind/keymap
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Stores the mapping from Key to action, which is built in the config
pub struct Keymap {
    map: HashMap<Key, Action>,
}

impl Keymap {
    pub fn from_config(config: &crate::config::Config) -> Self {
        let mut map = HashMap::new();
        let keys = config.keys();

        let parse_key = |s: &str| -> Option<Key> {
            let mut modifiers = KeyModifiers::NONE;
            let mut code: Option<KeyCode> = None;

            for part in s.split('+') {
                match part {
                    "Ctrl" | "Control" => modifiers |= KeyModifiers::CONTROL,
                    "Shift" => modifiers |= KeyModifiers::SHIFT,
                    "Alt" => modifiers |= KeyModifiers::ALT,

                    "Up" => code = Some(KeyCode::Up),
                    "Down" => code = Some(KeyCode::Down),
                    "Left" => code = Some(KeyCode::Left),
                    "Right" => code = Some(KeyCode::Right),
                    "Enter" => code = Some(KeyCode::Enter),
                    "Esc" => code = Some(KeyCode::Esc),
                    "Backspace" => code = Some(KeyCode::Backspace),
                    "Tab" => code = Some(KeyCode::Tab),

                    p if p.starts_with('F') => {
                        let n = p[1..].parse().ok()?;
                        code = Some(KeyCode::F(n));
                    }

                    p if p.len() == 1 => {
                        let mut char = p.chars().next()?;
                        if modifiers.contains(KeyModifiers::SHIFT) {
                            char = char.to_ascii_uppercase();
                        }
                        code = Some(KeyCode::Char(char));
                    }

                    _ => return None,
                }
            }

            Some(Key {
                code: code?,
                modifiers,
            })
        };

        let mut bind = |key_list: &[String], action: Action| {
            for k in key_list {
                if let Some(key) = parse_key(k) {
                    map.insert(key, action);
                }
            }
        };

        bind(keys.go_parent(), Action::Nav(NavAction::GoParent));
        bind(keys.go_into_dir(), Action::Nav(NavAction::GoIntoDir));
        bind(keys.go_up(), Action::Nav(NavAction::GoUp));
        bind(keys.go_down(), Action::Nav(NavAction::GoDown));
        bind(keys.toggle_marker(), Action::Nav(NavAction::ToggleMarker));
        bind(keys.open_file(), Action::File(FileAction::Open));
        bind(keys.delete(), Action::File(FileAction::Delete));
        bind(keys.copy(), Action::File(FileAction::Copy));
        bind(keys.paste(), Action::File(FileAction::Paste));
        bind(keys.rename(), Action::File(FileAction::Rename));
        bind(keys.create(), Action::File(FileAction::Create));
        bind(
            keys.create_directory(),
            Action::File(FileAction::CreateDirectory),
        );
        bind(keys.filter(), Action::File(FileAction::Filter));
        bind(keys.quit(), Action::System(SystemAction::Quit));
        bind(keys.show_info(), Action::File(FileAction::ShowInfo));
        bind(keys.find(), Action::File(FileAction::FuzzyFind));

        Keymap { map }
    }

    pub fn lookup(&self, key: KeyEvent) -> Option<Action> {
        let k = Key {
            code: key.code,
            modifiers: key.modifiers,
        };
        self.map.get(&k).copied()
    }
}
