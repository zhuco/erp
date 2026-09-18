//! Shared native/Web grid renderer. No business or network dependencies.
#![forbid(unsafe_code)]
mod accessibility;
mod batch_panel;
mod cell_presentation;
mod cell_text;
pub use cell_presentation::{CellAction, CellContext, CellPresentation, GridOptions};
mod change_preview;
mod clipboard;
mod header_groups;
pub mod report_chart;
pub use batch_panel::{BatchActionIntent, BatchActionPanel};
mod column_config;
mod editor;
mod entity_picker;
pub use entity_picker::{EntityOption, EntitySearch};
mod pagination;
pub mod query_builder;
mod quick_search;
mod row_layout;
mod theme;
mod tree_render;
pub use change_preview::{
    ChangeItemId, ChangePreview, ChangePreviewEvent, ChangeResult, ChangeRowResult,
};
pub use clipboard::selection_tsv;
pub use pagination::show_pagination;
pub use theme::GridTheme;

use egui::{Align2, FontId, Id, Key, Pos2, Rect, Sense, Stroke, Vec2};
use erp_grid_core::{
    Cell, CellRect, Column, ColumnId, RowId, RowSelection, Selection, SelectionOperation,
    move_column,
};
use std::collections::BTreeMap;

const MAX_SUMMARY_ROWS: usize = 3;
const MAX_SUMMARY_VALUE_BYTES: usize = 4096;
const MAX_SUMMARY_COLUMNS: usize = 256;
const MAX_SUMMARY_TEXT_BYTES: usize = 1024 * 1024;

/// One fixed summary row rendered below the scrolling data body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SummaryRow {
    pub label: String,
    pub cells: BTreeMap<ColumnId, String>,
}

/// One pointer gesture at a time. Selection previews are derived from a fixed baseline.
enum Gesture {
    CheckRows {
        anchor: usize,
        baseline: RowSelection,
        checked: bool,
    },
    Select {
        anchor: Cell,
        baseline: Selection,
        operation: SelectionOperation,
    },
    Resize {
        column: ColumnId,
        start_x: f32,
        width: f32,
    },
    Move {
        column: ColumnId,
        start_x: f32,
    },
    Fill {
        source: CellRect,
    },
}

pub struct GridState {
    /// Bounded presentation density. The default preserves the grid lab's
    /// existing 28px rows and 32px header; ERP hosts may use `legacy()`.
    pub metrics: GridMetrics,
    pub header_groups: Vec<erp_grid_core::HeaderGroup>,
    pub row_selection: RowSelection,
    pub row_number_start: u64,
    pub theme: Option<GridTheme>,
    pub quick_search_open: bool,
    pub quick_search: std::collections::BTreeMap<ColumnId, erp_grid_core::FilterDraft>,
    pub selection: Selection,
    pub focus: Option<Cell>,
    pub message: Option<String>,
    pub sorts: Vec<erp_grid_core::SortRule>,
    pub read_only: bool,
    pub sorting_enabled: bool,
    pub filtering_enabled: bool,
    pub fill_enabled: bool,
    pub context_actions: Vec<erp_grid_core::actions::ActionSpec>,
    /// Optional presentation metadata for loaded tree rows. Missing entries use the
    /// ordinary flat-row layout, so existing hosts do not need to opt in.
    pub tree_rows: std::collections::BTreeMap<RowId, TreeRowVisual>,
    /// Fixed totals shown at the bottom of the body. The renderer accepts at
    /// most three rows and bounds every displayed value to 4 KiB.
    pub summary_rows: Vec<SummaryRow>,
    editor: Option<editor::Editor>,
    requested_edit: Option<(Cell, RowId, ColumnId)>,
    next_edit_ticket: u64,
    context_cell: Option<Cell>,
    anchor: Option<Cell>,
    check_anchor: Option<usize>,
    category: Option<u64>,
    gesture: Option<Gesture>,
    view_generation: Option<u64>,
    scroll: Vec2,
    focus_request: Option<Cell>,
    column_config_open: bool,
    column_query: String,
    context_column: Option<ColumnId>,
    column_action: Option<column_config::ColumnAction>,
    visual_ids: Vec<ColumnId>,
}

impl Default for GridState {
    fn default() -> Self {
        Self {
            metrics: GridMetrics::default(),
            header_groups: Vec::new(),
            quick_search_open: true,
            row_selection: Default::default(),
            row_number_start: 0,
            theme: None,
            quick_search: Default::default(),
            selection: Default::default(),
            focus: None,
            message: None,
            sorts: Vec::new(),
            read_only: false,
            sorting_enabled: true,
            filtering_enabled: true,
            fill_enabled: false,
            context_actions: Vec::new(),
            tree_rows: Default::default(),
            summary_rows: Vec::new(),
            editor: None,
            requested_edit: None,
            next_edit_ticket: 0,
            context_cell: None,
            anchor: None,
            check_anchor: None,
            category: None,
            gesture: None,
            view_generation: None,
            scroll: Vec2::ZERO,
            focus_request: None,
            column_config_open: false,
            column_query: String::new(),
            context_column: None,
            column_action: None,
            visual_ids: Vec::new(),
        }
    }
}

/// Presentation dimensions shared by layout, hit testing, and virtualization.
/// Values are logical pixels and are clamped by [`GridMetrics::new`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridMetrics {
    pub row_height: f32,
    pub header_height: f32,
    pub font_size: f32,
}

impl GridMetrics {
    pub const fn new(row_height: f32, header_height: f32, font_size: f32) -> Self {
        // Callers should use `bounded`; const construction is retained for
        // simple fixed presets while render paths enforce the same bounds.
        Self {
            row_height,
            header_height,
            font_size,
        }
    }

    pub const fn legacy() -> Self {
        Self::new(21.0, 24.0, 12.8)
    }

    fn bounded(self) -> Self {
        Self {
            row_height: self.row_height.clamp(16.0, 64.0),
            header_height: self.header_height.clamp(16.0, 64.0),
            font_size: self.font_size.clamp(8.0, 24.0),
        }
    }
}

impl Default for GridMetrics {
    fn default() -> Self {
        Self::new(ROW, HEADER, 13.0)
    }
}

impl GridState {
    /// Take a new debounced object search. Cancel a host task when entity_search changes.
    pub fn take_entity_search(&mut self) -> Option<EntitySearch> {
        self.editor.as_mut()?.picker.outgoing.take()
    }

    pub fn entity_search(&self) -> Option<&EntitySearch> {
        self.editor.as_ref()?.picker.active.as_ref()
    }

    pub fn resolve_entity_options(
        &mut self,
        request: &EntitySearch,
        result: Result<Vec<EntityOption>, String>,
    ) -> bool {
        self.editor
            .as_mut()
            .is_some_and(|editor| !editor.awaiting && editor.picker.resolve(request, result))
    }
    /// Snapshot the single editor without changing its focus or submission ticket.
    pub fn edit_draft(&self) -> Option<erp_grid_core::CellEdit> {
        self.editor.as_ref().map(|editor| erp_grid_core::CellEdit {
            row: editor.row,
            column: editor.column,
            before: editor.before.clone(),
            after: editor.draft.clone(),
        })
    }
    /// Validate and mark the current editor as awaiting its host save. This is
    /// the same ticket transition used by Enter, exposed for application-level
    /// "save and leave" coordination.
    pub fn prepare_edit_commit(
        &mut self,
        column: &Column,
    ) -> Result<Option<(erp_grid_core::CellEdit, u64)>, String> {
        let Some(editor) = &mut self.editor else {
            return Ok(None);
        };
        if editor.awaiting {
            return Ok(None);
        }
        if editor.composing {
            return Err("Finish composing text before saving".into());
        }
        if editor.external_conflict {
            return Err("Row changed; review the latest version before saving".into());
        }
        if column.id != editor.column {
            return Err("Edited column does not match the active editor".into());
        }
        if matches!(column.edit_kind, erp_grid_core::EditKind::Entity) && !editor.picker.selected {
            return Err("Choose an object before saving".into());
        }
        column
            .edit_kind
            .validate(&editor.draft)
            .map_err(String::from)?;
        if editor.before == editor.draft {
            self.editor = None;
            return Ok(None);
        }
        editor.awaiting = true;
        Ok(Some((
            erp_grid_core::CellEdit {
                row: editor.row,
                column: editor.column,
                before: editor.before.clone(),
                after: editor.draft.clone(),
            },
            editor.ticket,
        )))
    }
    /// Authentication or host interruption must not turn returning focus into a save.
    pub fn hold_edit(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.retry_required = true;
            editor.first = true;
        }
    }
    /// Notify an unsubmitted editor that its row version changed, even if a
    /// different field was patched. Keep draft/base unchanged and block every
    /// commit path until explicit rebase or cancel. Pending saves retain their
    /// original receipt lifecycle and must be resolved by their host.
    pub fn notify_row_changed(&mut self, row: RowId) -> bool {
        if let Some(editor) = &mut self.editor
            && editor.row == row
            && !editor.awaiting
        {
            editor.external_conflict = true;
            editor.retry_required = true;
            self.message = Some("Row changed; review the latest version before saving".into());
            return true;
        }
        false
    }
    /// Only an explicit host conflict-resolution action may adopt a newer base.
    pub fn rebase_edit(&mut self, row: RowId, column: ColumnId, before: String) -> bool {
        if let Some(editor) = &mut self.editor
            && !editor.awaiting
            && editor.row == row
            && editor.column == column
        {
            editor.before = before;
            editor.external_conflict = false;
            editor.retry_required = true;
            editor.first = true;
            self.message = None;
            return true;
        }
        false
    }
    pub fn is_editing(&self) -> bool {
        self.editor.is_some() || self.requested_edit.is_some()
    }
    pub fn cancel_edit(&mut self) {
        if self.editor.as_ref().is_some_and(|e| e.awaiting) {
            return;
        }
        self.editor = None;
        self.requested_edit = None;
    }
    /// A host must acknowledge every emitted edit. Failure preserves the original
    /// draft and before-value; callers must not silently overwrite a conflict.
    pub fn resolve_edit(&mut self, ticket: u64, result: Result<(), String>) -> bool {
        if !self
            .editor
            .as_ref()
            .is_some_and(|e| e.awaiting && e.ticket == ticket)
        {
            return false;
        }
        match result {
            Ok(()) => self.editor = None,
            Err(error) => {
                self.next_edit_ticket = self.next_edit_ticket.wrapping_add(1);
                if let Some(editor) = &mut self.editor {
                    editor.ticket = self.next_edit_ticket;
                    editor.awaiting = false;
                    editor.retry_required = true;
                    editor.first = true;
                }
                self.message = Some(error);
            }
        }
        true
    }
    pub fn request_edit(&mut self, cell: Cell, row: RowId, column: ColumnId) {
        if self.editor.is_none() {
            self.requested_edit = Some((cell, row, column));
        }
    }

    /// Restore a host-validated, unsubmitted edit without submitting it.
    pub fn restore_edit(
        &mut self,
        cell: Cell,
        row: RowId,
        column: ColumnId,
        before: String,
        after: String,
    ) -> Result<(), &'static str> {
        if self.editor.is_some() || self.requested_edit.is_some() {
            return Err("An edit is already active");
        }
        if row.0 == 0
            || before == after
            || before.len() > 4096
            || after.len() > 4096
            || before.contains('\0')
            || after.contains('\0')
        {
            return Err("Invalid restored edit");
        }
        self.next_edit_ticket = self.next_edit_ticket.wrapping_add(1);
        self.editor = Some(editor::Editor {
            picker: Default::default(),
            ticket: self.next_edit_ticket,
            row,
            column,
            before,
            draft: after,
            first: true,
            composing: false,
            focused: false,
            awaiting: false,
            retry_required: true,
            external_conflict: false,
        });
        self.focus = Some(cell);
        self.focus_request = Some(cell);
        self.row_selection.active = Some(row);
        Ok(())
    }

    /// Set keyboard focus for a host action such as local find. The grid
    /// consumes the request on its next paint and scrolls the cell into view
    /// once; selection and checked-row identities are left untouched.
    pub fn request_focus(&mut self, cell: Cell) {
        self.focus = Some(cell);
        self.focus_request = Some(cell);
    }
    fn sort_column(&mut self, column: ColumnId, descending: Option<bool>, additive: bool) {
        let previous = self
            .sorts
            .iter()
            .find(|rule| rule.column == column)
            .copied();
        let next = descending
            .or_else(|| previous.map_or(Some(false), |rule| (!rule.descending).then_some(true)));
        if !additive {
            self.sorts.clear();
        } else {
            self.sorts.retain(|rule| rule.column != column);
        }
        if let Some(descending) = next {
            self.sorts
                .push(erp_grid_core::SortRule { column, descending });
        }
    }

    /// Call before replacing category-dependent column option schemas.
    pub fn bind_category(&mut self, category: u64) {
        if self.category != Some(category) {
            self.quick_search.clear();
            self.row_selection.clear();
            self.clear_selection();
            self.focus = None;
            self.check_anchor = None;
            self.category = Some(category);
            self.view_generation = None;
        }
    }
    /// Caller changes generation whenever row order, scope or column identity/order changes.
    pub fn bind_view(&mut self, generation: u64) {
        if self.view_generation != Some(generation) {
            self.context_cell = None;
            self.requested_edit = None;
            self.context_column = None;
            if self.editor.take().is_some() {
                self.message = Some("Edit cancelled because the data scope changed".into());
            }
            self.row_selection.clear();
            self.selection.clear();
            self.focus = None;
            self.anchor = None;
            self.check_anchor = None;
            self.gesture = None;
            self.scroll = Vec2::ZERO;
            self.focus_request = None;
            self.view_generation = Some(generation);
        }
    }

    /// Rebind positional state while preserving checked business identities.
    /// The host must clear checks separately when the filter or authorization scope changes.
    pub fn rebind_page(&mut self, generation: u64, reset_vertical: bool) {
        let mut rows = self.row_selection.clone();
        rows.active = None;
        let scroll = self.scroll;
        self.bind_view(generation);
        self.row_selection = rows;
        self.scroll = if reset_vertical {
            Vec2::new(scroll.x, 0.0)
        } else {
            scroll
        };
    }
    /// An unchanged page identity sequence can preserve focus, editor and clipboard coordinates.
    pub fn adopt_generation(&mut self, generation: u64) {
        self.view_generation = Some(generation);
    }
    pub fn scroll_offset(&self) -> Vec2 {
        self.scroll
    }

    /// Request a host scroll position; layout clamps it to the content bounds.
    pub fn set_scroll_offset(&mut self, offset: Vec2) {
        if offset.x.is_finite() && offset.y.is_finite() {
            self.scroll = offset.max(Vec2::ZERO);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.gesture = None;
        self.anchor = None;
    }
}

#[derive(Default, Debug)]
pub struct GridOutput {
    pub cell_action: Option<CellAction>,
    pub painted_cells: usize,
    pub columns_changed: bool,
    pub quick_search_changed: bool,
    pub sorts_changed: bool,
    pub edits: Vec<erp_grid_core::CellEdit>,
    pub edit_ticket: Option<u64>,
    pub edit_navigation: Option<EditNavigation>,
    pub paste: Option<(Cell, String)>,
    pub row_action: Option<(String, Vec<RowId>)>,
    pub undo: bool,
    pub redo: bool,
    pub toggle_detail: Option<RowId>,
    /// The host owns expansion and loading; the renderer only reports the intent.
    pub toggle_tree: Option<RowId>,
    pub fill: Option<FillIntent>,
    pub content_height: f32,
    pub viewport_height: f32,
    editor_drawn: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeRowVisual {
    pub depth: u8,
    pub expanded: bool,
    pub has_children: bool,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FillIntent {
    pub source: CellRect,
    /// The new rows to fill. This rectangle never overlaps `source`.
    pub target: CellRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditNavigation {
    pub row: RowId,
    pub column: ColumnId,
    pub backwards: bool,
}

/// Data remains in the application; painting requests visible cells only.
/// Explicit column auto-size additionally samples at most 200 loaded rows.
pub struct DataGrid<'a> {
    pub id: Id,
    pub columns: &'a mut Vec<Column>,
    pub rows: usize,
    pub state: &'a mut GridState,
    /// Stable identity mapping enables a pinned checkbox column.
    pub row_id: Option<&'a dyn Fn(usize) -> RowId>,
}

const ROW: f32 = 28.0;
const CHECK_WIDTH: f32 = 36.0;
const NUMBER_WIDTH: f32 = 44.0;
const STRUCTURE_WIDTH: f32 = CHECK_WIDTH + NUMBER_WIDTH;

fn truncate_utf8(value: &str, max_bytes: usize) -> (&str, bool) {
    if value.len() <= max_bytes {
        return (value, false);
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (&value[..end], true)
}

fn bounded_summary_rows(rows: &[SummaryRow], columns: &[Column]) -> (Vec<SummaryRow>, bool) {
    let mut bounded = Vec::with_capacity(rows.len().min(MAX_SUMMARY_ROWS));
    let mut exceeded = rows.len() > MAX_SUMMARY_ROWS || columns.len() > MAX_SUMMARY_COLUMNS;
    let mut remaining = MAX_SUMMARY_TEXT_BYTES;
    for row in rows.iter().take(MAX_SUMMARY_ROWS) {
        let label_limit = MAX_SUMMARY_VALUE_BYTES.min(remaining);
        let (label, label_truncated) = truncate_utf8(&row.label, label_limit);
        exceeded |= label_truncated;
        remaining = remaining.saturating_sub(label.len());
        let mut cells = BTreeMap::new();
        exceeded |= row.cells.len() > MAX_SUMMARY_COLUMNS;
        for column in columns.iter().take(MAX_SUMMARY_COLUMNS) {
            let Some(value) = row.cells.get(&column.id) else {
                continue;
            };
            let value_limit = MAX_SUMMARY_VALUE_BYTES.min(remaining);
            let (value, truncated) = truncate_utf8(value, value_limit);
            exceeded |= truncated;
            remaining = remaining.saturating_sub(value.len());
            cells.insert(column.id, value.to_owned());
        }
        bounded.push(SummaryRow {
            label: label.to_owned(),
            cells,
        });
    }
    (bounded, exceeded)
}
const HEADER: f32 = 32.0;

fn fill_target(source: CellRect, pointer_row: usize, rows: usize) -> Option<CellRect> {
    if source.bottom > rows || pointer_row >= rows {
        return None;
    }
    if pointer_row < source.top {
        Some(CellRect {
            top: pointer_row,
            left: source.left,
            bottom: source.top,
            right: source.right,
        })
    } else if pointer_row >= source.bottom {
        Some(CellRect {
            top: source.bottom,
            left: source.left,
            bottom: pointer_row + 1,
            right: source.right,
        })
    } else {
        None
    }
}

impl DataGrid<'_> {
    pub fn show(
        self,
        ui: &mut egui::Ui,
        cell_text: impl Fn(usize, ColumnId) -> String,
    ) -> GridOutput {
        self.show_with_details(ui, cell_text, false, &[], |_, _| {})
    }

    /// Details are keyed by stable identity. The host owns their data and drafts.
    pub fn show_with_details(
        self,
        ui: &mut egui::Ui,
        cell_text: impl Fn(usize, ColumnId) -> String,
        enabled: bool,
        expanded: &[(usize, RowId)],
        detail: impl FnMut(&mut egui::Ui, RowId),
    ) -> GridOutput {
        self.show_with_options(
            ui,
            cell_text,
            GridOptions {
                details_enabled: enabled,
                expanded,
                present: None,
            },
            detail,
        )
    }

    pub fn show_with_options(
        self,
        ui: &mut egui::Ui,
        cell_text: impl Fn(usize, ColumnId) -> String,
        options: GridOptions<'_>,
        mut detail: impl FnMut(&mut egui::Ui, RowId),
    ) -> GridOutput {
        let enabled = options.details_enabled;
        let Self {
            id,
            columns,
            rows,
            state,
            row_id,
        } = self;
        let gutter = STRUCTURE_WIDTH + if enabled { 20.0 } else { 0.0 };
        let pin_budget = (ui.available_width() - gutter - 80.0).max(0.0);
        let fitted = column_config::fit_pinned(columns, pin_budget);
        if fitted {
            state.message = Some(
                "Some rightmost columns were unlocked to keep the table usable at this width"
                    .into(),
            );
        }
        columns.sort_by_key(Column::pin_rank);
        let mut visible: Vec<_> = columns
            .iter()
            .filter(|column| column.visible)
            .cloned()
            .collect();
        let ids: Vec<_> = visible.iter().map(|column| column.id).collect();
        if state.visual_ids != ids {
            state.context_cell = None;
            state.context_column = None;
            state.clear_selection();
            state.focus = None;
            state.visual_ids = ids;
        }
        let mut data_rect = ui.available_rect_before_wrap();
        data_rect.min.x = (data_rect.min.x + gutter).min(data_rect.max.x);
        let mut output = ui
            .scope_builder(egui::UiBuilder::new().max_rect(data_rect), |ui| {
                ui.style_mut().spacing.scroll.floating = false;
                DataGrid {
                    id,
                    columns: &mut visible,
                    rows,
                    state,
                    row_id,
                }
                .show_visible(ui, cell_text, &options, &mut detail)
            })
            .inner;
        // Virtualization may omit the edited row or column. Keep its draft reachable
        // in a scoped editor instead of making the entire host permanently dirty.
        if !output.editor_drawn
            && let Some(editor) = &mut state.editor
        {
            let mut closed = false;
            let mut open = true;
            egui::Window::new("Edit cell")
                .id(id.with("offscreen-editor"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    if let Some(column) = columns.iter().find(|c| c.id == editor.column) {
                        ui.label(format!("{} · #{}", column.title, editor.row.0));
                        let (_, rect) =
                            ui.allocate_space(Vec2::new(300.0, state.metrics.bounded().row_height));
                        let outcome =
                            editor::show(ui, rect, id.with("cell-editor"), column, editor);
                        closed = outcome.finished;
                        output.edit_navigation = outcome.navigation;
                        if let Some(edit) = outcome.edit {
                            output.edit_ticket = Some(editor.ticket);
                            output.edits.push(edit);
                        }
                        if let Some(error) = outcome.error {
                            state.message = Some(error);
                        }
                    } else {
                        ui.label("Column unavailable; discard this draft to continue");
                    }
                    if ui
                        .add_enabled(!editor.awaiting, egui::Button::new("Cancel"))
                        .clicked()
                    {
                        closed = true;
                    }
                });
            if closed || !open {
                state.cancel_edit();
            }
        }
        if output.columns_changed {
            let mut reordered = visible.into_iter();
            for column in columns.iter_mut().filter(|column| column.visible) {
                if let Some(next) = reordered.next() {
                    *column = next;
                }
            }
            columns.sort_by_key(Column::pin_rank);
        }
        output.columns_changed |= fitted;
        let mut configured = false;
        if let Some(action) = state.column_action.take() {
            configured |= column_config::apply(columns, action);
        }
        configured |= column_config::show(
            ui,
            id,
            &mut state.column_config_open,
            &mut state.column_query,
            columns,
        );
        if column_config::fit_pinned(columns, pin_budget) {
            state.message = Some("Lock area is full; rightmost locks were released".into());
            configured = true;
        }
        if state.row_selection.take_overflowed() {
            state.message = Some("最多勾选 10,000 条；更多记录请使用支持的全部匹配操作".into());
        }
        output.columns_changed |= configured;
        if configured {
            let configured_ids: Vec<_> =
                columns.iter().filter(|c| c.visible).map(|c| c.id).collect();
            if configured_ids != state.visual_ids {
                state.clear_selection();
                state.focus = None;
            }
            ui.ctx().request_repaint();
        }
        output
    }

    fn show_visible(
        self,
        ui: &mut egui::Ui,
        cell_text: impl Fn(usize, ColumnId) -> String,
        options: &GridOptions<'_>,
        detail: &mut dyn FnMut(&mut egui::Ui, RowId),
    ) -> GridOutput {
        let details_enabled = options.details_enabled;
        let expanded = options.expanded;
        let Self {
            id,
            columns,
            rows,
            state,
            row_id,
        } = self;
        let metrics = state.metrics.bounded();
        let row_height = metrics.row_height;
        let column_header_height = metrics.header_height;
        let search_height = row_height.max(24.0);
        let gutter = STRUCTURE_WIDTH + if details_enabled { 20.0 } else { 0.0 };
        let group_height = if state.header_groups.is_empty() {
            0.0
        } else {
            header_groups::COMPACT_HEADER_GROUP_HEIGHT
        };
        let header_height = group_height
            + column_header_height
            + if state.quick_search_open {
                search_height
            } else {
                0.0
            };
        let (summary_rows, summary_limited) = bounded_summary_rows(&state.summary_rows, columns);
        if summary_limited {
            state.message = Some(
                "合计最多 3 行、256 列、总文本 1 MiB，单值最多 4096 字节；超出内容未显示或已截断"
                    .into(),
            );
        }
        let summary_height = row_height * summary_rows.len() as f32;
        let mut output = GridOutput::default();
        let theme = state
            .theme
            .unwrap_or_else(|| GridTheme::from_visuals(ui.visuals()));
        let mut positions = Vec::with_capacity(columns.len());
        let mut width = 0.0;
        for column in columns.iter() {
            positions.push(width);
            width += column.width();
        }
        let mut move_request = None;
        let mut resize_request = None;
        let mut auto_scroll = Vec2::ZERO;
        let layout = row_layout::RowLayout::with_row_height(
            rows,
            expanded.iter().map(|(row, _)| *row),
            row_height,
        );
        let scroll = egui::ScrollArea::both()
            .id_salt(id)
            .auto_shrink([false, false])
            .scroll_offset(state.scroll)
            .scroll_source(egui::scroll_area::ScrollSource {
                drag: egui::scroll_area::DragScroll::Never,
                ..Default::default()
            })
            .show_viewport(ui, |ui, viewport| {
                let origin = ui.cursor().min;
                let content_size =
                    Vec2::new(width, header_height + layout.height() + summary_height);
                ui.set_min_size(content_size);
                // Absolute-positioned controls must not add their layout cursor to scroll extent.
                let mut content_ui = ui.new_child(
                    egui::UiBuilder::new().max_rect(Rect::from_min_size(origin, content_size)),
                );
                let ui = &mut content_ui;
                let view = Rect::from_min_size(origin + viewport.min.to_vec2(), viewport.size())
                    .intersect(ui.clip_rect());
                let body = Rect::from_min_max(
                    Pos2::new(view.left(), view.top() + header_height),
                    Pos2::new(
                        view.right(),
                        (view.bottom() - summary_height).max(view.top() + header_height),
                    ),
                );
                let header_y = view.top() + group_height;
                let visible = layout.visible(view.top() - origin.y, body.height());
                let interaction = Rect::from_min_max(
                    Pos2::new(body.left(), view.top()),
                    Pos2::new(view.right(), body.bottom()),
                );
                let response =
                    ui.interact(interaction, id.with("interaction"), Sense::click_and_drag());
                let pointer = ui.input(|i| i.pointer.interact_pos());
                let mut insertion_x = None;
                let modifiers = ui.input(|i| i.modifiers);
                let pinned_width: f32 =
                    columns.iter().filter(|c| c.pinned).map(Column::width).sum();
                // Keep a scrollable strip available even if the window shrinks after pinning.
                let frozen_edge =
                    (body.left() + pinned_width).min((view.right() - 64.0).max(body.left()));
                let right_width: f32 = columns
                    .iter()
                    .filter(|c| c.pin_rank() == 2)
                    .map(Column::width)
                    .sum();
                let right_edge = (view.right() - right_width).max(frozen_edge);
                let column_left = |index: usize| {
                    if columns[index].pin_rank() == 2 {
                        view.right() + positions[index] - width
                    } else {
                        origin.x
                            + positions[index]
                            + if columns[index].pinned {
                                viewport.left()
                            } else {
                                0.0
                            }
                    }
                };
                let column_clip = |index: usize| {
                    Rect::from_min_max(
                        Pos2::new(
                            if columns[index].pin_rank() == 2 {
                                right_edge
                            } else if columns[index].pinned {
                                body.left()
                            } else {
                                frozen_edge
                            },
                            view.top(),
                        ),
                        Pos2::new(
                            if columns[index].pin_rank() == 2 {
                                view.right()
                            } else if columns[index].pinned {
                                frozen_edge
                            } else {
                                right_edge
                            },
                            view.bottom(),
                        ),
                    )
                };
                let find_column = |x: f32| -> Option<usize> {
                    (0..columns.len()).find(|&index| {
                        let clip = column_clip(index);
                        x >= column_left(index).max(clip.left())
                            && x < (column_left(index) + columns[index].width()).min(clip.right())
                    })
                };
                let fill_source = state
                    .fill_enabled
                    .then(|| state.selection.regions())
                    .filter(|regions| regions.len() == 1)
                    .map(|regions| regions[0])
                    .filter(|source| {
                        !state.read_only
                            && state.editor.is_none()
                            && row_id.is_some()
                            && source.top < source.bottom
                            && source.left < source.right
                            && source.bottom <= rows
                            && source.right <= columns.len()
                            && (source.left..source.right).all(|index| {
                                !matches!(
                                    columns[index].edit_kind,
                                    erp_grid_core::EditKind::ReadOnly
                                )
                            })
                    });
                let fill_handle = fill_source.map(|source| {
                    Rect::from_center_size(
                        Pos2::new(
                            column_left(source.right - 1) + columns[source.right - 1].width(),
                            origin.y + header_height + layout.top(source.bottom - 1) + row_height,
                        ),
                        Vec2::splat(8.0),
                    )
                });
                if response.hovered()
                    && state.gesture.is_none()
                    && pointer.is_some_and(|point| {
                        fill_handle.is_some_and(|handle| {
                            handle.intersect(body).is_positive() && handle.contains(point)
                        })
                    })
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                // Find an actual visible right edge, from either side of the divider.
                // A clipped off-screen edge must not become a resize handle at the viewport edge.
                let resize_target = pointer
                    .filter(|p| p.y >= header_y && p.y < header_y + column_header_height)
                    .and_then(|p| {
                        (0..columns.len())
                            .filter_map(|index| {
                                let edge = column_left(index) + columns[index].width();
                                let clip = column_clip(index);
                                (edge > clip.left()
                                    && edge <= clip.right()
                                    && (p.x - edge).abs() <= 6.0)
                                    .then_some((index, (p.x - edge).abs()))
                            })
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|(index, _)| index)
                    });
                if response.hovered() && state.editor.is_none() && resize_target.is_some() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
                if response.secondary_clicked() {
                    state.context_column = pointer
                        .filter(|p| p.y >= header_y && p.y < header_y + column_header_height)
                        .and_then(|p| find_column(p.x))
                        .map(|index| columns[index].id);
                    state.gesture = None;
                }
                if let Some(target) = state.context_column {
                    response.context_menu(|ui| {
                        for (label, descending, additive) in [
                            ("Sort ascending", false, false),
                            ("Sort descending", true, false),
                            ("Then ascending", false, true),
                            ("Then descending", true, true),
                        ] {
                            if ui
                                .add_enabled(state.sorting_enabled, egui::Button::new(label))
                                .clicked()
                            {
                                state.sort_column(target, Some(descending), additive);
                                output.sorts_changed = true;
                                ui.close();
                            }
                        }
                        if ui
                            .add_enabled(state.sorting_enabled, egui::Button::new("Clear sorting"))
                            .clicked()
                        {
                            state.sorts.clear();
                            output.sorts_changed = true;
                            ui.close();
                        }
                        if ui.button("Columns...").clicked() {
                            state.column_config_open = true;
                            ui.close();
                        }
                        if let Some(column) = columns.iter().find(|c| c.id == target) {
                            if ui
                                .add_enabled(columns.len() > 1, egui::Button::new("Hide column"))
                                .clicked()
                            {
                                state.column_action =
                                    Some(column_config::ColumnAction::Visibility(target, false));
                                ui.close();
                            }
                            if ui
                                .button(if column.pinned {
                                    "Unlock column"
                                } else {
                                    "Lock to left"
                                })
                                .clicked()
                            {
                                state.column_action =
                                    Some(column_config::ColumnAction::Pin(target, !column.pinned));
                                ui.close();
                            }
                            if ui
                                .button(if column.pin_rank() == 2 {
                                    "Unlock right column"
                                } else {
                                    "Lock to right"
                                })
                                .clicked()
                            {
                                state.column_action = Some(column_config::ColumnAction::PinRight(
                                    target,
                                    column.pin_rank() != 2,
                                ));
                                ui.close();
                            }
                        }
                    });
                }
                let hit_cell = |p: Pos2| -> Option<Cell> {
                    if rows == 0 || columns.is_empty() {
                        return None;
                    }
                    let x =
                        p.x.clamp(body.left(), (view.right() - 0.01).max(body.left()));
                    let y = (p.y - origin.y - header_height).max(0.0);
                    Some(Cell {
                        row: layout.hit(y)?,
                        column: find_column(x)?,
                    })
                };
                let tree_toggle_at = |p: Pos2| -> Option<(RowId, Rect)> {
                    let cell = hit_cell(p)?;
                    if cell.column != 0 {
                        return None;
                    }
                    let key = row_id.map(|map| map(cell.row))?;
                    let visual = state.tree_rows.get(&key)?;
                    let cell_rect = Rect::from_min_size(
                        Pos2::new(
                            column_left(0),
                            origin.y + header_height + layout.top(cell.row),
                        ),
                        Vec2::new(columns[0].width(), row_height),
                    );
                    let arrow = tree_render::arrow_rect(cell_rect, visual)?;
                    let hit = arrow.intersect(body).intersect(column_clip(0));
                    (hit.is_positive() && hit.contains(p)).then_some((key, arrow))
                };
                let tree_toggle = pointer.and_then(tree_toggle_at);
                let is_action_cell = |cell: Cell| {
                    options.present.is_some_and(|present| {
                        matches!(
                            present(CellContext {
                                row: cell.row,
                                row_id: row_id.map(|map| map(cell.row)),
                                column: columns[cell.column].id,
                                raw: &cell_text(cell.row, columns[cell.column].id),
                            }),
                            CellPresentation::Action { .. }
                        )
                    })
                };
                let action_hit = !(modifiers.shift || modifiers.ctrl || modifiers.command)
                    && pointer
                        .filter(|p| body.contains(*p))
                        .and_then(hit_cell)
                        .is_some_and(is_action_cell);
                if let Some((key, arrow)) = tree_toggle {
                    let toggle = ui.interact(
                        arrow.intersect(body).intersect(column_clip(0)),
                        id.with(("tree", key.0)),
                        Sense::click(),
                    );
                    let toggle = if let Some(error) = &state.tree_rows[&key].error {
                        toggle.on_hover_text(error)
                    } else {
                        toggle.on_hover_text("Expand / collapse tree row")
                    };
                    if toggle.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if toggle.clicked() {
                        output.toggle_tree = Some(key);
                    }
                }

                if response.secondary_clicked() {
                    state.context_cell = pointer.filter(|p| body.contains(*p)).and_then(hit_cell);
                    if let (Some(cell), Some(map)) = (state.context_cell, row_id) {
                        state.row_selection.active = Some(map(cell.row));
                        state.focus = Some(cell);
                    }
                }
                let mut begin_edit = None;
                if state.context_column.is_none()
                    && let Some(cell) = state.context_cell
                {
                    response.context_menu(|ui| {
                        let column = &columns[cell.column];
                        if ui.button("Copy cell").clicked() {
                            ui.ctx().copy_text(cell_text(cell.row, column.id));
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                !state.read_only
                                    && !matches!(
                                        column.edit_kind,
                                        erp_grid_core::EditKind::ReadOnly
                                    ),
                                egui::Button::new("Edit cell"),
                            )
                            .clicked()
                        {
                            begin_edit = Some(cell);
                            ui.close();
                        }
                        for action in &state.context_actions {
                            let reason = action.unavailable(state.row_selection.target_count());
                            let response = ui.add_enabled(
                                !state.read_only && reason.is_none(),
                                egui::Button::new(&action.label),
                            );
                            let response = if let Some(reason) = reason {
                                response.on_disabled_hover_text(reason)
                            } else {
                                response
                            };
                            if response.clicked() {
                                output.row_action =
                                    Some((action.id.clone(), state.row_selection.targets()));
                                ui.close();
                            }
                        }
                        for (label, invert) in [("Select page", false), ("Invert page", true)] {
                            if ui
                                .add_enabled(
                                    row_id.is_some() && rows <= 10_000,
                                    egui::Button::new(label),
                                )
                                .clicked()
                            {
                                if let Some(map) = row_id {
                                    for row in 0..rows {
                                        let id = map(row);
                                        let checked =
                                            !invert || !state.row_selection.checked().contains(&id);
                                        state.row_selection.set_checked(id, checked);
                                    }
                                }
                                ui.close();
                            }
                        }
                    });
                }
                if response.double_clicked()
                    && tree_toggle.is_none()
                    && let Some(p) = pointer
                    && body.contains(p)
                {
                    begin_edit = hit_cell(p);
                }
                if response.has_focus() && state.editor.is_none() {
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F2)) {
                        begin_edit = state.focus;
                    }
                    output.undo = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::Z));
                    output.redo = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::Y));
                    if !state.read_only
                        && let Some(focus) = state.focus
                    {
                        output.paste = ui.input(|i| {
                            i.events.iter().find_map(|event| {
                                if let egui::Event::Paste(text) = event {
                                    Some((focus, text.clone()))
                                } else {
                                    None
                                }
                            })
                        });
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::A))
                        && let Some(map) = row_id
                        && rows <= 10_000
                    {
                        for row in 0..rows {
                            state.row_selection.set_checked(map(row), true);
                        }
                    }
                }
                let mut requested = false;
                if !state.read_only
                    && state.editor.is_none()
                    && let Some((cell, expected_row, expected_column)) = state.requested_edit.take()
                {
                    if cell.row < rows
                        && columns
                            .get(cell.column)
                            .is_some_and(|c| c.id == expected_column)
                        && row_id.is_some_and(|map| map(cell.row) == expected_row)
                    {
                        begin_edit = Some(cell);
                        state.focus = Some(cell);
                        state.row_selection.active = Some(expected_row);
                        requested = true;
                    } else {
                        state.message = Some("Edit target changed; choose the cell again".into());
                    }
                }
                if let (Some(cell), Some(map)) = (begin_edit, row_id)
                    && !is_action_cell(cell)
                    && state.editor.is_none()
                    && !state.read_only
                    && !matches!(
                        columns[cell.column].edit_kind,
                        erp_grid_core::EditKind::ReadOnly
                    )
                {
                    let before = cell_text(cell.row, columns[cell.column].id);
                    state.column_config_open = false;
                    state.next_edit_ticket = state.next_edit_ticket.wrapping_add(1);
                    state.editor = Some(editor::Editor {
                        picker: Default::default(),
                        ticket: state.next_edit_ticket,
                        row: map(cell.row),
                        column: columns[cell.column].id,
                        draft: before.clone(),
                        before,
                        first: true,
                        composing: false,
                        focused: false,
                        awaiting: false,
                        retry_required: false,
                        external_conflict: false,
                    });
                    if requested {
                        let mut target = Rect::from_min_size(
                            Pos2::new(
                                column_left(cell.column),
                                origin.y + header_height + layout.top(cell.row),
                            ),
                            Vec2::new(columns[cell.column].width(), row_height),
                        );
                        if columns[cell.column].is_pinned() {
                            target.min.x = body.center().x;
                            target.max.x = body.center().x;
                        } else {
                            target.min.x -= pinned_width;
                            target.max.x += right_width;
                        }
                        ui.scroll_to_rect(target, None);
                    }
                    state.gesture = None;
                }
                if state.editor.is_none()
                    && (state.gesture.is_some() || response.has_focus() || response.lost_focus())
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
                {
                    match state.gesture.take() {
                        Some(Gesture::Select { baseline, .. }) => state.selection = baseline,
                        Some(Gesture::CheckRows { baseline, .. }) => state.row_selection = baseline,
                        Some(Gesture::Resize { column, width, .. }) => {
                            resize_request = Some((column, width))
                        }
                        Some(Gesture::Move { .. }) => {}
                        Some(Gesture::Fill { .. }) => {}
                        None => state.clear_selection(),
                    }
                }

                if response.hovered()
                    && state.editor.is_none()
                    && ui.input(|i| i.pointer.primary_pressed())
                    && !action_hit
                    && tree_toggle.is_none()
                    && pointer.is_some_and(|p| p.y < body.top() || hit_cell(p).is_some())
                {
                    response.request_focus();
                    state.message = None;
                    if let Some(p) = pointer {
                        if let (Some(source), Some(handle)) = (fill_source, fill_handle)
                            && handle.intersect(body).is_positive()
                            && handle.contains(p)
                        {
                            state.gesture = Some(Gesture::Fill { source });
                        } else if p.y >= header_y && p.y < header_y + column_header_height {
                            if let Some(index) = resize_target.or_else(|| find_column(p.x)) {
                                let column = &columns[index];
                                state.gesture = Some(if resize_target.is_some() {
                                    Gesture::Resize {
                                        column: column.id,
                                        start_x: p.x,
                                        width: column.width(),
                                    }
                                } else {
                                    Gesture::Move {
                                        column: column.id,
                                        start_x: p.x,
                                    }
                                });
                            }
                        } else if p.y >= body.top()
                            && find_column(p.x).is_some()
                            && p.y < origin.y + header_height + layout.height()
                            && let Some(cell) = hit_cell(p)
                        {
                            if (modifiers.shift || modifiers.command || modifiers.ctrl)
                                && let Some(map) = row_id
                            {
                                let anchor = if modifiers.shift {
                                    state
                                        .check_anchor
                                        .or(state.focus.map(|c| c.row))
                                        .unwrap_or(cell.row)
                                } else {
                                    cell.row
                                };
                                let checked =
                                    !state.row_selection.checked().contains(&map(cell.row));
                                state.gesture = Some(Gesture::CheckRows {
                                    anchor,
                                    baseline: state.row_selection.clone(),
                                    checked,
                                });
                                if !modifiers.shift {
                                    state.check_anchor = Some(cell.row);
                                }
                                state.focus = Some(cell);
                            } else if !modifiers.shift && !modifiers.command && !modifiers.ctrl {
                                state.anchor = Some(cell);
                                state.focus = Some(cell);
                                state.gesture = Some(Gesture::Select {
                                    anchor: cell,
                                    baseline: state.selection.clone(),
                                    operation: SelectionOperation::Replace,
                                });
                            } else {
                                state.message =
                                    Some("Row identity is required for modifier selection".into());
                            }
                        }
                    }
                }
                if let Some(p) = pointer {
                    match &state.gesture {
                        Some(Gesture::CheckRows {
                            anchor,
                            baseline,
                            checked,
                        }) => {
                            if let (Some(map), Some(cell)) = (row_id, hit_cell(p)) {
                                let first = (*anchor).min(cell.row);
                                let last = (*anchor).max(cell.row);
                                if last - first >= 10_000 {
                                    state.message = Some("一次连续勾选最多 10,000 行".into());
                                } else {
                                    let mut next = baseline.clone();
                                    for index in first..=last {
                                        next.set_checked(map(index), *checked);
                                    }
                                    state.row_selection = next;
                                    state.focus = Some(cell);
                                }
                            }
                            if ui.input(|i| i.pointer.primary_down()) {
                                let speed = 480.0 * ui.input(|i| i.stable_dt).min(0.05);
                                if p.y < body.top() {
                                    auto_scroll.y = -speed;
                                }
                                if p.y > body.bottom() {
                                    auto_scroll.y = speed;
                                }
                            }
                        }
                        Some(Gesture::Select {
                            anchor,
                            baseline,
                            operation,
                        }) => {
                            if let Some(cell) = hit_cell(p) {
                                let mut next = baseline.clone();
                                match next.apply(CellRect::between(*anchor, cell), *operation) {
                                    Ok(()) => {
                                        state.selection = next;
                                        state.focus = Some(cell);
                                    }
                                    Err(_) => {
                                        state.message =
                                            Some("选区过于零碎，请清除部分选区后重试".into())
                                    }
                                }
                            }
                            if ui.input(|i| i.pointer.primary_down()) {
                                let speed = 480.0 * ui.input(|i| i.stable_dt).min(0.05);
                                if p.y < body.top() {
                                    auto_scroll.y = -speed;
                                }
                                if p.y > body.bottom() {
                                    auto_scroll.y = speed;
                                }
                                if p.x < body.left() {
                                    auto_scroll.x = -speed;
                                }
                                if p.x > body.right() {
                                    auto_scroll.x = speed;
                                }
                            }
                        }
                        Some(Gesture::Resize {
                            column,
                            start_x,
                            width,
                        }) => {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            resize_request = Some((*column, width + p.x - start_x));
                        }
                        Some(Gesture::Move { column, start_x }) if (p.x - start_x).abs() > 5.0 => {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            if let Some(target) = find_column(p.x) {
                                let after =
                                    p.x > column_left(target) + columns[target].width() / 2.0;
                                let insertion = target + usize::from(after);
                                let x = column_left(target)
                                    + if after { columns[target].width() } else { 0.0 };
                                insertion_x = Some(x);
                                if ui.input(|i| i.pointer.primary_released())
                                    && columns
                                        .iter()
                                        .find(|c| c.id == *column)
                                        .is_some_and(|c| c.pin_rank() == columns[target].pin_rank())
                                {
                                    move_request =
                                        Some((*column, columns.get(insertion).map(|c| c.id)));
                                }
                            }
                        }
                        Some(Gesture::Fill { source }) => {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                            let pointer_row = if body.contains(p) {
                                layout.hit((p.y - origin.y - header_height).max(0.0))
                            } else {
                                None
                            };
                            if let Some(pointer_row) = pointer_row
                                && let Some(target) = fill_target(*source, pointer_row, rows)
                                    .or_else(|| {
                                        let column = find_column(p.x)?;
                                        if pointer_row < source.top || pointer_row >= source.bottom
                                        {
                                            return None;
                                        }
                                        if column < source.left {
                                            Some(CellRect {
                                                top: source.top,
                                                bottom: source.bottom,
                                                left: column,
                                                right: source.left,
                                            })
                                        } else if column >= source.right {
                                            Some(CellRect {
                                                top: source.top,
                                                bottom: source.bottom,
                                                left: source.right,
                                                right: column + 1,
                                            })
                                        } else {
                                            None
                                        }
                                    })
                            {
                                for row in
                                    target.top.max(visible.start)..target.bottom.min(visible.end)
                                {
                                    for (index, column) in columns
                                        .iter()
                                        .enumerate()
                                        .take(target.right)
                                        .skip(target.left)
                                    {
                                        let rect = Rect::from_min_size(
                                            Pos2::new(
                                                column_left(index),
                                                origin.y + header_height + layout.top(row),
                                            ),
                                            Vec2::new(column.width(), row_height),
                                        );
                                        ui.painter()
                                            .with_clip_rect(body.intersect(column_clip(index)))
                                            .rect_stroke(
                                                rect.shrink(2.0),
                                                0.0,
                                                Stroke::new(2.0, theme.accent),
                                                egui::StrokeKind::Inside,
                                            );
                                    }
                                }
                                if ui.input(|i| i.pointer.primary_released()) {
                                    output.fill = Some(FillIntent {
                                        source: *source,
                                        target,
                                    });
                                }
                            }
                        }
                        Some(Gesture::Move { .. }) | None => {}
                    }
                }
                if response.double_clicked()
                    && state.editor.is_none()
                    && let Some(index) = resize_target
                {
                    let column = &columns[index];
                    let measure = |text: String| {
                        ui.painter()
                            .layout_no_wrap(
                                text.chars().take(512).collect(),
                                FontId::proportional(metrics.font_size),
                                theme.text,
                            )
                            .size()
                            .x
                    };
                    let mut measured = measure(column.title.clone()) + 36.0;
                    for row in 0..rows.min(200) {
                        measured = measured.max(measure(cell_text(row, column.id)) + 18.0);
                        if measured >= 640.0 {
                            break;
                        }
                    }
                    resize_request = Some((column.id, measured));
                    state.gesture = None;
                }
                if response.clicked()
                    && state.sorting_enabled
                    && state.editor.is_none()
                    && !matches!(state.gesture, Some(Gesture::Resize { .. }))
                    && resize_target.is_none()
                    && let Some(p) = pointer
                    && p.y >= header_y
                    && p.y < header_y + column_header_height
                    && let Some(index) = find_column(p.x)
                {
                    state.sort_column(columns[index].id, None, modifiers.shift);
                    output.sorts_changed = true;
                }
                if !ui.input(|i| i.pointer.primary_down()) {
                    // Only an ordinary click activates a row. Dragging a copy rectangle does not.
                    if response.clicked()
                        && !modifiers.shift
                        && !modifiers.command
                        && !modifiers.ctrl
                        && tree_toggle.is_none()
                        && let (Some(map), Some(p)) = (row_id, pointer)
                        && body.contains(p)
                        && let Some(cell) = hit_cell(p)
                    {
                        state.row_selection.active = Some(map(cell.row));
                        state.check_anchor = Some(cell.row);
                    }
                    state.gesture = None;
                }
                if response.has_focus() && state.gesture.is_none() && state.editor.is_none() {
                    if row_id.is_some()
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Space))
                        && let (Some(map), Some(focus)) = (row_id, state.focus)
                    {
                        state.row_selection.toggle(map(focus.row));
                        state.check_anchor = Some(focus.row);
                    }
                    if ui.input(|i| {
                        i.events
                            .iter()
                            .any(|event| matches!(event, egui::Event::Copy))
                    }) {
                        match selection_tsv(&state.selection, columns, rows, &cell_text) {
                            Ok(text) if !text.is_empty() => ui.ctx().copy_text(text),
                            Ok(_) => {}
                            Err(error) => state.message = Some(error.into()),
                        }
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
                        state.clear_selection();
                    }
                    let mut focus = state.focus.unwrap_or(Cell { row: 0, column: 0 });
                    let mut changed = false;
                    if focus.row < rows
                        && let Some(key) = row_id.map(|map| map(focus.row))
                        && let Some(visual) = state.tree_rows.get(&key)
                    {
                        let key_pressed = if visual.has_children && !visual.expanded {
                            Some(Key::ArrowRight)
                        } else if visual.has_children && visual.expanded {
                            Some(Key::ArrowLeft)
                        } else {
                            None
                        };
                        if let Some(key_pressed) = key_pressed
                            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key_pressed))
                        {
                            output.toggle_tree = Some(key);
                        }
                    }
                    for (key, dr, dc) in [
                        (Key::ArrowDown, 1, 0),
                        (Key::ArrowUp, -1, 0),
                        (Key::ArrowRight, 0, 1),
                        (Key::ArrowLeft, 0, -1),
                        (Key::Home, 0, -(columns.len() as isize)),
                        (Key::End, 0, columns.len() as isize),
                        (
                            Key::PageUp,
                            -((body.height() / row_height) as isize).max(1),
                            0,
                        ),
                        (
                            Key::PageDown,
                            ((body.height() / row_height) as isize).max(1),
                            0,
                        ),
                    ] {
                        if ui.input_mut(|i| i.consume_key(modifiers, key)) {
                            focus.row = focus
                                .row
                                .saturating_add_signed(dr)
                                .min(rows.saturating_sub(1));
                            focus.column = focus
                                .column
                                .saturating_add_signed(dc)
                                .min(columns.len().saturating_sub(1));
                            changed = rows > 0 && !columns.is_empty();
                        }
                    }
                    if changed {
                        if modifiers.shift || modifiers.command || modifiers.ctrl {
                            if let Some(map) = row_id {
                                let anchor = if modifiers.shift {
                                    state
                                        .check_anchor
                                        .or(state.focus.map(|c| c.row))
                                        .unwrap_or(focus.row)
                                } else {
                                    focus.row
                                };
                                let first = anchor.min(focus.row);
                                let last = anchor.max(focus.row);
                                if last - first < 10_000 {
                                    let checked =
                                        !state.row_selection.checked().contains(&map(focus.row));
                                    for index in first..=last {
                                        state.row_selection.set_checked(map(index), checked);
                                    }
                                    if !modifiers.shift {
                                        state.check_anchor = Some(focus.row);
                                    }
                                } else {
                                    state.message = Some("一次连续勾选最多 10,000 行".into());
                                }
                            }
                        } else {
                            if let Some(map) = row_id {
                                state.row_selection.active = Some(map(focus.row));
                                state.check_anchor = Some(focus.row);
                            }
                            state.anchor = Some(focus);
                            let _ = state.selection.apply(
                                CellRect::between(focus, focus),
                                SelectionOperation::Replace,
                            );
                        }
                        state.focus = Some(focus);
                        let cell = Rect::from_min_size(
                            origin
                                + Vec2::new(
                                    positions[focus.column],
                                    header_height + layout.top(focus.row),
                                ),
                            Vec2::new(columns[focus.column].width(), row_height),
                        );
                        let mut target = cell.expand2(Vec2::new(0.0, column_header_height));
                        if columns[focus.column].is_pinned() {
                            target.min.x = body.center().x;
                            target.max.x = body.center().x;
                        } else {
                            target.min.x -= pinned_width;
                            target.max.x += right_width;
                        }
                        ui.scroll_to_rect(target, None);
                    }
                }

                // Host initiated focus (for example, local find) is applied
                // once after layout is known, without changing selection.
                if let Some(focus) = state.focus_request.take()
                    && focus.row < rows
                    && focus.column < columns.len()
                {
                    let cell = Rect::from_min_size(
                        origin
                            + Vec2::new(
                                positions[focus.column],
                                header_height + layout.top(focus.row),
                            ),
                        Vec2::new(columns[focus.column].width(), row_height),
                    );
                    let mut target = cell.expand2(Vec2::new(0.0, column_header_height));
                    if columns[focus.column].is_pinned() {
                        target.min.x = body.center().x;
                        target.max.x = body.center().x;
                    } else {
                        target.min.x -= pinned_width;
                        target.max.x += right_width;
                    }
                    ui.scroll_to_rect(target, None);
                }

                let visible_columns: Vec<_> = (0..columns.len())
                    .filter(|&index| {
                        column_left(index) < column_clip(index).right()
                            && column_left(index) + columns[index].width()
                                > column_clip(index).left()
                    })
                    .collect();
                for row in visible.clone() {
                    for &index in &visible_columns {
                        let column = &columns[index];
                        let cell_clip = body.intersect(column_clip(index));
                        let painter = ui.painter().with_clip_rect(cell_clip);
                        let rect = Rect::from_min_size(
                            Pos2::new(
                                column_left(index),
                                origin.y + header_height + layout.top(row),
                            ),
                            Vec2::new(column.width(), row_height),
                        );
                        if !rect.intersects(body) {
                            continue;
                        }
                        let selected = state.selection.contains(Cell { row, column: index });
                        let color = if selected {
                            theme.selected_cell
                        } else if row_id
                            .is_some_and(|map| state.row_selection.checked().contains(&map(row)))
                        {
                            theme.checked_row
                        } else if row_id
                            .is_some_and(|map| state.row_selection.active == Some(map(row)))
                        {
                            theme.active_row
                        } else if row % 2 == 0 {
                            theme.row
                        } else {
                            theme.alternate_row
                        };
                        painter.rect_filled(rect, 0.0, color);
                        painter.line_segment(
                            [rect.left_bottom(), rect.right_bottom()],
                            Stroke::new(1.0, theme.grid_line),
                        );
                        let tree_visual = (index == 0)
                            .then(|| row_id.map(|map| map(row)))
                            .flatten()
                            .and_then(|key| state.tree_rows.get(&key));
                        let raw = cell_text(row, column.id);
                        let presentation =
                            options
                                .present
                                .map_or(CellPresentation::Default, |present| {
                                    present(CellContext {
                                        row,
                                        row_id: row_id.map(|map| map(row)),
                                        column: column.id,
                                        raw: &raw,
                                    })
                                });
                        let text = match presentation {
                            CellPresentation::Default => Some(raw),
                            CellPresentation::Text(text) => Some(text),
                            CellPresentation::Badge { label, color } => {
                                painter.rect_filled(rect.shrink2(Vec2::new(4.0, 3.0)), 3.0, color);
                                Some(label)
                            }
                            CellPresentation::Image { texture, tooltip } => {
                                let target = Rect::from_center_size(
                                    rect.center(),
                                    Vec2::splat((row_height - 4.0).min(rect.width())),
                                );
                                painter.image(
                                    texture,
                                    target,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                                ui.interact(
                                    rect.intersect(cell_clip),
                                    id.with((
                                        "cell-image",
                                        row_id.map(|map| map(row)),
                                        row,
                                        column.id.0,
                                    )),
                                    Sense::hover(),
                                )
                                .on_hover_text(tooltip);
                                None
                            }
                            CellPresentation::Action {
                                id: action,
                                label,
                                enabled,
                            } => {
                                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                    ui.set_clip_rect(rect.intersect(cell_clip));
                                    ui.push_id(
                                        (
                                            id,
                                            "cell-action",
                                            row_id.map(|map| map(row)),
                                            row,
                                            column.id.0,
                                        ),
                                        |ui| {
                                            if !(enabled
                                                && row_id.is_some()
                                                && state.editor.is_none()
                                                && !(modifiers.shift
                                                    || modifiers.ctrl
                                                    || modifiers.command)
                                                && state.gesture.is_none())
                                            {
                                                ui.disable();
                                            }
                                            if ui
                                                .put(rect.shrink(2.0), egui::Button::new(label))
                                                .clicked()
                                                && let Some(map) = row_id
                                            {
                                                output.cell_action = Some(CellAction {
                                                    row: map(row),
                                                    column: column.id,
                                                    action,
                                                });
                                            }
                                        },
                                    );
                                });
                                None
                            }
                        };
                        if let Some(text) = text {
                            cell_text::paint(
                                &painter,
                                rect,
                                tree_visual.map_or(rect.left() + 8.0, |visual| {
                                    tree_render::text_left(rect, visual)
                                }),
                                text,
                                if selected {
                                    theme.selected_text
                                } else {
                                    theme.text
                                },
                            );
                        }
                        if let Some(visual) = tree_visual {
                            tree_render::paint(
                                ui,
                                cell_clip,
                                rect,
                                visual,
                                if selected {
                                    theme.selected_text
                                } else {
                                    theme.text
                                },
                                theme.accent,
                            );
                        }
                        if state.focus == Some(Cell { row, column: index }) {
                            painter.rect_stroke(
                                rect.shrink(1.0),
                                0.0,
                                Stroke::new(1.5, theme.accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if let Some(editor) = &mut state.editor
                            && editor.column == column.id
                            && row_id.is_some_and(|map| editor.row == map(row))
                            && rect.intersect(cell_clip).is_positive()
                        {
                            output.editor_drawn = true;
                            let outcome = ui
                                .scope(|ui| {
                                    ui.set_clip_rect(cell_clip);
                                    editor::show(ui, rect, id.with("cell-editor"), column, editor)
                                })
                                .inner;
                            output.edit_navigation = outcome.navigation;
                            if let Some(edit) = outcome.edit {
                                output.edit_ticket = Some(editor.ticket);
                                output.edits.push(edit);
                            }
                            if let Some(error) = outcome.error {
                                state.message = Some(error);
                            }
                            if outcome.finished {
                                state.editor = None;
                            }
                        }
                        output.painted_cells += 1;
                    }
                }
                accessibility::expose(
                    ui,
                    accessibility::VisibleGrid {
                        id,
                        response: &response,
                        bounds: interaction,
                        rows,
                        row_number_start: state.row_number_start,
                        columns,
                        visible_rows: visible.clone(),
                        visible_columns: &visible_columns,
                        state,
                        row_id,
                    },
                    &cell_text,
                    |row, index| {
                        Rect::from_min_size(
                            Pos2::new(
                                column_left(index),
                                origin.y + header_height + layout.top(row),
                            ),
                            Vec2::new(columns[index].width(), row_height),
                        )
                        .intersect(body)
                        .intersect(column_clip(index))
                    },
                    |index| {
                        Rect::from_min_size(
                            Pos2::new(column_left(index), header_y),
                            Vec2::new(columns[index].width(), column_header_height),
                        )
                        .intersect(view)
                        .intersect(column_clip(index))
                    },
                );
                // On very short hosts, totals start below the header and the
                // viewport clips any remainder instead of covering controls.
                let summary_top = body.bottom();
                for (summary_index, summary) in summary_rows.iter().enumerate() {
                    let y = summary_top + summary_index as f32 * row_height;
                    for &index in &visible_columns {
                        let column = &columns[index];
                        let rect = Rect::from_min_size(
                            Pos2::new(column_left(index), y),
                            Vec2::new(column.width(), row_height),
                        );
                        let clip =
                            Rect::from_min_max(Pos2::new(view.left(), summary_top), view.max)
                                .intersect(column_clip(index));
                        let painter = ui.painter().with_clip_rect(clip);
                        painter.rect_filled(rect, 0.0, theme.structure);
                        painter.line_segment(
                            [rect.left_top(), rect.right_top()],
                            Stroke::new(1.0, theme.structure_line),
                        );
                        if let Some(value) = summary.cells.get(&column.id) {
                            painter
                                .with_clip_rect(rect.shrink2(Vec2::new(8.0, 0.0)).intersect(clip))
                                .text(
                                    rect.left_center() + Vec2::new(8.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    value,
                                    FontId::proportional(metrics.font_size),
                                    theme.text,
                                );
                        }
                    }
                }
                if let Some(handle) = fill_handle
                    && handle.intersect(body).is_positive()
                {
                    ui.painter()
                        .with_clip_rect(body)
                        .rect_filled(handle, 1.0, theme.accent);
                }
                let header = ui.painter().with_clip_rect(Rect::from_min_max(
                    view.min,
                    Pos2::new(view.right(), body.top()),
                ));
                header.rect_filled(
                    Rect::from_min_max(view.min, Pos2::new(view.right(), body.top())),
                    0.0,
                    theme.header,
                );
                if group_height > 0.0 {
                    header_groups::paint_header_groups(
                        &header,
                        &state.header_groups,
                        columns,
                        Rect::from_min_max(view.min, Pos2::new(view.right(), header_y)),
                        |column_id| {
                            columns.iter().position(|c| c.id == column_id).map(|index| {
                                Rect::from_min_size(
                                    Pos2::new(column_left(index), view.top()),
                                    Vec2::new(columns[index].width(), group_height),
                                )
                            })
                        },
                        |rank| {
                            let (left, right) = match rank {
                                0 => (view.left(), frozen_edge),
                                2 => (right_edge, view.right()),
                                _ => (frozen_edge, right_edge),
                            };
                            Rect::from_min_max(
                                Pos2::new(left, view.top()),
                                Pos2::new(right, header_y),
                            )
                        },
                        &theme,
                    );
                }
                for &index in &visible_columns {
                    let column = &columns[index];
                    let rect = Rect::from_min_size(
                        Pos2::new(column_left(index), header_y),
                        Vec2::new(column.width(), column_header_height),
                    );
                    header
                        .with_clip_rect(rect.intersect(column_clip(index)))
                        .text(
                            rect.left_center() + Vec2::new(8.0, 0.0),
                            Align2::LEFT_CENTER,
                            state
                                .sorts
                                .iter()
                                .position(|rule| rule.column == column.id)
                                .map_or_else(
                                    || column.title.clone(),
                                    |index| {
                                        format!(
                                            "{} {}{}",
                                            column.title,
                                            if state.sorts[index].descending {
                                                "↓"
                                            } else {
                                                "↑"
                                            },
                                            index + 1
                                        )
                                    },
                                ),
                            FontId::proportional(metrics.font_size),
                            theme.header_text,
                        );
                    header.with_clip_rect(column_clip(index)).line_segment(
                        [rect.right_top(), rect.right_bottom()],
                        Stroke::new(1.0, theme.header_line),
                    );
                }
                if state.quick_search_open {
                    let search_clip = Rect::from_min_max(
                        Pos2::new(body.left(), header_y + column_header_height),
                        Pos2::new(view.right(), body.top()),
                    );
                    ui.scope(|ui| {
                        if !state.filtering_enabled {
                            ui.disable();
                        }
                        ui.set_clip_rect(search_clip);
                        for &index in &visible_columns {
                            let column = &columns[index];
                            if matches!(column.filter, erp_grid_core::FilterKind::None) {
                                state.quick_search.remove(&column.id);
                                continue;
                            }
                            let rect = Rect::from_min_size(
                                Pos2::new(column_left(index) + 3.0, search_clip.top() + 2.0),
                                Vec2::new(column.width() - 6.0, search_height - 4.0),
                            )
                            .intersect(search_clip)
                            .intersect(column_clip(index));
                            if rect.width() > 8.0 {
                                let value =
                                    state.quick_search.entry(column.id).or_insert_with(|| {
                                        erp_grid_core::FilterDraft::for_kind(&column.filter)
                                    });
                                ui.set_clip_rect(search_clip.intersect(column_clip(index)));
                                output.quick_search_changed |= quick_search::show(
                                    ui,
                                    rect,
                                    id.with(("search", column.id.0)),
                                    column,
                                    value,
                                );
                            }
                        }
                    });
                }
                {
                    let rail = Rect::from_min_max(
                        Pos2::new(view.left() - gutter, view.top()),
                        Pos2::new(view.left(), view.bottom()),
                    );
                    let rail_body = Rect::from_min_max(
                        Pos2::new(rail.left(), body.top()),
                        Pos2::new(rail.right(), body.bottom()),
                    );
                    let painter = ui.ctx().layer_painter(ui.layer_id()).with_clip_rect(rail);
                    painter.rect_filled(rail, 0.0, theme.structure);
                    painter.text(
                        Pos2::new(
                            rail.left() + CHECK_WIDTH + (gutter - CHECK_WIDTH) / 2.0,
                            header_y + column_header_height / 2.0,
                        ),
                        Align2::CENTER_CENTER,
                        "序",
                        FontId::proportional(metrics.font_size),
                        theme.header_text,
                    );
                    painter.line_segment(
                        [
                            Pos2::new(rail.left() + CHECK_WIDTH, rail.top()),
                            Pos2::new(rail.left() + CHECK_WIDTH, rail.bottom()),
                        ],
                        Stroke::new(1.0, theme.structure_line),
                    );
                    for (summary_index, summary) in summary_rows.iter().enumerate() {
                        let rect = Rect::from_min_size(
                            Pos2::new(rail.left(), summary_top + summary_index as f32 * row_height),
                            Vec2::new(rail.width(), row_height),
                        );
                        painter.rect_filled(rect, 0.0, theme.structure);
                        painter.line_segment(
                            [rect.left_top(), rect.right_top()],
                            Stroke::new(1.0, theme.structure_line),
                        );
                        painter
                            .with_clip_rect(rect.shrink2(Vec2::new(7.0, 0.0)))
                            .text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                &summary.label,
                                FontId::proportional(metrics.font_size),
                                theme.header_text,
                            );
                    }
                    ui.scope(|ui| {
                        ui.set_clip_rect(rail_body);
                        let visuals = &mut ui.style_mut().visuals;
                        for widget in [
                            &mut visuals.widgets.inactive,
                            &mut visuals.widgets.hovered,
                            &mut visuals.widgets.active,
                            &mut visuals.widgets.noninteractive,
                        ] {
                            widget.bg_stroke = Stroke::new(1.0, theme.muted_text);
                            widget.fg_stroke = Stroke::new(1.5, theme.text);
                        }

                        for row in visible.clone() {
                            let y = origin.y + header_height + layout.top(row);
                            painter.with_clip_rect(rail_body).text(
                                Pos2::new(
                                    rail.left() + CHECK_WIDTH + (gutter - CHECK_WIDTH) / 2.0,
                                    y + row_height / 2.0,
                                ),
                                Align2::CENTER_CENTER,
                                state
                                    .row_number_start
                                    .saturating_add(row as u64)
                                    .saturating_add(1)
                                    .to_string(),
                                FontId::proportional(metrics.font_size),
                                theme.muted_text,
                            );
                            let rect = Rect::from_min_size(
                                Pos2::new(
                                    rail.left() + 7.0,
                                    y + (row_height - 20.0).max(0.0) / 2.0,
                                ),
                                Vec2::splat(20.0_f32.min(row_height)),
                            );
                            if !rect.intersects(ui.clip_rect()) {
                                continue;
                            }
                            let row_key = row_id.map(|map| map(row));
                            if details_enabled && let Some(key) = row_key {
                                let toggle = Rect::from_min_size(
                                    Pos2::new(
                                        rail.left() + CHECK_WIDTH + 1.0,
                                        y + (row_height - 20.0).max(0.0) / 2.0,
                                    ),
                                    Vec2::new(15.0, 20.0_f32.min(row_height)),
                                );
                                if ui
                                    .put(
                                        toggle,
                                        egui::Button::new(if layout.expanded(row) {
                                            "▾"
                                        } else {
                                            "▸"
                                        })
                                        .frame(false),
                                    )
                                    .on_hover_text("Expand / collapse row details")
                                    .clicked()
                                {
                                    output.toggle_detail = Some(key);
                                }
                            }

                            let mut checked = row_key
                                .is_some_and(|key| state.row_selection.checked().contains(&key));
                            ui.push_id(
                                id.with(("check", row_key.map_or(row as u128, |key| key.0))),
                                |ui| {
                                    ui.add_enabled_ui(row_key.is_some(), |ui| {
                                        if ui
                                            .put(rect, egui::Checkbox::without_text(&mut checked))
                                            .changed()
                                            && let Some(map) = row_id
                                        {
                                            let anchor = if modifiers.shift {
                                                state.check_anchor.unwrap_or(row)
                                            } else {
                                                row
                                            };
                                            let first = row.min(anchor);
                                            let last = row.max(anchor);
                                            if last - first >= 10_000 {
                                                state.message = Some(
                                                    "一次连续勾选最多 10,000 行，请缩小范围".into(),
                                                );
                                            } else {
                                                for index in first..=last {
                                                    state
                                                        .row_selection
                                                        .set_checked(map(index), checked);
                                                }
                                                state.check_anchor = Some(row);
                                            }
                                        }
                                    });
                                },
                            );
                        }
                    });
                    ui.scope(|ui| {
                        ui.set_clip_rect(rail);
                        let visuals = &mut ui.style_mut().visuals;
                        for widget in [
                            &mut visuals.widgets.inactive,
                            &mut visuals.widgets.hovered,
                            &mut visuals.widgets.active,
                            &mut visuals.widgets.noninteractive,
                        ] {
                            widget.bg_stroke = Stroke::new(1.0, theme.muted_text);
                            widget.fg_stroke = Stroke::new(1.5, theme.text);
                        }

                        let selectable = row_id.is_some() && rows > 0 && rows <= 10_000;
                        let checked_count = if rows <= 10_000 {
                            row_id.map_or(0, |map| {
                                (0..rows)
                                    .filter(|&row| {
                                        state.row_selection.checked().contains(&map(row))
                                    })
                                    .count()
                            })
                        } else {
                            0
                        };
                        let mut all = rows > 0 && checked_count == rows;
                        ui.add_enabled_ui(selectable, |ui| {
                            let response = ui
                                .put(
                                    Rect::from_min_size(
                                        rail.min
                                            + Vec2::new(
                                                7.0,
                                                (column_header_height - 20.0).max(0.0) / 2.0,
                                            ),
                                        Vec2::splat(20.0_f32.min(column_header_height)),
                                    ),
                                    egui::Checkbox::without_text(&mut all)
                                        .indeterminate(checked_count > 0 && checked_count < rows),
                                )
                                .on_hover_text("当页全选 / 取消全选；右键反选当页");
                            if (response.clicked() || response.secondary_clicked())
                                && let Some(map) = row_id
                            {
                                for row in 0..rows {
                                    let key = map(row);
                                    let checked = if response.secondary_clicked() {
                                        !state.row_selection.checked().contains(&key)
                                    } else {
                                        all
                                    };
                                    state.row_selection.set_checked(key, checked);
                                }
                            }
                        });
                    });
                }
                for &(row, key) in expanded {
                    if !visible.contains(&row) {
                        continue;
                    }
                    let rect = Rect::from_min_size(
                        Pos2::new(
                            body.left() - gutter,
                            origin.y + header_height + layout.top(row) + metrics.row_height,
                        ),
                        Vec2::new(view.width() + gutter, row_layout::DETAIL_HEIGHT),
                    );
                    let clip = rect.intersect(Rect::from_min_max(
                        Pos2::new(body.left() - gutter, body.top()),
                        view.max,
                    ));
                    if clip.is_positive() {
                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .id_salt(id.with(("detail", key.0)))
                                .max_rect(rect.shrink(4.0)),
                            |ui| {
                                ui.set_clip_rect(clip);
                                ui.painter().rect_filled(rect, 0.0, theme.detail);
                                detail(ui, key);
                            },
                        );
                    }
                }
                if let Some(x) = insertion_x {
                    ui.painter().line_segment(
                        [Pos2::new(x, view.top()), Pos2::new(x, body.bottom())],
                        Stroke::new(2.0, theme.accent),
                    );
                }
            });
        output.content_height = scroll.content_size.y;
        output.viewport_height = scroll.inner_rect.height();
        state.scroll = scroll.state.offset + auto_scroll;
        state.scroll = state.scroll.max(Vec2::ZERO);
        if auto_scroll != Vec2::ZERO {
            ui.ctx().request_repaint();
        }
        if let Some((id, width)) = resize_request
            && let Some(column) = columns.iter_mut().find(|c| c.id == id)
        {
            let before = column.width();
            column.resize(width);
            output.columns_changed |= before != column.width();
            if output.columns_changed {
                ui.ctx().request_repaint();
            }
        }
        if let Some((source, before)) = move_request {
            output.columns_changed |= move_column(columns, source, before);
            // Positional selection must never silently identify different cells after reorder.
            state.clear_selection();
            state.focus = None;
        }
        output
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod column_tests;

#[cfg(test)]
mod editor_tests;

#[cfg(test)]
mod layout_tests;

#[cfg(test)]
mod action_tests;
#[cfg(test)]
mod resize_tests;

#[cfg(test)]
mod fill_tests;

#[cfg(test)]
mod extensions_tests;
#[cfg(test)]
mod summary_tests;
