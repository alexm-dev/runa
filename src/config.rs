use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub dirs_first: bool,
    pub show_hidden: bool,
    pub case_insensitive: bool,
}

impl Config {
    pub fn load(path: &str) -> Self {
        match fs::read_to_string(path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
        {
            Some(cfg) => cfg,
            None => {
                println!("Config file missing or invalid, using defaults.");
                Config::default()
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dirs_first: true,
            show_hidden: false,
            case_insensitive: false,
        }
    }
}
