//! Preset themes for the runa
//!
//! This module defines several preset themes that can be used in runa.
//! Each theme is created using a specific color palette and a unique name.

use crate::config::Theme;
use crate::config::theme::{Palette, make_theme};

const TOKYO_STORM: Palette = Palette {
    base: (36, 40, 59),
    surface: (41, 46, 66),
    overlay: (86, 95, 137),
    primary: (187, 154, 247),
    secondary: (125, 207, 255),
    directory: (122, 162, 247),
};

const TOKYO_NIGHT: Palette = Palette {
    base: (26, 27, 38),
    surface: (44, 51, 78),
    overlay: (86, 95, 137),
    primary: (187, 154, 247),
    secondary: (125, 207, 255),
    directory: (122, 162, 247),
};

const TOKYO_DAY: Palette = Palette {
    base: (225, 226, 231),
    surface: (196, 199, 209),
    overlay: (168, 175, 199),
    primary: (152, 94, 171),
    secondary: (52, 90, 183),
    directory: (52, 90, 183),
};

pub(crate) fn tokyonight_storm() -> Theme {
    make_theme("tokyonight-storm", TOKYO_STORM, "┃")
}
pub(crate) fn tokyonight_night() -> Theme {
    make_theme("tokyonight-night", TOKYO_NIGHT, "┃")
}
pub(crate) fn tokyonight_day() -> Theme {
    make_theme("tokyonight-day", TOKYO_DAY, "┃")
}

const GRUV_DARK_HARD: Palette = Palette {
    base: (29, 32, 33),
    surface: (60, 56, 54),
    overlay: (146, 131, 116),
    primary: (211, 134, 155),
    secondary: (142, 192, 124),
    directory: (131, 165, 152),
};

const GRUV_DARK: Palette = Palette {
    base: (40, 40, 40),
    surface: (60, 56, 54),
    overlay: (146, 131, 116),
    primary: (211, 134, 155),
    secondary: (142, 192, 124),
    directory: (131, 165, 152),
};

const GRUV_LIGHT: Palette = Palette {
    base: (251, 241, 199),
    surface: (213, 196, 161),
    overlay: (124, 111, 100),
    primary: (143, 63, 113),
    secondary: (66, 123, 88),
    directory: (7, 102, 120),
};

pub(crate) fn gruvbox_dark_hard() -> Theme {
    make_theme("gruvbox-dark-hard", GRUV_DARK_HARD, "*")
}
pub(crate) fn gruvbox_dark() -> Theme {
    make_theme("gruvbox-dark", GRUV_DARK, "*")
}
pub(crate) fn gruvbox_light() -> Theme {
    make_theme("gruvbox-light", GRUV_LIGHT, "*")
}

const MOCHA: Palette = Palette {
    base: (30, 30, 46),
    surface: (49, 50, 68),
    overlay: (108, 112, 134),
    primary: (203, 166, 247),
    secondary: (148, 226, 213),
    directory: (137, 180, 250),
};

const FRAPPE: Palette = Palette {
    base: (48, 52, 70),
    surface: (65, 69, 89),
    overlay: (115, 121, 148),
    primary: (202, 158, 230),
    secondary: (129, 200, 190),
    directory: (140, 170, 238),
};

const LATTE: Palette = Palette {
    base: (239, 241, 245),
    surface: (204, 208, 218),
    overlay: (156, 160, 176),
    primary: (136, 57, 239),
    secondary: (23, 146, 153),
    directory: (30, 102, 245),
};

pub(crate) fn catppuccin_mocha() -> Theme {
    make_theme("catppuccin-mocha", MOCHA, "┃")
}
pub(crate) fn catppuccin_frappe() -> Theme {
    make_theme("catppuccin-frappe", FRAPPE, "┃")
}
pub(crate) fn catppuccin_latte() -> Theme {
    make_theme("catppuccin-latte", LATTE, "┃")
}

const CARBON: Palette = Palette {
    base: (22, 22, 22),
    surface: (42, 42, 42),
    overlay: (82, 82, 82),
    primary: (190, 149, 233),
    secondary: (61, 187, 199),
    directory: (120, 169, 235),
};

const NIGHTFOX: Palette = Palette {
    base: (25, 30, 36),
    surface: (43, 51, 63),
    overlay: (87, 91, 112),
    primary: (195, 157, 239),
    secondary: (99, 199, 209),
    directory: (113, 161, 236),
};

pub(crate) fn carbonfox() -> Theme {
    make_theme("carbonfox", CARBON, "┃")
}
pub(crate) fn nightfox() -> Theme {
    make_theme("nightfox", NIGHTFOX, "┃")
}

const FOREST: Palette = Palette {
    base: (43, 51, 57),
    surface: (74, 82, 88),
    overlay: (133, 146, 137),
    primary: (167, 192, 128),
    secondary: (230, 126, 128),
    directory: (127, 187, 179),
};

const ROSE_PINE: Palette = Palette {
    base: (25, 23, 36),
    surface: (31, 29, 46),
    overlay: (110, 106, 134),
    primary: (196, 167, 231),
    secondary: (235, 188, 186),
    directory: (49, 116, 143),
};

pub(crate) fn everforest() -> Theme {
    make_theme("everforest", FOREST, "*")
}
pub(crate) fn rose_pine() -> Theme {
    make_theme("rose_pine", ROSE_PINE, "*")
}

const NORD: Palette = Palette {
    base: (46, 52, 64),
    surface: (67, 76, 94),
    overlay: (94, 129, 172),
    primary: (163, 190, 140),
    secondary: (191, 97, 106),
    directory: (129, 161, 193),
};

pub(crate) fn nord() -> Theme {
    make_theme("nord", NORD, "*")
}

const TWO_DARK: Palette = Palette {
    base: (40, 44, 52),
    surface: (33, 37, 43),
    overlay: (92, 99, 112),
    primary: (97, 175, 239),
    secondary: (198, 120, 221),
    directory: (229, 192, 123),
};

pub(crate) fn two_dark() -> Theme {
    make_theme("two-dark", TWO_DARK, "*")
}

const ONE_DARK: Palette = Palette {
    base: (40, 44, 52),
    surface: (56, 60, 69),
    overlay: (97, 102, 117),
    primary: (97, 175, 239),
    secondary: (198, 120, 221),
    directory: (229, 192, 123),
};

pub(crate) fn one_dark() -> Theme {
    make_theme("one-dark", ONE_DARK, "*")
}

const SOLARIZED_DARK: Palette = Palette {
    base: (0, 43, 54),
    surface: (7, 54, 66),
    overlay: (101, 123, 131),
    primary: (38, 139, 210),
    secondary: (211, 54, 130),
    directory: (42, 161, 152),
};

const SOLARIZED_LIGHT: Palette = Palette {
    base: (253, 246, 227),
    surface: (238, 232, 213),
    overlay: (101, 123, 131),
    primary: (38, 139, 210),
    secondary: (211, 54, 130),
    directory: (42, 161, 152),
};

pub(crate) fn solarized_dark() -> Theme {
    make_theme("solarized-dark", SOLARIZED_DARK, "*")
}

pub(crate) fn solarized_light() -> Theme {
    make_theme("solarized-light", SOLARIZED_LIGHT, "*")
}

const DRACULA: Palette = Palette {
    base: (40, 42, 54),
    surface: (68, 71, 90),
    overlay: (139, 233, 253),
    primary: (255, 121, 198),
    secondary: (80, 250, 123),
    directory: (189, 147, 249),
};

pub(crate) fn dracula() -> Theme {
    make_theme("dracula", DRACULA, "┃")
}

const MONOKAI: Palette = Palette {
    base: (39, 40, 34),
    surface: (49, 51, 43),
    overlay: (117, 113, 94),
    primary: (249, 38, 114),
    secondary: (166, 226, 46),
    directory: (102, 217, 239),
};

pub(crate) fn monokai() -> Theme {
    make_theme("monokai", MONOKAI, "┃")
}
