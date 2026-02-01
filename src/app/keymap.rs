//! Key mapping and action dispatch system for runa
//!
//! Defines key to an action, parsing from the config, and enum variants
//! for all navigation, file and actions used by runa.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Represents any action in the app: navigation, file, or system.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Action {
    Nav(NavAction),
    File(FileAction),
    System(SystemAction),
}

/// Navigation actions (move, into_parent, markers, etc.)
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum NavAction {
    GoParent,
    GoIntoDir,
    GoUp,
    GoDown,
    GoToPath,
    GoToTop,
    ToggleMarker,
    ClearMarker,
    ClearFilter,
}

/// File actions (delete, copy, open, paste, etc.)
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum FileAction {
    Delete,
    Copy,
    Open,
    Paste,
    Rename,
    Create,
    CreateDirectory,
    Filter,
    ShowInfo,
    Find,
    MoveFile,
    AlternateDelete,
}

/// System actions (quit)
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum SystemAction {
    Quit,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum PrefixCommand {
    Nav(NavAction),
}

/// Key + modifiers as used in keybind/keymap
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub(crate) struct Key {
    pub(crate) code: KeyCode,
    pub(crate) modifiers: KeyModifiers,
}

/// Stores the mapping from Key to action, which is built in the config
pub(crate) struct Keymap {
    map: HashMap<Key, Action>,
    gmap: HashMap<KeyCode, PrefixCommand>,
}

impl Keymap {
    /// Builds the keymap from the config
    pub(crate) fn from_config(config: &crate::config::Config) -> Self {
        let mut map = HashMap::new();
        let mut gmap = HashMap::new();
        let keys = config.keys();

        macro_rules! bind {
            ($keys:expr, $action:expr) => {
                bind($keys, $action, &mut map);
            };
        }

        macro_rules! bind_prefix {
            ($keys:expr, $action:expr, $prefix:expr) => {
                bind_prefix($keys, $action, $prefix, &mut map, &mut gmap);
            };
        }

        bind!(keys.go_parent(), Action::Nav(NavAction::GoParent));
        bind!(keys.go_into_dir(), Action::Nav(NavAction::GoIntoDir));
        bind!(keys.go_up(), Action::Nav(NavAction::GoUp));
        bind!(keys.go_down(), Action::Nav(NavAction::GoDown));
        bind!(keys.toggle_marker(), Action::Nav(NavAction::ToggleMarker));
        bind!(keys.open_file(), Action::File(FileAction::Open));
        bind!(keys.delete(), Action::File(FileAction::Delete));
        bind!(keys.copy(), Action::File(FileAction::Copy));
        bind!(keys.paste(), Action::File(FileAction::Paste));
        bind!(keys.move_file(), Action::File(FileAction::MoveFile));
        bind!(keys.rename(), Action::File(FileAction::Rename));
        bind!(keys.create(), Action::File(FileAction::Create));
        bind!(
            keys.create_directory(),
            Action::File(FileAction::CreateDirectory)
        );
        bind!(keys.filter(), Action::File(FileAction::Filter));
        bind!(keys.quit(), Action::System(SystemAction::Quit));
        bind!(keys.show_info(), Action::File(FileAction::ShowInfo));
        bind!(keys.find(), Action::File(FileAction::Find));
        bind!(keys.clear_markers(), Action::Nav(NavAction::ClearMarker));
        bind!(keys.clear_filter(), Action::Nav(NavAction::ClearFilter));
        bind!(
            keys.alternate_delete(),
            Action::File(FileAction::AlternateDelete)
        );

        bind_prefix!(
            keys.go_to_top(),
            Action::Nav(NavAction::GoToTop),
            PrefixCommand::Nav(NavAction::GoToTop)
        );

        bind_prefix!(
            keys.go_to_path(),
            Action::Nav(NavAction::GoToPath),
            PrefixCommand::Nav(NavAction::GoToPath)
        );

        Keymap { map, gmap }
    }

    /// Looks up the action for a given key event
    pub(crate) fn lookup(&self, key: KeyEvent) -> Option<Action> {
        let k = Key {
            code: key.code,
            modifiers: key.modifiers,
        };
        self.map.get(&k).copied()
    }

    pub(crate) fn gmap(&self) -> &HashMap<KeyCode, PrefixCommand> {
        &self.gmap
    }
}

pub(crate) struct KeyPrefix {
    state: PrefixState,
    last_time: Option<Instant>,
    timeout: Duration,
    started: bool,
    exited: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum PrefixState {
    None,
    G,
}

impl KeyPrefix {
    pub(crate) fn new(timeout: Duration) -> Self {
        Self {
            state: PrefixState::None,
            last_time: None,
            timeout,
            started: false,
            exited: false,
        }
    }

    pub(crate) fn feed(
        &mut self,
        key: &KeyEvent,
        gmap: &HashMap<KeyCode, PrefixCommand>,
    ) -> Option<PrefixCommand> {
        self.started = false;
        self.exited = false;
        let now = Instant::now();
        match self.state {
            PrefixState::None => {
                if key.code == KeyCode::Char('g') && key.modifiers.is_empty() {
                    self.state = PrefixState::G;
                    self.last_time = Some(now);
                    self.started = true;
                    None
                } else {
                    None
                }
            }
            PrefixState::G => {
                let elapsed = self
                    .last_time
                    .map_or(Duration::MAX, |t| now.duration_since(t));
                self.state = PrefixState::None;
                self.last_time = None;
                self.exited = true;
                if elapsed <= self.timeout {
                    gmap.get(&key.code).copied()
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn started_prefix(&self) -> bool {
        self.started
    }

    pub(crate) fn exited_prefix(&self) -> bool {
        self.exited
    }

    pub(crate) fn is_g_state(&self) -> bool {
        self.state == PrefixState::G
    }

    pub(crate) fn expired(&self) -> bool {
        self.state == PrefixState::G
            && self
                .last_time
                .is_some_and(|time| time.elapsed() >= self.timeout)
    }

    pub(crate) fn cancel(&mut self) {
        self.state = PrefixState::None;
        self.last_time = None;
        self.exited = true;
    }
}

fn parse_key(s: &str) -> Option<Key> {
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
}

fn bind(key_list: &[String], action: Action, map: &mut HashMap<Key, Action>) {
    for k in key_list {
        if let Some(key) = parse_key(k) {
            map.insert(key, action);
        }
    }
}

fn bind_prefix(
    key_list: &[String],
    action: Action,
    prefix: PrefixCommand,
    map: &mut HashMap<Key, Action>,
    gmap: &mut HashMap<KeyCode, PrefixCommand>,
) {
    for k in key_list {
        if let Some(key) = parse_key(k) {
            map.insert(key, action);
            if key.modifiers.is_empty()
                && let KeyCode::Char(c) = key.code
            {
                gmap.insert(KeyCode::Char(c), prefix);
            }
        }
    }
}
