use egui::{Color32, TextureId};
use erp_grid_core::{ColumnId, RowId};

/// Raw value remains authoritative for edit, copy and export.
pub struct CellContext<'a> {
    pub row: usize,
    pub row_id: Option<RowId>,
    pub column: ColumnId,
    pub raw: &'a str,
}

#[derive(Default)]
pub enum CellPresentation {
    #[default]
    Default,
    Text(String),
    Badge {
        label: String,
        color: Color32,
    },
    /// Loading and bounded texture caching belong to the host.
    Image {
        texture: TextureId,
        tooltip: String,
    },
    /// Suitable for document links too; emits an intent, never opens a URL.
    Action {
        id: String,
        label: String,
        enabled: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellAction {
    pub row: RowId,
    pub column: ColumnId,
    pub action: String,
}

#[derive(Default)]
pub struct GridOptions<'a> {
    pub details_enabled: bool,
    pub expanded: &'a [(usize, RowId)],
    /// Pure, cheap callback, invoked for visible cells and pointer/keyboard hits only.
    pub present: Option<&'a dyn Fn(CellContext<'_>) -> CellPresentation>,
}
