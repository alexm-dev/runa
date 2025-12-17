use crate::config::Editor;
use ratatui::style::Color;
use std::path::Path;
use std::process::Command;

pub fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "default" | "reset" => Color::Reset,
        "yellow" => Color::Yellow,
        "red" => Color::Red,
        "blue" => Color::Blue,
        "green" => Color::Green,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "black" => Color::Black,
        _ => {
            if let Some(color) = s.strip_prefix('#') {
                if color.len() == 6 {
                    if let Ok(rgb) = u32::from_str_radix(color, 16) {
                        return Color::Rgb(
                            ((rgb >> 16) & 0xFF) as u8,
                            ((rgb >> 8) & 0xFF) as u8,
                            (rgb & 0xFF) as u8,
                        );
                    }
                }
            }
            Color::Reset // Fallback
        }
    }
}

pub fn open_in_editor(editor: &Editor, path: &Path) -> std::io::Result<()> {
    Command::new(&editor.cmd).arg(path).status()?;
    Ok(())
}
