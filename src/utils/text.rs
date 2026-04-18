//! Text and color rendering utils.

use ratatui::style::Color;

#[derive(Debug)]
pub(crate) struct StrBuffer {
    data: String,
    offset: Vec<u32>,
}

impl StrBuffer {
    pub(crate) fn from_iter<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();

        let mut data = String::with_capacity(lower * 12);
        let mut offset = Vec::with_capacity(lower);

        for s in iter {
            let s = s.as_ref();
            offset.push(data.len() as u32);
            data.push_str(s);
        }
        Self { data, offset }
    }

    pub(crate) fn len(&self) -> usize {
        self.offset.len()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &str> {
        (0..self.len()).map(move |i| self.get(i))
    }

    pub(crate) fn get(&self, index: usize) -> &str {
        let start = self.offset[index] as usize;
        let end = self
            .offset
            .get(index + 1)
            .map(|&v| v as usize)
            .unwrap_or(self.data.len());
        &self.data[start..end]
    }
}

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
