//! Editor configuration options for runa
//!
//! This module defines the editor configuration options which are read from the runa.toml
//! configuration file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::input::InputKeyLists;

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
        ext_parts: usize,
    }

    /// Creats a command that matches no extension, so stem drops the last one.
    fn without_extension(args: &'a [String]) -> Self {
        Self { args, ext_parts: 1 }
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
    fn ext_parts_count_the_matched_extension() {
        let editor = editor(&[("tar.gz", &["tar"])], &[("Cargo.toml", &["vim"])]);
        assert_eq!(editor.cmd(Path::new("/x/release.tar.gz")).ext_parts(), 2);
        assert_eq!(editor.cmd(Path::new("/x/Cargo.toml")).ext_parts(), 1);
        assert_eq!(editor.cmd(Path::new("/x/notes.txt")).ext_parts(), 1);
    }
}
