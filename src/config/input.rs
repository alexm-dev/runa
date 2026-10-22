//! Input configuration options for runa
//!
//! This module defines the input configuration options which are read from the runa.toml
//! configuration file.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// File variables in open commands: the full path, the file name, the parent directory and the file stem.
const FILE_VARIABLES: [&str; 4] = ["{path}", "{name}", "{dir}", "{stem}"];

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
    pub(crate) fn cmd(&self, path: &Path) -> OpenCommand<'_> {
        let Some(name) = path.file_name().map(|name| name.to_string_lossy()) else {
            return OpenCommand::without_extension(&self.default.0);
        };

        if let Some(cmd) = self.filename.get(name.as_ref()) {
            return OpenCommand::without_extension(&cmd.0);
        }

        let lower = name.to_lowercase();
        if let Some(cmd) = self.filename.get(&lower) {
            return OpenCommand::without_extension(&cmd.0);
        }

        for (dot, _) in lower.match_indices('.').filter(|&(dot, _)| dot > 0) {
            let ext = &lower[dot + 1..];
            if let Some(cmd) = self.ext.get(ext) {
                return OpenCommand {
                    args: &cmd.0,
                    ext_parts: ext.matches('.').count() + 1,
                };
            }
        }

        OpenCommand::without_extension(&self.default.0)
    }

    pub(crate) fn resolved_path(&self, path: &Path) -> Option<PathBuf> {
        let cmd = self.cmd(path);
        let program = cmd.args().first()?;
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

/// A command that opens a path, with the number of extensions that stem drops.
pub(crate) struct OpenCommand<'a> {
    args: &'a [String],
    ext_parts: usize,
}

impl<'a> OpenCommand<'a> {
    crate::getters! {
        args: &[String],
    }

    /// Creats a command that matches no extension, so stem drops the last one.
    fn without_extension(args: &'a [String]) -> Self {
        Self { args, ext_parts: 1 }
    }

    /// Returns the arguments after the program, with the file variables filled in.
    pub(crate) fn expand_args(&self, path: &Path) -> Vec<OsString> {
        let rest = self.args.get(1..).unwrap_or_default();
        let has_variable = rest
            .iter()
            .any(|arg| FILE_VARIABLES.iter().any(|fv| arg.contains(fv)));

        if !has_variable {
            let mut args: Vec<OsString> = rest.iter().map(OsString::from).collect();
            args.push(path.as_os_str().to_owned());
            return args;
        }

        let name = path.file_name().unwrap_or_default();
        let values = [
            path.as_os_str(),
            name,
            path.parent().map(Path::as_os_str).unwrap_or_default(),
            Self::strip_extension(name, self.ext_parts),
        ];

        rest.iter()
            .map(|arg| Self::expand_arg(arg, &values))
            .collect()
    }

    /// Returns the name without its last count extension
    fn strip_extension(name: &OsStr, count: usize) -> &OsStr {
        let mut stem = name;
        for _ in 0..count {
            match Path::new(stem).file_stem() {
                Some(shorter) if shorter.len() < stem.len() => stem = shorter,
                _ => break,
            }
        }

        stem
    }

    /// Replaces each file variable in the argument with its value. Unknown braces stay as braces.
    fn expand_arg(arg: &str, values: &[&OsStr; FILE_VARIABLES.len()]) -> OsString {
        let mut expanded = OsString::with_capacity(arg.len());
        let mut rest = arg;

        while let Some(start) = rest.find('{') {
            expanded.push(&rest[..start]);
            let tail = &rest[start..];
            let found = FILE_VARIABLES
                .iter()
                .zip(values)
                .find(|(fv, _)| tail.starts_with(**fv));

            match found {
                Some((variable, value)) => {
                    expanded.push(value);
                    rest = &tail[variable.len()..];
                }
                None => {
                    expanded.push("{");
                    rest = &tail[1..];
                }
            }
        }

        expanded.push(rest);
        expanded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(ext: &[(&str, &[&str])], filename: &[(&str, &[&str])]) -> Editor {
        let map = |pairs: &[(&str, &[&str])]| {
            pairs
                .iter()
                .map(|&(key, cmd)| {
                    let cmd = cmd.iter().map(|arg| arg.to_string()).collect();
                    (key.to_string(), InputKeyLists(cmd))
                })
                .collect()
        };
        Editor {
            default: InputKeyLists(Box::new(["nvim".to_string()])),
            ext: map(ext),
            filename: map(filename),
        }
    }

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn longest_extension_wins() {
        let editor = editor(&[("gz", &["gunzip"]), ("tar.gz", &["tar"])], &[]);
        assert_eq!(editor.cmd(Path::new("/x/release.tar.gz")).args(), ["tar"]);
        assert_eq!(editor.cmd(Path::new("/x/access.log.gz")).args(), ["gunzip"]);
        assert_eq!(editor.cmd(Path::new("/x/RELEASE.TAR.GZ")).args(), ["tar"]);
    }

    #[test]
    fn hidden_file_has_no_extension() {
        let editor = editor(&[("bashrc", &["code"])], &[]);
        assert_eq!(editor.cmd(Path::new("/x/.bashrc")).args(), ["nvim"]);
    }

    #[test]
    fn filename_beats_extension() {
        let editor = editor(&[("toml", &["code"])], &[("Cargo.toml", &["vim"])]);
        assert_eq!(editor.cmd(Path::new("/x/Cargo.toml")).args(), ["vim"]);
        assert_eq!(editor.cmd(Path::new("/x/rustfmt.toml")).args(), ["code"]);
    }

    #[test]
    fn path_is_appended_without_file_variables() {
        let editor = editor(&[("rs", &["nvim", "-p"])], &[]);
        let path = Path::new("/x/main.rs");
        assert_eq!(
            editor.cmd(path).expand_args(path),
            os_args(&["-p", "/x/main.rs"])
        );
    }

    #[test]
    fn file_variables_replace_the_appended_path() {
        let editor = editor(&[("tar.gz", &["tar", "-xf", "{path}", "-C", "{dir}"])], &[]);
        let path = Path::new("/x/release.tar.gz");
        assert_eq!(
            editor.cmd(path).expand_args(path),
            os_args(&["-xf", "/x/release.tar.gz", "-C", "/x"])
        );
    }

    #[test]
    fn stem_drops_the_matched_extension() {
        let editor = editor(&[("tar.gz", &["7z", "x", "-o{stem}", "{name}"])], &[]);
        let path = Path::new("/x/Release.tar.gz");
        assert_eq!(
            editor.cmd(path).expand_args(path),
            os_args(&["x", "-oRelease", "Release.tar.gz"])
        );
    }

    #[test]
    fn stem_without_extension_match_drops_the_last_extension() {
        let editor = editor(&[], &[("Cargo.toml", &["show", "{stem}"])]);
        let path = Path::new("/x/Cargo.toml");
        assert_eq!(editor.cmd(path).expand_args(path), os_args(&["Cargo"]));
    }

    #[test]
    fn unknown_braces_stay_literal() {
        let editor = editor(&[("txt", &["run", "{}", "{foo}", "{path}"])], &[]);
        let path = Path::new("/x/a.txt");
        assert_eq!(
            editor.cmd(path).expand_args(path),
            os_args(&["{}", "{foo}", "/x/a.txt"])
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_is_kept_exact() {
        use std::os::unix::ffi::OsStrExt;

        let editor = editor(&[("zip", &["7z", "x", "{path}", "-o{stem}"])], &[]);
        let path = Path::new(OsStr::from_bytes(b"/x/caf\xe9.zip"));
        let expected = vec![
            OsString::from("x"),
            path.as_os_str().to_owned(),
            OsString::from(OsStr::from_bytes(b"-ocaf\xe9")),
        ];
        assert_eq!(editor.cmd(path).expand_args(path), expected);
    }
}
