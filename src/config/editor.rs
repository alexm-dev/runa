//! Editor configuration options for runa
//!
//! This module defines the editor configuration options which are read from the runa.toml
//! configuration file.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::input::InputKeyLists;

/// An editor command and whether it takes over the terminal.
#[derive(Debug)]
struct EditorEntry {
    args: Box<[String]>,
    terminal: bool,
}

/// The table form of an editor command: `{ cmd = [...], terminal = false }`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditorTable {
    cmd: InputKeyLists,
    terminal: Option<bool>,
}

impl<'de> Deserialize<'de> for EditorEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Entry {
            Command(InputKeyLists),
            Table(EditorTable),
        }

        Ok(match Entry::deserialize(deserializer)? {
            Entry::Command(cmd) => EditorEntry {
                args: cmd.0,
                terminal: true,
            },
            Entry::Table(table) => EditorEntry {
                args: table.cmd.0,
                terminal: table.terminal.unwrap_or(true),
            },
        })
    }
}

/// Editor configuration options
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct Editor {
    default: EditorEntry,
    ext: HashMap<String, EditorEntry>,
    filename: HashMap<String, EditorEntry>,
    system: HashSet<String>,
}

/// Public methods for accessing editor configuration options
impl Editor {
    /// Returns the command that opens the path
    /// Checks the file name first then the extension from longest to shorted, then default.
    pub(crate) fn cmd(&self, path: &Path) -> EditorCommand<'_> {
        let Some(name) = path.file_name().map(|name| name.to_string_lossy()) else {
            return EditorCommand::without_extension(&self.default);
        };

        if let Some(entry) = self.filename.get(name.as_ref()) {
            return EditorCommand::without_extension(entry);
        }

        let lower = name.to_lowercase();
        if let Some(entry) = self.filename.get(&lower) {
            return EditorCommand::without_extension(entry);
        }

        for (dot, _) in lower.match_indices('.').filter(|&(dot, _)| dot > 0) {
            let ext = &lower[dot + 1..];
            let ext_parts = ext.matches('.').count() + 1;
            if let Some(entry) = self.ext.get(ext) {
                return EditorCommand::new(entry, ext_parts);
            }
            if self.system.contains(ext) {
                return EditorCommand::with_system_app(ext_parts);
            }
        }

        EditorCommand::without_extension(&self.default)
    }

    pub(crate) fn resolved_path(&self, path: &Path) -> Option<PathBuf> {
        let cmd = self.cmd(path);
        let program = cmd.args().first()?;
        which::which(program).ok()
    }
}

/// Default editor configuration options
impl Default for Editor {
    fn default() -> Self {
        Editor {
            default: EditorEntry {
                args: Box::new(["nvim".into()]),
                terminal: true,
            },
            ext: HashMap::new(),
            filename: HashMap::new(),
            system: HashSet::new(),
        }
    }
}

/// A command that opens a path, with the number of extensions that stem drops.
pub(crate) struct EditorCommand<'a> {
    args: &'a [String],
    terminal: bool,
    ext_parts: usize,
    system: bool,
}

impl<'a> EditorCommand<'a> {
    crate::getters! {
        args: &[String],
        terminal: bool,
        ext_parts: usize,
        system: bool,
    }

    fn new(entry: &'a EditorEntry, ext_parts: usize) -> Self {
        Self {
            args: &entry.args,
            terminal: entry.terminal,
            ext_parts,
            system: false,
        }
    }

    /// Creats a command that matches no extension, so stem drops the last one.
    fn without_extension(entry: &'a EditorEntry) -> Self {
        Self::new(entry, 1)
    }

    /// Opens the path with the default app of the system.
    fn with_system_app(ext_parts: usize) -> Self {
        Self {
            args: &[],
            terminal: false,
            ext_parts,
            system: true,
        }
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
                    let entry = EditorEntry {
                        args: cmd.iter().map(|arg| arg.to_string()).collect(),
                        terminal: true,
                    };
                    (key.to_string(), entry)
                })
                .collect()
        };
        Editor {
            ext: map(ext),
            filename: map(filename),
            ..Editor::default()
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

    #[test]
    fn terminal_is_on_unless_the_table_turns_it_off() -> Result<(), toml::de::Error> {
        let editor: Editor = toml::from_str(
            r#"
            ext = { rs = ["code", "-w"], md = { cmd = "glow" }, zip = { cmd = ["7z", "x", "{path}"], terminal = false } }
            "#,
        )?;

        assert!(editor.cmd(Path::new("/x/main.rs")).terminal());
        assert!(editor.cmd(Path::new("/x/notes.md")).terminal());
        assert!(editor.cmd(Path::new("/x/notes.txt")).terminal());

        let zip = editor.cmd(Path::new("/x/release.zip"));
        assert!(!zip.terminal());
        assert_eq!(zip.args(), ["7z", "x", "{path}"]);
        Ok(())
    }

    #[test]
    fn unknown_table_fields_are_rejected() {
        let result: Result<Editor, _> =
            toml::from_str(r#"ext = { zip = { cmd = ["7z"], termnal = false } }"#);
        assert!(result.is_err());
    }

    #[test]
    fn system_list_opens_with_the_default_app() -> Result<(), toml::de::Error> {
        let editor: Editor = toml::from_str(
            r#"
            system = ["pdf", "tar.gz"]
            ext = { gz = "gunzip" }
            filename = { "manual.pdf" = "zathura" }
            "#,
        )?;

        let pdf = editor.cmd(Path::new("/x/report.pdf"));
        assert!(pdf.system());
        assert!(!pdf.terminal());
        assert!(editor.cmd(Path::new("/x/release.tar.gz")).system());
        assert!(!editor.cmd(Path::new("/x/access.log.gz")).system());
        assert!(!editor.cmd(Path::new("/x/manual.pdf")).system());
        assert!(!editor.cmd(Path::new("/x/notes.txt")).system());
        Ok(())
    }
}
