use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Display {
    selection_marker: bool,
    dir_marker: bool,
    borders: BorderStyle,
    titles: bool,
    separators: bool,
    origin: bool,
    preview: bool,
    origin_ratio: u16,
    main_ratio: u16,
    preview_ratio: u16,
    preview_underline: bool,
    preview_underline_color: bool,
    scroll_padding: usize,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    None,
    Unified,
    Split,
}

impl Display {
    pub fn selection_marker(&self) -> bool {
        self.selection_marker
    }

    pub fn dir_marker(&self) -> bool {
        self.dir_marker
    }

    pub fn is_unified(&self) -> bool {
        matches!(self.borders, BorderStyle::Unified)
    }

    pub fn is_split(&self) -> bool {
        matches!(self.borders, BorderStyle::Split)
    }

    pub fn titles(&self) -> bool {
        self.titles
    }

    pub fn separators(&self) -> bool {
        self.separators
    }

    pub fn origin(&self) -> bool {
        self.origin
    }

    pub fn preview(&self) -> bool {
        self.preview
    }

    pub fn origin_ratio(&self) -> u16 {
        self.origin_ratio
    }

    pub fn main_ratio(&self) -> u16 {
        self.main_ratio
    }

    pub fn preview_ratio(&self) -> u16 {
        self.preview_ratio
    }

    pub fn preview_underline(&self) -> bool {
        self.preview_underline
    }

    pub fn preview_underline_color(&self) -> bool {
        self.preview_underline_color
    }

    pub fn scroll_padding(&self) -> usize {
        self.scroll_padding
    }
}

impl Default for Display {
    fn default() -> Self {
        Display {
            selection_marker: true,
            dir_marker: true,
            borders: BorderStyle::Unified,
            titles: true,
            separators: true,
            origin: false,
            preview: true,
            origin_ratio: 20,
            main_ratio: 40,
            preview_ratio: 40,
            preview_underline: false,
            preview_underline_color: true,
            scroll_padding: 5,
        }
    }
}
