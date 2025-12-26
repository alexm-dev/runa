use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct LayoutConfig {
    parent: u16,
    main: u16,
    preview: u16,
}

impl LayoutConfig {
    fn parent_ratio(&self) -> u16 {
        self.parent
    }

    fn main_ratio(&self) -> u16 {
        self.main
    }

    fn preview_ratio(&self) -> u16 {
        self.preview
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Display {
    selection_marker: bool,
    dir_marker: bool,
    borders: BorderStyle,
    titles: bool,
    separators: bool,
    parent: bool,
    preview: bool,
    layout: LayoutConfig,
    preview_underline: bool,
    preview_underline_color: bool,
    entry_padding: u8,
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

    pub fn parent(&self) -> bool {
        self.parent
    }

    pub fn preview(&self) -> bool {
        self.preview
    }

    pub fn parent_ratio(&self) -> u16 {
        self.layout.parent_ratio()
    }

    pub fn main_ratio(&self) -> u16 {
        self.layout.main_ratio()
    }

    pub fn preview_ratio(&self) -> u16 {
        self.layout.preview_ratio()
    }

    pub fn preview_underline(&self) -> bool {
        self.preview_underline
    }

    pub fn preview_underline_color(&self) -> bool {
        self.preview_underline_color
    }

    pub fn entry_padding(&self) -> u8 {
        self.entry_padding
    }

    pub fn scroll_padding(&self) -> usize {
        self.scroll_padding
    }

    pub fn padding_str(&self) -> &'static str {
        // ASCII whitespaces
        match self.entry_padding {
            0 => "",
            1 => " ",
            2 => "  ",
            3 => "   ",
            _ => "    ",
        }
    }
}

impl Default for Display {
    fn default() -> Self {
        Display {
            selection_marker: true,
            dir_marker: true,
            borders: BorderStyle::Split,
            titles: false,
            separators: true,
            parent: true,
            preview: true,
            layout: LayoutConfig {
                parent: 20,
                main: 40,
                preview: 40,
            },
            preview_underline: true,
            preview_underline_color: false,
            entry_padding: 1,
            scroll_padding: 5,
        }
    }
}
