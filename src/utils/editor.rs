//! Editor utilities

use std::fs;
use std::io;
use std::path::Path;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use crate::config::Editor;

/// Opens a specified path/file in the configured editor ("nvim" or "vim" etc.).
///
/// Temporary disables raw mode and exits alternate sceen while the editor runs.
/// On return, restores raw mode and alternate sceen.
pub(crate) fn open_in_editor(editor: &Editor, file_path: &Path) -> std::io::Result<()> {
    let cmd = editor.cmd(file_path);
    let binary = cmd
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No editor command configured"))?;
    let args = &cmd[1..];
    let editor_path = which::which(binary).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Editor '{}' not found", binary),
        )
    })?;

    let resolved_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());

    let mut stdout = io::stdout();
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;

    let mut editor_cmd = std::process::Command::new(editor_path);
    editor_cmd.args(args).arg(file_path);

    let target_dir = if resolved_path.is_dir() {
        Some(resolved_path.as_path())
    } else {
        resolved_path.parent()
    };

    if let Some(dir) = target_dir
        && !dir.as_os_str().is_empty()
    {
        editor_cmd.current_dir(dir);
    }

    let status = editor_cmd.status();

    execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(io::Error::other(format!(
            "Editor exited with status: {}",
            s
        ))),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Command '{}' not found: {}", binary, e),
        )),
    }
}
