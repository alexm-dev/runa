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
    mode: SortMode,
    order: SortOrder,
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            mode: SortMode::Natural,
            order: SortOrder::Ascending,
        }
    }
}

impl From<(SortMode, SortOrder)> for SortConfig {
    fn from((mode, order): (SortMode, SortOrder)) -> Self {
        Self { mode, order }
    }
}

impl SortConfig {
    crate::getters!(
        mode: SortMode,
        order: SortOrder,
    );

    pub(crate) fn set_mode(&mut self, mode: SortMode) {
        self.mode = mode;
    }

    pub(crate) fn set_order(&mut self, order: SortOrder) {
        self.order = order
    }
}
