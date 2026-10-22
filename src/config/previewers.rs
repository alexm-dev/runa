//! Archive preview config for runa
//!
//! Prints path of an archive per line
//! Is used to configurate the archive preview.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::config::input::InputKeyLists;
use crate::utils::command;

#[derive(Deserialize, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Previewers {
    archive: HashMap<String, InputKeyLists>,
}

impl Previewers {
    pub(crate) fn cmd(&self, path: &Path) -> Option<PreviewCommand<'_>> {
        if self.archive.is_empty() {
            return None;
        }

        let lower = path.file_name()?.to_string_lossy().to_lowercase();
        command::extensions(&lower).find_map(|ext| {
            self.archive.get(ext).map(|entry| PreviewCommand {
                args: &entry.0,
                ext_parts: ext.matches('.').count() + 1,
            })
        })
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct PreviewCommand<'a> {
    args: &'a [String],
    ext_parts: usize,
}

impl PreviewCommand<'_> {
    crate::getters! {
        args: &[String],
        ext_parts: usize,
    }
}
