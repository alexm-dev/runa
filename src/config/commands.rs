//! Command config options for runa
//!
//! Defines the user commands which are read from runa.toml

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::config::input::InputKeyLists;
use crate::utils::command::RunMode;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserCommand {
    #[serde(skip)]
    name: String,
    keys: InputKeyLists,
    cmd: InputKeyLists,
    terminal: Option<bool>,
}

impl UserCommand {
    crate::getters! {
        name: &str,
    }

    pub(crate) fn keys(&self) -> &[String] {
        &self.keys.0
    }

    pub(crate) fn cmd(&self) -> &[String] {
        &self.cmd.0
    }

    pub(crate) fn run_mode(&self) -> RunMode {
        RunMode::from_terminal(self.terminal.unwrap_or(true))
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(from = "BTreeMap<String, UserCommand>")]
pub(crate) struct Commands {
    list: Vec<UserCommand>,
}

impl From<BTreeMap<String, UserCommand>> for Commands {
    fn from(map: BTreeMap<String, UserCommand>) -> Self {
        let list = map
            .into_iter()
            .map(|(name, mut command)| {
                command.name = name;
                command
            })
            .collect();
        Self { list }
    }
}

impl Commands {
    crate::getters! {
        list: &[UserCommand],
    }

    pub(crate) fn get(&self, index: usize) -> Option<&UserCommand> {
        self.list.get(index)
    }
}
