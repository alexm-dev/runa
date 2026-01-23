//! Module for mapping file types and names to Nerd Font icons.
//! This module provides functions to retrieve appropriate icons
//! based on file extensions, special filenames, and directory names.
//!
//! The main function `nerd_font_icon` takes a `FileEntry` and returns
//! the corresponding Nerd Font icon.

use crate::core::FileEntry;
use crate::utils::with_lowered_stack;

use phf::phf_map;

/// File extension to icon mapping
/// This map associates common file extensions with their corresponding
/// Nerd Font icons.
/// For example, "rs" maps to the Rust icon "".
static EXT_ICON_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "rs" => "",
    "py" => "",
    "js" => "",
    "md" => "",
    "html" => "",
    "css" => "",
    "json" => "",
    "xml" => "",
    "sh" => "",
    "go" => "󰟓",
    "java" => "",
    "c" => "",
    "cpp" => "",
    "h" => "",
    "hpp" => "",
    "php" => "",
    "rb" => "",
    "swift" => "",
    "kt" => "",
    "lua" => "",
    "ts" => "",
    "tsx" => "",
    "jsx" => "",
    "vue" => "",
    "sql" => "",
    "lock" => "",
    "exe" => "",
    "zip" => "",
    "tar" => "",
    "gz" => "",
    "mp3" => "",
    "mp4" => "",
    "png" => "",
    "jpg" => "",
    "jpeg" => "",
    "gif" => "",
    "svg" => "",
    "pdf" => "",
    "doc" => "",
    "docx" => "",
    "xls" => "",
    "xlsx" => "",
    "ppt" => "",
    "pptx" => "",
    "txt" => "",
    "log" => "",
    "cfg" => "",
    "config" => "",
    "ini" => "",
    "bat" => "",
    "ps1" => "󰨊",
    "cmd" => "",
    "dll" => "",
    "yml" => "",
    "yaml" => "",
    "toml" => "",
    "deb" => "",
    "rpm" => "",
    "dmg" => "",
    "appimage" => "",
    "snap" => "",
    "flatpak" => "",
    "msi" => "",
    "iso" => "󰗮",
    "img" => "󰗮",
    "vhd" => "",
    "cab" => "",
    "psd" => "",
    "patch" => "",
    "diff" => "",
    "ebuild" => "",
    "spec" => "",
};

/// Special file names
/// This map associates specific filenames with their corresponding
/// Nerd Font icons.
/// For example, "Cargo.toml" maps to the icon "".
pub(crate) static SPECIAL_FILE_ICON_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "README.md" => "",
    "LICENSE" => "",
    "LICENSE-MIT" => "",
    "LICENSE-APACHE" => "",
    "COPYING" => "",
    "LICENSE.txt" => "",
    "Makefile" => "",
    ".gitignore" => "",
    ".gitconfig" => "",
    "Cargo.toml" => "",
    "Dockerfile" => "",
    "package.json" => "",
    "tsconfig.json" => "",
    "webpack.config.js" => "",
    "Pipfile" => "",
    "requirements.txt" => "",
    "setup.py" => "",
    "config.yaml" => "",
    "config.yml" => "",
    ".env" => "",
    ".env.local" => "",
    ".env.production" => "",
    ".env.development" => "",
    "README" => "",
    "TODO" => "",
    "Dockerfile.dev" => "",
    "Dockerfile.prod" => "",
    "Cargo.lock" => "",
    "CMakeLists.txt" => "",
    "PKGBUILD" => "󰣇",
    ".bashrc" => "󱆃",
    ".vimrc" => "",
};

/// Special directory names
/// This map associates specific directory names with their corresponding
/// Nerd Font icons.
/// For example, "node_modules" maps to the icon "".
pub(crate) static SPECIAL_DIR_ICON_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "bin" => "",
    "lib" => "",
    "node_modules" => "",
    ".git" => "",
    ".github" => "",
    ".config" => "",
    "nvim" => "",
};

/// Get the Nerd Font icon for a given file entry.
/// This function determines the appropriate icon based on whether
/// the entry is a directory or a file, and uses the special
/// filename and extension mappings to find the correct icon.
pub(crate) fn nerd_font_icon(entry: &FileEntry) -> &'static str {
    let name_str = entry.name_str();
    let name = name_str.as_ref();

    if entry.is_symlink() {
        return if entry.is_dir() { "" } else { "" };
    }

    if entry.is_dir() {
        if let Some(icon) = SPECIAL_DIR_ICON_MAP.get(name.as_ref()) {
            return icon;
        }
        if let Some(icon) = with_lowered_stack(name, |s| SPECIAL_DIR_ICON_MAP.get(s).copied()) {
            return icon;
        }
        return "";
    }

    if let Some(icon) = SPECIAL_FILE_ICON_MAP.get(name.as_ref()) {
        return icon;
    }

    if let Some(dot_idx) = name.rfind('.')
        && dot_idx > 0
        && dot_idx < name.len() - 1
    {
        let ext = &name[dot_idx + 1..];
        if let Some(icon) = EXT_ICON_MAP.get(ext) {
            return icon;
        }

        if let Some(icon) = with_lowered_stack(ext, |s| EXT_ICON_MAP.get(s).copied()) {
            return icon;
        }
    }

    ""
}
