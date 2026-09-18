//! UI-independent grid interactions. Rectangles use half-open view coordinates.
//! Selection is scoped to a view generation, never used directly as business IDs.
#![forbid(unsafe_code)]

pub mod actions;
pub mod edit_planner;
pub mod formula;
pub mod header_groups;
pub use header_groups::*;
pub mod options;
pub mod report;
pub mod tree;
pub use options::*;
pub use tree::{LoadTicket, TreeError, TreeModel, TreeNode, TreeRow};
mod editing;
mod find;
pub mod matched_selection;
pub use find::*;
mod filter;
mod preferences;
mod query;
pub use editing::*;
pub use preferences::*;
pub use query::*;
mod pagination;
pub use pagination::Pagination;
pub mod paged_query;
pub mod remote;
mod rows;
pub use filter::{DateTimeParts, Filter, FilterDraft, FilterKind, money_minor};
mod selection;
pub use rows::{RowId, RowSelection};
pub use selection::{Cell, CellRect, Selection, SelectionError, SelectionOperation};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct ColumnId(pub u64);

#[derive(Clone, Debug)]
pub struct Column {
    pub id: ColumnId,
    pub title: String,
    pub filter: FilterKind,
    pub visible: bool,
    pub pinned: bool,
    /// Right pinning; legacy `pinned` remains the left pin and takes precedence.
    pub pinned_right: bool,
    pub value_kind: ValueKind,
    pub edit_kind: EditKind,
    width: f32,
    min_width: f32,
    max_width: f32,
}

/// Width limits shared by the grid's resize, auto-fit, configuration and
/// preference restore paths. Per-column limits remain inside this safe range.
pub const DEFAULT_MIN_COLUMN_WIDTH: f32 = 64.0;
pub const DEFAULT_MAX_COLUMN_WIDTH: f32 = 640.0;

impl Column {
    pub fn new(id: ColumnId, title: impl Into<String>, width: f32) -> Self {
        Self {
            id,
            title: title.into(),
            filter: FilterKind::Text,
            visible: true,
            pinned: false,
            pinned_right: false,
            value_kind: ValueKind::Text,
            edit_kind: EditKind::ReadOnly,
            width: valid_width(width, DEFAULT_MIN_COLUMN_WIDTH, DEFAULT_MAX_COLUMN_WIDTH),
            min_width: DEFAULT_MIN_COLUMN_WIDTH,
            max_width: DEFAULT_MAX_COLUMN_WIDTH,
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn pin_rank(&self) -> u8 {
        if self.pinned {
            0
        } else if self.pinned_right {
            2
        } else {
            1
        }
    }

    pub fn is_pinned(&self) -> bool {
        self.pinned || self.pinned_right
    }

    /// Returns the inclusive minimum and maximum width accepted by this column.
    pub fn width_bounds(&self) -> (f32, f32) {
        (self.min_width, self.max_width)
    }

    /// Configure per-column limits. Non-finite or reversed bounds fall back to
    /// 64..=640; finite bounds are clamped to that safe global range.
    pub fn set_width_bounds(&mut self, min_width: f32, max_width: f32) {
        let (min_width, max_width) = normalized_bounds(min_width, max_width);
        self.min_width = min_width;
        self.max_width = max_width;
        self.width = valid_width(self.width, min_width, max_width);
    }

    pub fn with_width_bounds(mut self, min_width: f32, max_width: f32) -> Self {
        self.set_width_bounds(min_width, max_width);
        self
    }
    pub fn with_filter(mut self, filter: FilterKind) -> Self {
        self.filter = match filter {
            FilterKind::Enum(values) => FilterKind::categorical(values, true),
            other => other,
        };
        self
    }

    pub fn resize(&mut self, width: f32) {
        self.width = valid_width(width, self.min_width, self.max_width);
    }
}

fn normalized_bounds(min_width: f32, max_width: f32) -> (f32, f32) {
    if !min_width.is_finite() || !max_width.is_finite() {
        return (DEFAULT_MIN_COLUMN_WIDTH, DEFAULT_MAX_COLUMN_WIDTH);
    }
    let min_width = min_width.clamp(DEFAULT_MIN_COLUMN_WIDTH, DEFAULT_MAX_COLUMN_WIDTH);
    let max_width = max_width.clamp(DEFAULT_MIN_COLUMN_WIDTH, DEFAULT_MAX_COLUMN_WIDTH);
    if min_width > max_width {
        (DEFAULT_MIN_COLUMN_WIDTH, DEFAULT_MAX_COLUMN_WIDTH)
    } else {
        (min_width, max_width)
    }
}

fn valid_width(width: f32, min_width: f32, max_width: f32) -> f32 {
    if width.is_finite() {
        width.clamp(min_width, max_width)
    } else {
        120.0_f32.clamp(min_width, max_width)
    }
}

/// Move a column before a target ID; identity does not change with visible order.
pub fn move_column(columns: &mut Vec<Column>, source: ColumnId, before: Option<ColumnId>) -> bool {
    if Some(source) == before {
        return false;
    }
    let Some(from) = columns.iter().position(|c| c.id == source) else {
        return false;
    };
    if before.is_some_and(|id| !columns.iter().any(|c| c.id == id)) {
        return false;
    }
    let column = columns.remove(from);
    let target = columns
        .iter()
        .position(|c| Some(c.id) == before)
        .unwrap_or(columns.len());
    columns.insert(target, column);
    true
}

/// Half-open visible row interval; bounded independently of total row count.
pub fn visible_rows(
    offset: f32,
    height: f32,
    row_height: f32,
    count: usize,
) -> std::ops::Range<usize> {
    if !offset.is_finite() || !height.is_finite() || !row_height.is_finite() || row_height <= 0.0 {
        return 0..0;
    }
    let start = ((offset.max(0.0) / row_height).floor() as usize)
        .saturating_sub(1)
        .min(count);
    let end = (((offset.max(0.0) + height.max(0.0)) / row_height).ceil() as usize)
        .saturating_add(1)
        .min(count);
    start..end.max(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_identity_and_invalid_widths() {
        let mut cols = vec![
            Column::new(ColumnId(1), "A", 100.0),
            Column::new(ColumnId(2), "B", 100.0),
            Column::new(ColumnId(3), "C", 100.0),
        ];
        assert!(move_column(&mut cols, ColumnId(3), Some(ColumnId(1))));
        assert_eq!(cols.iter().map(|c| c.id.0).collect::<Vec<_>>(), [3, 1, 2]);
        assert!(!move_column(&mut cols, ColumnId(9), Some(ColumnId(1))));
        assert!(move_column(&mut cols, ColumnId(3), None));
        assert_eq!(cols.iter().map(|c| c.id.0).collect::<Vec<_>>(), [1, 2, 3]);
        cols[0].resize(f32::NAN);
        assert_eq!(cols[0].width(), 120.0);
        cols[0].resize(-100.0);
        assert_eq!(cols[0].width(), 64.0);
    }

    #[test]
    fn column_width_bounds_apply_to_resize_and_invalid_bounds_use_defaults() {
        let mut column = Column::new(ColumnId(1), "A", 100.0).with_width_bounds(96.0, 240.0);
        assert_eq!(column.width_bounds(), (96.0, 240.0));
        column.resize(1.0);
        assert_eq!(column.width(), 96.0);
        column.resize(999.0);
        assert_eq!(column.width(), 240.0);
        column.set_width_bounds(300.0, 200.0);
        assert_eq!(column.width_bounds(), (64.0, 640.0));
        column.set_width_bounds(f32::NAN, 200.0);
        assert_eq!(column.width_bounds(), (64.0, 640.0));
    }

    #[test]
    fn million_rows_only_exposes_viewport() {
        let rows = visible_rows(300_000.0, 600.0, 30.0, 1_000_000);
        assert_eq!(rows, 9_999..10_021);
        assert_eq!(visible_rows(50.0, 600.0, 30.0, 0), 0..0);
        assert_eq!(visible_rows(0.0, 1.0, f32::NAN, 10), 0..0);
    }
}
