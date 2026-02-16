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
    GoToTop,
    GoToBottom,
    GoToPath,
    GoToHome,
    ToggleMarker,
    ClearMarker,
    ClearFilter,
    ClearAll,
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
    ClearClipboard,
}

/// System actions (quit)
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum SystemAction {
    Quit,
    KeyBindHelp,
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
    #[rustfmt::skip]
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
                bind_prefix($keys, $prefix, &mut gmap);
            };
        }

        use NavAction as N;
        use FileAction as F;
        use SystemAction as S;

        // NavActions
        bind!(keys.go_parent(),         Action::Nav(N::GoParent));
        bind!(keys.go_into_dir(),       Action::Nav(N::GoIntoDir));
        bind!(keys.go_up(),             Action::Nav(N::GoUp));
        bind!(keys.go_down(),           Action::Nav(N::GoDown));
        bind!(keys.toggle_marker(),     Action::Nav(N::ToggleMarker));
        bind!(keys.clear_filter(),      Action::Nav(N::ClearFilter));
        bind!(keys.clear_markers(),     Action::Nav(N::ClearMarker));
        bind!(keys.clear_all(),         Action::Nav(N::ClearAll));
        bind!(keys.go_to_bottom(),      Action::Nav(N::GoToBottom));

        // FileActions
        bind!(keys.open_file(),         Action::File(F::Open));
        bind!(keys.delete(),            Action::File(F::Delete));
        bind!(keys.copy(),              Action::File(F::Copy));
        bind!(keys.paste(),             Action::File(F::Paste));
        bind!(keys.move_file(),         Action::File(F::MoveFile));
        bind!(keys.rename(),            Action::File(F::Rename));
        bind!(keys.create(),            Action::File(F::Create));
        bind!(keys.create_directory(),  Action::File(F::CreateDirectory));
        bind!(keys.filter(),            Action::File(F::Filter));
        bind!(keys.show_info(),         Action::File(F::ShowInfo));
        bind!(keys.find(),              Action::File(F::Find));
        bind!(keys.clear_clipboard(),   Action::File(F::ClearClipboard));
        bind!(keys.alternate_delete(),  Action::File(F::AlternateDelete));

        // SystemActions
        bind!(keys.keybind_help(),      Action::System(S::KeyBindHelp));
        bind!(keys.quit(),              Action::System(S::Quit));

        // Prefix actions
        bind_prefix!(keys.go_to_top(),  Action::Nav(N::GoToTop),  PrefixCommand::Nav(N::GoToTop));
        bind_prefix!(keys.go_to_home(), Action::Nav(N::GoToHome), PrefixCommand::Nav(N::GoToHome));
        bind_prefix!(keys.go_to_path(), Action::Nav(N::GoToPath), PrefixCommand::Nav(N::GoToPath));

        Keymap { map, gmap }
    }

    /// Looks up the action for a given key event
    pub(crate) fn lookup(&self, key: KeyEvent) -> Option<Action> {
        let k = Key {
            code: key.code,
            modifiers: key.modifiers,
        };

        if let Some(action) = self.map.get(&k).copied() {
            return Some(action);
        }

        if matches!(key.code, KeyCode::Char(_)) && key.modifiers.contains(KeyModifiers::SHIFT) {
            let k2 = Key {
                code: key.code,
                modifiers: key.modifiers - KeyModifiers::SHIFT,
            };
            return self.map.get(&k2).copied();
        }
        None
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

    #[inline]
    pub(crate) fn started_prefix(&self) -> bool {
        self.started
    }

    #[inline]
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

    let is_bracketed = s.starts_with('<') && s.ends_with('>');
    let mut input = s.trim_start_matches('<').trim_end_matches('>').to_string();

    if is_bracketed && input.contains('-') {
        let parts: Vec<&str> = input.split('-').collect();

        for &prefix in parts.iter().take(parts.len().saturating_sub(1)) {
            match prefix.to_lowercase().as_str() {
                "c" | "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "a" | "m" | "alt" => modifiers |= KeyModifiers::ALT,
                "s" | "shift" => modifiers |= KeyModifiers::SHIFT,
                _ => return None,
            }
        }
        input = parts.last()?.to_string();
    }

    let normalized = input.replace('-', "+");
    for part in normalized.split('+') {
        let p_low = part.to_lowercase();
        match p_low.as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "meta" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,

            "up" => code = Some(KeyCode::Up),
            "down" => code = Some(KeyCode::Down),
            "left" => code = Some(KeyCode::Left),
            "right" => code = Some(KeyCode::Right),
            "enter" => code = Some(KeyCode::Enter),
            "esc" => code = Some(KeyCode::Esc),
            "backspace" | "back" => code = Some(KeyCode::Backspace),
            "tab" => code = Some(KeyCode::Tab),
            "space" | "spc" => code = Some(KeyCode::Char(' ')),

            _ => {
                if part.len() == 1 {
                    let mut c = part.chars().next()?;
                    if modifiers.contains(KeyModifiers::SHIFT) {
                        c = c.to_ascii_uppercase();
                    }
                    code = Some(KeyCode::Char(c));
                } else if p_low.starts_with('f')
                    && p_low.len() > 1
                    && p_low[1..].chars().all(|c| c.is_ascii_digit())
                {
                    let n = p_low[1..].parse().ok()?;
                    code = Some(KeyCode::F(n));
                } else if part.is_empty() {
                    continue;
                } else {
                    return None;
                }
            }
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
    prefix: PrefixCommand,
    gmap: &mut HashMap<KeyCode, PrefixCommand>,
) {
    for k in key_list {
        if let Some(key) = parse_key(k)
            && key.modifiers.is_empty()
            && let KeyCode::Char(c) = key.code
        {
            gmap.insert(KeyCode::Char(c), prefix);
        }
    }
}
