//! Input configuration options for runa
//!
//! This module defines the input configuration options which are read from the runa.toml
//! configuration file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug)]
struct InputKeyLists(pub Box<[String]>);

impl<'de> Deserialize<'de> for InputKeyLists {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany {
            One(String),
            Many(Box<[String]>),
        }

        Ok(InputKeyLists(match OneOrMany::deserialize(deserializer)? {
            OneOrMany::One(s) => Box::new([s]),
            OneOrMany::Many(v) => v,
        }))
    }
}

macro_rules! define_keys {
    ($($variant:ident => $method:ident = [$($default:expr),*]),+ $(,)?) => {
        #[derive(Deserialize, Debug, Hash, Eq, PartialEq)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum InputKeys {
            $($variant),+
        }

        impl Keys {
            $(
                #[inline]
                pub(crate) fn $method(&self) -> &[String] {
                    self.bindings
                        .get(&InputKeys::$variant)
                        .map(|v| v.0.as_ref())
                        .unwrap_or(&[])
                }
            )+
        }

        impl Default for Keys {
            fn default() -> Self {
                Keys {
                    bindings: HashMap::from([
                        $(
                            (
                                InputKeys::$variant,
                                InputKeyLists(Box::new([$($default.into()),*]))
                            )
                        ),+
                    ])
                }
            }
        }
    };
}

/// Input configuration options of all actions
#[derive(Debug)]
pub(crate) struct Keys {
    bindings: HashMap<InputKeys, InputKeyLists>,
}

impl<'de> Deserialize<'de> for Keys {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let user_bindings = HashMap::<InputKeys, InputKeyLists>::deserialize(deserializer)?;
        let mut keys = Keys::default();
        keys.bindings.extend(user_bindings);
        Ok(keys)
    }
}

define_keys!(
    OpenFile => open_file = ["enter"],
    GoUp => go_up = ["k", "up"],
    GoDown => go_down = ["j", "down"],
    GoParent => go_parent = ["h", "left", "back"],
    GoIntoDir => go_into_dir = ["l", "right"],
    Quit => quit = ["q", "esc"],
    Delete => delete = ["d"],
    Copy => copy = ["y"],
    Paste => paste = ["p"],
    Rename => rename = ["r"],
    Create => create = ["n"],
    CreateDirectory => create_directory = ["N"],
    MoveFile => move_file = ["m"],
    Filter => filter = ["f"],
    ToggleMarker => toggle_marker = ["<space>"],
    ShowInfo => show_info = ["i"],
    Find => find = ["s"],
    ClearMarkers => clear_markers = ["<c-c>"],
    ClearClipboard => clear_clipboard = ["<f2>"],
    ClearFilter => clear_filter = ["<c-f>"],
    ClearAll => clear_all = ["<c-l>"],
    AlternateDelete => alternate_delete = ["<m-d>"],
    SelectAll => select_all = ["<c-a>"],
    PrefixGoTo => prefix_go_to = ["g"],
    GoToTop => go_to_top = ["g"],
    GoToHome => go_to_home = ["h"],
    GoToPath => go_to_path = ["p"],
    GoToBottom => go_to_bottom = ["G"],
    TabNew => tab_new = ["<c-t>"],
    TabClose => tab_close = ["<c-w>"],
    TabNext => tab_next = ["<c-n>"],
    TabPrev => tab_prev = ["<c-p>"],
    ScrollUp => scroll_up = ["pgup"],
    ScrollDown => scroll_down = ["pgdn"],
    Sort => sort = ["o"],
    SortByName => sort_by_name = ["n"],
    SortByNatural => sort_by_natural = ["N"],
    SortByExtension => sort_by_extension = ["e"],
    SortBySize => sort_by_size = ["s"],
    SortByModified => sort_by_modified = ["m"],
    SortByAccessed => sort_by_accessed = ["a"],
    SortByCreated => sort_by_created = ["c"],
    KeybindHelp => keybind_help = ["?"],
    Reload => reload = ["<c-r>"],
);

/// Editor configuration options
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct Editor {
    default: InputKeyLists,
    ext: HashMap<String, InputKeyLists>,
    filename: HashMap<String, InputKeyLists>,
}

/// Public methods for accessing editor configuration options
impl Editor {
    /// Returns the command that opens the path
    /// Checks the file name first then the extension from longest to shorted, then default.
    pub(crate) fn cmd(&self, path: &Path) -> &[String] {
        let Some(name) = path.file_name().map(|name| name.to_string_lossy()) else {
            return &self.default.0;
        };

        if let Some(cmd) = self.filename.get(name.as_ref()) {
            return &cmd.0;
        }

        let lower = name.to_lowercase();
        if let Some(cmd) = self.filename.get(&lower) {
            return &cmd.0;
        }

        for (dot, _) in lower.match_indices('.').filter(|&(dot, _)| dot > 0) {
            if let Some(cmd) = self.ext.get(&lower[dot + 1..]) {
                return &cmd.0;
            }
        }

        &self.default.0
    }

    pub(crate) fn resolved_path(&self, path: &Path) -> Option<PathBuf> {
        let cmd = self.cmd(path);
        let program = cmd.first()?;
        which::which(program).ok()
    }

    pub(crate) fn exists(&self, path: &Path) -> bool {
        self.resolved_path(path).is_some()
    }
}

/// Default editor configuration options
impl Default for Editor {
    fn default() -> Self {
        Editor {
            default: InputKeyLists(vec!["nvim".into()].into_boxed_slice()),
            ext: HashMap::new(),
            filename: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(ext: &[(&str, &str)], filename: &[(&str, &str)]) -> Editor {
        let map = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|&(key, cmd)| (key.to_string(), InputKeyLists(Box::new([cmd.to_string()]))))
                .collect()
        };
        Editor {
            default: InputKeyLists(Box::new(["nvim".to_string()])),
            ext: map(ext),
            filename: map(filename),
        }
    }

    #[test]
    fn longest_extension_wins() {
        let editor = editor(&[("gz", "gunzip"), ("tar.gz", "tar")], &[]);
        assert_eq!(editor.cmd(Path::new("/x/release.tar.gz")), ["tar"]);
        assert_eq!(editor.cmd(Path::new("/x/access.log.gz")), ["gunzip"]);
        assert_eq!(editor.cmd(Path::new("/x/RELEASE.TAR.GZ")), ["tar"]);
    }

    #[test]
    fn hidden_file_has_no_extension() {
        let editor = editor(&[("bashrc", "code")], &[]);
        assert_eq!(editor.cmd(Path::new("/x/.bashrc")), ["nvim"]);
    }

    #[test]
    fn filename_beats_extension() {
        let editor = editor(&[("toml", "code")], &[("Cargo.toml", "vim")]);
        assert_eq!(editor.cmd(Path::new("/x/Cargo.toml")), ["vim"]);
        assert_eq!(editor.cmd(Path::new("/x/rustfmt.toml")), ["code"]);
    }
}
