//! Module for mapping file types and names to Nerd Font icons.
//! This module provides functions to retrieve appropriate icons
//! based on file extensions, special filenames, and directory names.
//!
//! It uses `once_cell` for lazy static initialization of icon maps.
//! The icons are represented as static string slices.
//!
//! The main function `nerd_font_icon` takes a `FileEntry` and returns
//! the corresponding Nerd Font icon.

use crate::core::FileEntry;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// File extension to icon mapping
/// This map associates common file extensions with their corresponding
/// Nerd Font icons.
/// For example, "rs" maps to the Rust icon "".
pub static EXT_ICON_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("rs", "");
    m.insert("py", "");
    m.insert("js", "");
    m.insert("md", "");
    m.insert("html", "");
    m.insert("css", "");
    m.insert("json", "");
    m.insert("xml", "");
    m.insert("sh", "");
    m.insert("go", "󰟓");
    m.insert("java", "");
    m.insert("c", "");
    m.insert("cpp", "");
    m.insert("h", "");
    m.insert("hpp", "");
    m.insert("php", "");
    m.insert("rb", "");
    m.insert("swift", "");
    m.insert("kt", "");
    m.insert("lua", "");
    m.insert("ts", "");
    m.insert("tsx", "");
    m.insert("jsx", "");
    m.insert("vue", "");
    m.insert("sql", "");
    m.insert("lock", "");
    m.insert("exe", "");
    m.insert("zip", "");
    m.insert("tar", "");
    m.insert("gz", "");
    m.insert("mp3", "");
    m.insert("mp4", "");
    m.insert("png", "");
    m.insert("jpg", "");
    m.insert("jpeg", "");
    m.insert("gif", "");
    m.insert("svg", "");
    m.insert("pdf", "");
    m.insert("doc", "");
    m.insert("docx", "");
    m.insert("xls", "");
    m.insert("xlsx", "");
    m.insert("ppt", "");
    m.insert("pptx", "");
    m.insert("txt", "");
    m.insert("log", "");
    m.insert("cfg", "");
    m.insert("config", "");
    m.insert("ini", "");
    m.insert("bat", "");
    m.insert("ps1", "󰨊");
    m.insert("cmd", "");
    m.insert("dll", "");
    m.insert("yml", "");
    m.insert("yaml", "");
    m.insert("toml", "");
    m.insert("deb", "");
    m.insert("rpm", "");
    m.insert("dmg", "");
    m.insert("appimage", "");
    m.insert("snap", "");
    m.insert("flatpak", "");
    m.insert("msi", "");
    m.insert("iso", "󰗮");
    m.insert("img", "󰗮");
    m.insert("vhd", "");
    m.insert("cab", "");
    m.insert("psd", "");
    m.insert("patch", "");
    m.insert("diff", "");
    m.insert("ebuild", "");
    m.insert("spec", "");
    m
});

/// Special file names
/// This map associates specific filenames with their corresponding
/// Nerd Font icons.
/// For example, "Cargo.toml" maps to the icon "".
pub static SPECIAL_FILE_ICON_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("README.md", "");
    m.insert("LICENSE", "");
    m.insert("LICENSE.txt", "");
    m.insert("Makefile", "");
    m.insert(".gitignore", "");
    m.insert(".gitconfig", "");
    m.insert("Cargo.toml", "");
    m.insert("Dockerfile", "");
    m.insert("package.json", "");
    m.insert("tsconfig.json", "");
    m.insert("webpack.config.js", "");
    m.insert("Pipfile", "");
    m.insert("requirements.txt", "");
    m.insert("setup.py", "");
    m.insert("config.yaml", "");
    m.insert("config.yml", "");
    m.insert(".env", "");
    m.insert(".env.local", "");
    m.insert(".env.production", "");
    m.insert(".env.development", "");
    m.insert("README", "");
    m.insert("TODO", "");
    m.insert("Dockerfile.dev", "");
    m.insert("Dockerfile.prod", "");
    m.insert("Cargo.lock", "");
    m.insert("CmakeLists.txt", "");
    m.insert("PKGBUILD", "󰣇");
    m.insert(".bashrc", "󱆃");
    m.insert(".vimrc", "");
    m
});

/// Special directory names
/// This map associates specific directory names with their corresponding
/// Nerd Font icons.
/// For example, "node_modules" maps to the icon "".
pub static SPECIAL_DIR_ICON_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("bin", "");
    m.insert("lib", "");
    m.insert("node_modules", "");
    m.insert(".git", "");
    m.insert(".github", "");
    m.insert(".config", "");
    m.insert("nvim", "");
    m
});

/// Get the Nerd Font icon for a given file entry.
/// This function determines the appropriate icon based on whether
/// the entry is a directory or a file, and uses the special
/// filename and extension mappings to find the correct icon.
///
/// # Arguments
/// * `entry` - A reference to a `FileEntry` representing the file or directory.
pub fn nerd_font_icon(entry: &FileEntry) -> &'static str {
    let lowercase_name = entry.lowercase_name();
    let entry_name = entry.name_str();

    if entry.is_dir() {
        if let Some(dir_icon) = SPECIAL_DIR_ICON_MAP.get(lowercase_name) {
            return dir_icon;
        }
        return "";
    }

    if let Some(icon) = SPECIAL_FILE_ICON_MAP.get(entry_name) {
        return icon;
    }

    let ext = match entry_name.rsplit('.').next() {
        Some(ext) if ext != entry_name => ext,
        _ => "",
    };

    if !ext.is_empty() {
        let ext_lc = ext.to_ascii_lowercase();
        if let Some(icon) = EXT_ICON_MAP.get(ext_lc.as_str()) {
            return icon;
        }
    }

    ""
}
