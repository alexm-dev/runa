//! Module for mapping file types and names to Nerd Font icons.
//! This module provides functions to retrieve appropriate icons
//! based on file extensions, special filenames, and directory names.
//!
//! The main function `nerd_font_icon` takes a `FileEntry` and returns
//! the corresponding Nerd Font icon.

use crate::config::Theme;
use crate::core::FileEntry;
use crate::utils::parse_color;
use ratatui::style::Color;

use phf::phf_map;

/// File extension to icon mapping
/// This map associates common file extensions with their corresponding
/// Nerd Font icons.
/// For example, "rs" maps to the Rust icon "".
pub(crate) static EXT_ICON_MAP: phf::Map<&'static str, (&'static str, Option<&'static str>)> = phf_map! {
    "rs"    => ("", Some("#dea584")),
    "rlib"  => ("", Some("#dea584")),
    "py"    => ("", Some("#3572a5")),
    "js"    => ("", Some("#f1e05a")),
    "ts"    => ("", Some("#3178c6")),
    "tsx"   => ("", Some("#61dafb")),
    "jsx"   => ("", Some("#61dafb")),
    "go"    => ("󰟓", Some("#00add8")),
    "java"  => ("", Some("#cc2e2d")),
    "lua"   => ("", Some("#51a0cf")),
    "php"   => ("", Some("#777bb4")),
    "rb"    => ("", Some("#701516")),
    "html"  => ("", Some("#e34c26")),
    "css"   => ("", Some("#563d7c")),
    "swift" => ("", Some("#f05138")),
    "kt"    => ("", Some("#7f52ff")),
    "json"  => ("", Some("#cbcb41")),
    "toml"  => ("", Some("#9c4221")),
    "yaml"  => ("", None),
    "yml"   => ("", None),
    "xml"   => ("", None),
    "sql"   => ("", Some("#dad8d8")),
    "lock"  => ("", Some("#bbbbbb")),
    "sh"    => ("", Some("#4d5a5e")),
    "bash"  => ("", Some("#4d5a5e")),
    "zsh"   => ("", Some("#4d5a5e")),
    "fish"  => ("", Some("#4d5a5e")),
    "md"    => ("", None),
    "txt"   => ("", None),
    "pdf"   => ("", Some("#ff0000")),
    "png"   => ("", Some("#a074c4")),
    "jpg"   => ("", Some("#a074c4")),
    "jpeg"  => ("", Some("#a074c4")),
    "gif"   => ("", Some("#a074c4")),
    "svg"   => ("", Some("#ffb13b")),
    "zip"   => ("", Some("#f9ae28")),
    "tar"   => ("", Some("#f9ae28")),
    "gz"    => ("", Some("#f9ae28")),
    "c"     => ("", None),
    "cpp"   => ("", None),
    "h"     => ("", None),
    "hpp"   => ("", None),
    "exe"   => ("", None),
    "bat"   => ("", None),
    "ps1"   => ("󰨊", None),
    "cmd"   => ("", None),
    "deb"   => ("", None),
    "rpm"   => ("", None),
    "dmg"   => ("", None),
    "appimage" => ("", None),
    "snap"  => ("", None),
    "flatpak" => ("", None),
    "msi"   => ("", None),
    "iso"   => ("󰗮", None),
    "img"   => ("󰗮", None),
    "vhd"   => ("", None),
    "cab"   => ("", None),
    "psd"   => ("", None),
    "patch" => ("", None),
    "diff"  => ("", None),
    "ebuild" => ("", None),
    "spec"  => ("", None),
    "dll"   => ("", None),
    "a"     => ("", None),
    "so"    => ("", None),
    "lib"   => ("", None),
    "o"     => ("", None),
    "d"     => ("", None),
};

/// Special file names
/// This map associates specific filenames with their corresponding
/// Nerd Font icons.
pub(crate) static SPECIAL_FILE_ICON_MAP: phf::Map<
    &'static str,
    (&'static str, Option<&'static str>),
> = phf_map! {
    "README.md"          => ("", None),
    "LICENSE"            => ("", Some("#cbcb41")),
    "LICENSE-MIT"        => ("", Some("#cbcb41")),
    "LICENSE-APACHE"     => ("", Some("#cbcb41")),
    "COPYING"            => ("", Some("#cbcb41")),
    "LICENSE.txt"        => ("", Some("#cbcb41")),
    "LICENSE-MIT.txt"    => ("", Some("#cbcb41")),
    "LICENSE-APACHE.txt" => ("", Some("#cbcb41")),
    "COPYING.txt"        => ("", Some("#cbcb41")),
    "LICENSE.md"         => ("", Some("#cbcb41")),
    "CHANGELOG"          => ("", None),
    "CHANGELOG.md"       => ("", None),
    "CHANGELOG.txt"      => ("", None),
    "SECURITY"           => ("󰒃", Some("#ed333b")),
    "SECURITY.md"        => ("󰒃", Some("#ed333b")),
    "TODO"               => ("", Some("#ffb13b")),
    "Makefile"           => ("", Some("#6d8086")),
    "CMakeLists.txt"     => ("", Some("#064f8c")),
    ".gitignore"         => ("", Some("#f14e32")),
    ".gitconfig"         => ("", Some("#f14e32")),
    "PKGBUILD"           => ("󰣇", Some("#1793d1")),
    "Cargo.toml"         => ("", Some("#dea584")),
    "Cargo.lock"         => ("", Some("#bbbbbb")),
    "package.json"       => ("", Some("#8bc0d0")),
    "tsconfig.json"      => ("", Some("#3178c6")),
    "webpack.config.js"  => ("", Some("#8bc0d0")),
    "Pipfile"            => ("", Some("#3572a5")),
    "requirements.txt"   => ("", Some("#3572a5")),
    "setup.py"           => ("", Some("#3572a5")),
    "Dockerfile"         => ("", Some("#384d54")),
    "Dockerfile.dev"     => ("", Some("#384d54")),
    "Dockerfile.prod"    => ("", Some("#384d54")),
    ".env"               => ("", Some("#faf77e")),
    ".env.local"         => ("", Some("#faf77e")),
    ".env.production"    => ("", Some("#faf77e")),
    ".env.development"   => ("", Some("#faf77e")),
    "config.yaml"        => ("", None),
    "config.yml"         => ("", None),
    ".bashrc"            => ("󱆃", Some("#4d5a5e")),
    ".vimrc"             => ("", Some("#019833")),
};

/// Special directory names
/// This map associates specific directory names with their corresponding
/// Nerd Font icons.
pub(crate) static SPECIAL_DIR_ICON_MAP: phf::Map<
    &'static str,
    (&'static str, Option<&'static str>),
> = phf_map! {
    "Desktop"      => ("󰍹", Some("#43a047")),
    "Documents"    => ("󱔗", Some("#1e88e5")),
    "Downloads"    => ("", Some("#1e88e5")),
    "Pictures"     => ("󰉔", Some("#8e24aa")),
    "Music"        => ("󱍙", Some("#fb8c00")),
    "Videos"       => ("", Some("#e53935")),
    "lib"          => ("", Some("#78909c")),
    "node_modules" => ("", Some("#388e3c")),
    ".git"         => ("", Some("#f14e32")),
    ".github"      => ("", None),
    ".config"      => ("", Some("#546e7a")),
    "nvim"         => ("", Some("#50a044")),
};

/// Get the Nerd Font icon for a given file entry.
pub(crate) fn nerd_font_icon(entry: &FileEntry, theme: &Theme) -> (&'static str, Option<Color>) {
    let name_str = entry.name_str();
    let is_dir = entry.is_dir();

    if entry.is_symlink() {
        return if is_dir { ("", None) } else { ("", None) };
    }

    #[cfg(unix)]
    if entry.is_executable() && !is_dir {
        return ("", Some(theme.exe_color()));
    }

    let ext = entry.ext();
    let lookup = if is_dir {
        SPECIAL_DIR_ICON_MAP.get(name_str)
    } else {
        SPECIAL_FILE_ICON_MAP
            .get(name_str)
            .or_else(|| ext.and_then(|e| EXT_ICON_MAP.get(e)))
    };

    let icon = lookup
        .map(|(i, _)| *i)
        .unwrap_or(if is_dir { "" } else { "" });

    let color = theme
        .icon_color()
        .get(name_str)
        .or_else(|| ext.and_then(|e| theme.icon_color().get(e)))
        .copied()
        .or_else(|| lookup.and_then(|(_, hex)| hex.map(parse_color)));

    (icon, color)
}
