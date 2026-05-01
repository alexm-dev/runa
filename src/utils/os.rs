//! Environment relevant utils.

use std::env;

#[cfg(windows)]
use std::ffi::OsString;

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

#[cfg(windows)]
static PATHEXT_CACHE: OnceLock<Vec<OsString>> = OnceLock::new();

/// Thread safe for getting home_dir once.
#[inline]
pub(crate) fn get_home() -> Option<&'static PathBuf> {
    HOME_DIR_CACHE.get_or_init(dirs::home_dir).as_ref()
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

pub(crate) fn command_exists(cmd: &str) -> bool {
    resolve_command(cmd).is_ok()
}

pub(crate) fn resolve_command(cmd: &str) -> io::Result<PathBuf> {
    let candidate = Path::new(cmd);
    if candidate.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Command name cannot be empty",
        ));
    }

    if candidate.components().count() > 1 || candidate.is_absolute() {
        return resolve_command_candidate(candidate).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Command '{}' not found", cmd),
            )
        });
    }

    let path_var = env::var_os("PATH").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Command '{}' not found (PATH is empty)", cmd),
        )
    })?;

    for dir in env::split_paths(&path_var) {
        let full = dir.join(candidate);
        if let Some(found) = resolve_command_candidate(&full) {
            return Ok(found);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Command '{}' not found", cmd),
    ))
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
        .get_or_init(|| binaries.iter().find(|&&bin| command_exists(bin)).copied())
        .as_ref()
        .copied()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, err_msg))
}

fn resolve_command_candidate(path: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if path.extension().is_some() {
            return is_executable_file(path).then(|| path.to_path_buf());
        }

        for ext in windows_path_exts() {
            let mut candidate = OsString::from(path.as_os_str());
            candidate.push(ext);
            let candidate = PathBuf::from(candidate);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    #[cfg(not(windows))]
    {
        is_executable_file(path).then(|| path.to_path_buf())
    }
}

#[cfg(windows)]
fn windows_path_exts() -> &'static [OsString] {
    PATHEXT_CACHE.get_or_init(|| {
        if let Some(exts) = env::var_os("PATHEXT") {
            let parsed = exts
                .to_string_lossy()
                .split(';')
                .filter_map(|ext| {
                    let ext = ext.trim();
                    if ext.is_empty() {
                        return None;
                    }
                    let ext = ext.to_ascii_uppercase();
                    Some(if ext.starts_with('.') {
                        OsString::from(ext)
                    } else {
                        OsString::from(format!(".{ext}"))
                    })
                })
                .collect::<Vec<_>>();
            if !parsed.is_empty() {
                return parsed;
            }
        }

        vec![
            OsString::from(".COM"),
            OsString::from(".EXE"),
            OsString::from(".BAT"),
            OsString::from(".CMD"),
        ]
    })
}

fn is_executable_file(path: &Path) -> bool {
    let md = match std::fs::metadata(path) {
        Ok(md) => md,
        Err(_) => return false,
    };
    if !md.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        md.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}
