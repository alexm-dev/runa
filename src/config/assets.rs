//! Default `runa.toml` configuration assets used by config/load and cli.

pub(crate) const FULL_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/config/runa_full.toml"
));

pub(crate) const MINIMAL_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/config/runa_minimal.toml"
));
