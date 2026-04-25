//! Sort config module

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SortMode {
    Name,
    Modified,
    Created,
    Accessed,
    Size,
    Extension,
    Natural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub(crate) fn toggle(self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SortConfig {
    pub(crate) mode: SortMode,
    pub(crate) order: SortOrder,
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            mode: SortMode::Natural,
            order: SortOrder::Ascending,
        }
    }
}
