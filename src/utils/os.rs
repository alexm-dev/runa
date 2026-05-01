//! Environment relevant utils.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Shared cache for the home_dir dirs call
static HOME_DIR_CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// A OnceLock to store the fd binary name.
/// This is used to avoid checking for the binary multiple times.
static FD_BIN: OnceLock<Option<&'static str>> = OnceLock::new();

/// OnceLock cache to store the bat binary name.
/// This is used to avoid checking for the binary multiple times.
/// If bat is not found, the value will be None.
static BAT_BIN: OnceLock<Option<&'static str>> = OnceLock::new();

/// Thread safe for getting home_dir once.
#[inline]
pub(crate) fn get_home() -> Option<&'static PathBuf> {
    HOME_DIR_CACHE.get_or_init(home::home_dir).as_ref()
}

#[inline]
pub(crate) fn fd_binary() -> io::Result<&'static str> {
    cached_binary(&FD_BIN, &["fd", "fd-find"], "fd/fd-find not found")
}

#[inline]
pub(crate) fn bat_binary() -> io::Result<&'static str> {
    cached_binary(&BAT_BIN, &["bat"], "bat not found")
}

pub(crate) fn default_config_path() -> PathBuf {
    std::env::var_os("RUNA_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME").map(|s| PathBuf::from(s).join("runa/runa.toml"))
        })
        .or_else(|| get_home().map(|h| h.join(".config/runa/runa.toml")))
        .unwrap_or_else(|| PathBuf::from("runa.toml"))
}

pub(crate) fn is_regular_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        std::fs::symlink_metadata(path)
            .map(|md| md.file_type().is_file())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn cached_binary(
    cache: &'static OnceLock<Option<&'static str>>,
    binaries: &[&'static str],
    err_msg: &'static str,
) -> io::Result<&'static str> {
    cache
        .get_or_init(|| {
            binaries
                .iter()
                .find(|&&bin| which::which(bin).is_ok())
                .copied()
        })
        .as_ref()
        .copied()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, err_msg))
}
