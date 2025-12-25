use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Action {
    Nav(NavAction),
    File(FileAction),
    System(SystemAction),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum NavAction {
    GoParent,
    GoIntoDir,
    GoUp,
    GoDown,
    ToggleMarker,
}

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
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SystemAction {
    Quit,
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

pub struct Keymap {
    map: HashMap<Key, Action>,
}

impl Keymap {
    pub fn from_config(config: &crate::config::Config) -> Self {
        let mut map = HashMap::new();
        let keys = config.keys();

        let parse_key = |s: &str| -> Option<Key> {
            match s {
                "Up" => Some(Key {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }),
                "Down" => Some(Key {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                }),
                "Left" => Some(Key {
                    code: KeyCode::Left,
                    modifiers: KeyModifiers::NONE,
                }),
                "Right" => Some(Key {
                    code: KeyCode::Right,
                    modifiers: KeyModifiers::NONE,
                }),
                "Enter" => Some(Key {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                }),
                "Esc" => Some(Key {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                }),
                "Backspace" => Some(Key {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::NONE,
                }),
                "Tab" => Some(Key {
                    code: KeyCode::Tab,
                    modifiers: KeyModifiers::NONE,
                }),
                s if s.starts_with('F') => {
                    let n = s[1..].parse().ok()?;
                    Some(Key {
                        code: KeyCode::F(n),
                        modifiers: KeyModifiers::NONE,
                    })
                }
                s if s.len() == 1 => Some(Key {
                    code: KeyCode::Char(s.chars().next()?),
                    modifiers: KeyModifiers::NONE,
                }),
                _ => None,
            }
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

        Keymap { map }
    }

    pub fn lookup(&self, key: KeyEvent) -> Option<Action> {
        let k = Key {
            code: key.code,
            modifiers: key.modifiers,
        };
        self.map.get(&k).copied()
    }

    pub fn keycode_to_str(code: &KeyCode) -> Cow<'static, str> {
        match code {
            KeyCode::Char(c) => Cow::Owned(c.to_string()),
            KeyCode::Enter => Cow::Borrowed("Enter"),
            KeyCode::Esc => Cow::Borrowed("Esc"),
            KeyCode::Backspace => Cow::Borrowed("Backspace"),
            KeyCode::Tab => Cow::Borrowed("Tab"),
            KeyCode::Delete => Cow::Borrowed("Delete"),
            KeyCode::Up => Cow::Borrowed("Up"),
            KeyCode::Down => Cow::Borrowed("Down"),
            KeyCode::Left => Cow::Borrowed("Left"),
            KeyCode::Right => Cow::Borrowed("Right"),
            KeyCode::Home => Cow::Borrowed("Home"),
            KeyCode::End => Cow::Borrowed("End"),
            KeyCode::PageUp => Cow::Borrowed("PageUp"),
            KeyCode::PageDown => Cow::Borrowed("PageDown"),
            KeyCode::F(n) => Cow::Owned(format!("F{}", n)),
            KeyCode::Null => Cow::Borrowed("Null"),
            KeyCode::Insert => Cow::Borrowed("Insert"),
            other => Cow::Owned(format!("{:?}", other)),
        }
    }
}
