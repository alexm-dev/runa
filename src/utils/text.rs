//! Text and color rendering utils.

use ratatui::style::Color;

/// Parses a string (color name or hex) into a ratatui::style::color
///
/// Supports standard names (red, green, etc.) as well as hex values (#RRGGBB or #RGB)
pub(crate) fn parse_color(s: &str) -> Color {
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
        "gray" => Color::Gray,
        "darkgray" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        _ => {
            if let Some(color) = s.strip_prefix('#') {
                match color.len() {
                    6 => {
                        if let Ok(rgb) = u32::from_str_radix(color, 16) {
                            return Color::Rgb(
                                ((rgb >> 16) & 0xFF) as u8,
                                ((rgb >> 8) & 0xFF) as u8,
                                (rgb & 0xFF) as u8,
                            );
                        }
                    }
                    3 => {
                        let expanded = color
                            .chars()
                            .map(|c| format!("{}{}", c, c))
                            .collect::<String>();
                        if let Ok(rgb) = u32::from_str_radix(&expanded, 16) {
                            return Color::Rgb(
                                ((rgb >> 16) & 0xFF) as u8,
                                ((rgb >> 8) & 0xFF) as u8,
                                (rgb & 0xFF) as u8,
                            );
                        }
                    }
                    _ => {}
                }
            }
            // fallback
            Color::Reset
        }
    }
}
