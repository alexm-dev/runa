//! Editor configuration options for runa
//!
//! This module defines the editor configuration options which are read from the runa.toml
//! configuration file.

use std::collections::{HashMap, HashSet};
use std::path::Path;

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
    /// Extensions that {stem} drops when no extensions are matched.
    const LAST_EXTENSION: usize = 1;

    /// Returns the command that opens the path
    /// Checks the file name first then the extension from longest to shorted, then default.
    pub(crate) fn cmd(&self, path: &Path) -> EditorCommand<'_> {
        let Some(name) = path.file_name().map(|name| name.to_string_lossy()) else {
            return EditorCommand::program(&self.default, Self::LAST_EXTENSION);
        };

        if let Some(entry) = self.filename.get(name.as_ref()) {
            return EditorCommand::program(entry, Self::LAST_EXTENSION);
        }

        let lower = name.to_lowercase();
        if let Some(entry) = self.filename.get(&lower) {
            return EditorCommand::program(entry, Self::LAST_EXTENSION);
        }

        for (dot, _) in lower.match_indices('.').filter(|&(dot, _)| dot > 0) {
            let ext = &lower[dot + 1..];
            if let Some(entry) = self.ext.get(ext) {
                let ext_parts = ext.matches('.').count() + 1;
                return EditorCommand::program(entry, ext_parts);
            }
            if self.system.contains(ext) {
                return EditorCommand::System;
            }
        }

        EditorCommand::program(&self.default, Self::LAST_EXTENSION)
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

#[derive(Debug, PartialEq)]
pub(crate) enum EditorCommand<'a> {
    Program {
        args: &'a [String],
        terminal: bool,
        ext_parts: usize,
    },
    System,
}

impl<'a> EditorCommand<'a> {
    fn program(entry: &'a EditorEntry, ext_parts: usize) -> Self {
        EditorCommand::Program {
            args: &entry.args,
            terminal: entry.terminal,
            ext_parts,
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

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    fn program(args: &[String], terminal: bool, ext_parts: usize) -> EditorCommand<'_> {
        EditorCommand::Program {
            args,
            terminal,
            ext_parts,
        }
    }

    #[test]
    fn longest_extension_wins() {
        let editor = editor(&[("gz", &["gunzip"]), ("tar.gz", &["tar"])], &[]);
        let (tar, gunzip) = (strings(&["tar"]), strings(&["gunzip"]));
        assert_eq!(
            editor.cmd(Path::new("/x/release.tar.gz")),
            program(&tar, true, 2)
        );
        assert_eq!(
            editor.cmd(Path::new("/x/access.log.gz")),
            program(&gunzip, true, Editor::LAST_EXTENSION)
        );
        assert_eq!(
            editor.cmd(Path::new("/x/RELEASE.TAR.GZ")),
            program(&tar, true, 2)
        );
    }

    #[test]
    fn hidden_file_has_no_extension() {
        let editor = editor(&[("bashrc", &["code"])], &[]);
        let nvim = strings(&["nvim"]);
        assert_eq!(
            editor.cmd(Path::new("/x/.bashrc")),
            program(&nvim, true, Editor::LAST_EXTENSION)
        );
    }

    #[test]
    fn filename_beats_extension() {
        let editor = editor(&[("toml", &["code"])], &[("Cargo.toml", &["vim"])]);
        let (vim, code) = (strings(&["vim"]), strings(&["code"]));
        assert_eq!(
            editor.cmd(Path::new("/x/Cargo.toml")),
            program(&vim, true, Editor::LAST_EXTENSION)
        );
        assert_eq!(
            editor.cmd(Path::new("/x/rustfmt.toml")),
            program(&code, true, Editor::LAST_EXTENSION)
        );
    }

    #[test]
    fn terminal_is_on_unless_the_table_turns_it_off() -> Result<(), toml::de::Error> {
        let editor: Editor = toml::from_str(
            r#"
            ext = { rs = ["code", "-w"], md = { cmd = "glow" }, zip = { cmd = ["7z", "x", "{path}"], terminal = false } }
            "#,
        )?;

        let code = strings(&["code", "-w"]);
        let glow = strings(&["glow"]);
        let nvim = strings(&["nvim"]);
        let zip = strings(&["7z", "x", "{path}"]);
        assert_eq!(
            editor.cmd(Path::new("/x/main.rs")),
            program(&code, true, Editor::LAST_EXTENSION)
        );
        assert_eq!(
            editor.cmd(Path::new("/x/notes.md")),
            program(&glow, true, Editor::LAST_EXTENSION)
        );
        assert_eq!(
            editor.cmd(Path::new("/x/notes.txt")),
            program(&nvim, true, Editor::LAST_EXTENSION)
        );
        assert_eq!(
            editor.cmd(Path::new("/x/release.zip")),
            program(&zip, false, Editor::LAST_EXTENSION)
        );
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

        let (gunzip, zathura) = (strings(&["gunzip"]), strings(&["zathura"]));
        assert_eq!(
            editor.cmd(Path::new("/x/report.pdf")),
            EditorCommand::System
        );
        assert_eq!(
            editor.cmd(Path::new("/x/release.tar.gz")),
            EditorCommand::System
        );
        assert_eq!(
            editor.cmd(Path::new("/x/access.log.gz")),
            program(&gunzip, true, Editor::LAST_EXTENSION)
        );
        assert_eq!(
            editor.cmd(Path::new("/x/manual.pdf")),
            program(&zathura, true, Editor::LAST_EXTENSION)
        );
        Ok(())
    }
}
