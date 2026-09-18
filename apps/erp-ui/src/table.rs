use crate::{
    components::{self as export, Delivery, Format},
    data::{DataSet, Kind},
    query::{QueryJob, Summary},
};
use eframe::egui;
use erp_grid_core::*;
use erp_ui_egui::{DataGrid, FillIntent, GridState, show_pagination};

mod find;
mod navigation;
mod refresh;

pub struct TableView {
    pub scope: String,
    pub columns: Vec<Column>,
    pub state: GridState,
    pub pagination: Pagination,
    pub data: DataSet,
    result: Option<Vec<usize>>,
    query: Option<QueryJob>,
    query_scope: Option<refresh::QueryScope>,
    query_sorts: Vec<SortRule>,
    query_sort_changed: bool,
    resume_editor: Option<(usize, RowId, ColumnId)>,
    navigation_scan: usize,
    generation: u64,
    deadline: Option<f64>,
    composing: bool,
    valid: bool,
    summary: Summary,
    summary_scope: usize,
    pending: Option<Vec<CellEdit>>,
    pending_preview: Option<erp_ui_egui::ChangePreview>,
    pending_error: Option<String>,
    export_open: bool,
    export_scope: usize,
    export_format: Format,
    delivery: Option<Delivery>,
    pub event: Option<(Vec<RowId>, String)>,
    pub notice: Option<String>,
    /// Opt-in per page; bottom master-detail is the default.
    pub inline_details_enabled: bool,
    pub detail_edits: crate::shared_edits::SharedEdits,
    inline: std::collections::BTreeMap<RowId, Box<TableView>>,
    expanded: std::collections::BTreeSet<RowId>,
    seen_revision: u64,
    query_processing_seconds: f64,
    load_seconds: Option<f64>,
    selected_summary: crate::components::SelectionSummary,
    pub named_views: Option<crate::components::NamedViewsUi>,
    named_views_open: bool,
    find_open: bool,
    find_query: String,
    find_case: bool,
    find_cursor: Option<FindCursor>,
    find_columns: Vec<ColumnId>,
}
impl TableView {
    pub fn new(scope: &str, kind: Kind) -> Self {
        let mut state = GridState::default();
        if kind != Kind::Logs {
            state.context_actions = vec![
                erp_grid_core::actions::ActionSpec::new("ready", "Set Ready", 10_000),
                erp_grid_core::actions::ActionSpec::new("hold", "Set Hold", 10_000),
            ];
        }
        let mut view = Self {
            scope: scope.into(),
            columns: crate::data::columns(kind),
            state,
            pagination: Pagination::default(),
            data: DataSet::new(kind),
            result: None,
            query: None,
            query_scope: None,
            query_sorts: Vec::new(),
            query_sort_changed: false,
            resume_editor: None,
            navigation_scan: 0,
            generation: 1,
            deadline: None,
            composing: false,
            valid: true,
            summary: Summary::default(),
            summary_scope: 0,
            pending: None,
            pending_preview: None,
            pending_error: None,
            export_open: false,
            export_scope: 0,
            export_format: Format::Csv,
            delivery: None,
            event: None,
            notice: None,
            inline_details_enabled: false,
            detail_edits: Default::default(),
            inline: Default::default(),
            expanded: Default::default(),
            seen_revision: 0,
            query_processing_seconds: 0.0,
            load_seconds: None,
            selected_summary: Default::default(),
            named_views: None,
            named_views_open: false,
            find_open: false,
            find_query: String::new(),
            find_case: false,
            find_cursor: None,
            find_columns: Vec::new(),
        };
        view.state.header_groups = match kind {
            Kind::Products => vec![
                HeaderGroup::new("Product", [ColumnId(0), ColumnId(1)]),
                HeaderGroup::new("Inventory", [ColumnId(2), ColumnId(3), ColumnId(4)]),
                HeaderGroup::new("Context", [ColumnId(5), ColumnId(6)]),
            ],
            Kind::Goods => vec![
                HeaderGroup::new("Item", [ColumnId(0), ColumnId(1)]),
                HeaderGroup::new("Inventory", [ColumnId(2), ColumnId(3), ColumnId(4)]),
                HeaderGroup::new("Context", [ColumnId(5), ColumnId(6)]),
            ],
            Kind::Logs => Vec::new(),
        };
        view.start_query();
        view
    }
    pub fn dirty(&self) -> bool {
        self.state.is_editing()
            || self.pending.is_some()
            || self.inline.values().any(|view| view.dirty())
    }
    pub fn hold_for_leave(&mut self) {
        self.state.hold_edit();
        for view in self.inline.values_mut() {
            view.hold_for_leave();
        }
    }
    pub fn busy(&self) -> bool {
        self.query.is_some()
            || self.deadline.is_some()
            || !self.valid
            || self.resume_editor.is_some()
    }
    pub fn discard_draft(&mut self) {
        for view in self.inline.values_mut() {
            view.discard_draft();
        }
        self.state.cancel_edit();
        self.pending = None;
        self.pending_preview = None;
        self.pending_error = None;
    }
    pub fn save_for_leave(&mut self) -> Result<bool, String> {
        for view in self.inline.values_mut() {
            if !view.save_for_leave()? {
                return Ok(false);
            }
        }
        if self.pending.is_some() {
            return Ok(false);
        }
        let Some(edit) = self.state.edit_draft() else {
            return Ok(true);
        };
        if !self.apply(vec![edit]) {
            return Err(self
                .pending_error
                .clone()
                .unwrap_or_else(|| "Save failed; correct the editor value before leaving".into()));
        }
        self.state.cancel_edit();
        Ok(!self.dirty())
    }
    pub fn set_rows(&mut self, rows: usize) {
        self.data.rows = rows;
        self.result = None;
        self.generation += 1;
        self.start_query();
    }
    pub fn bind_parent(&mut self, parent: Option<RowId>, logs: Vec<String>) {
        if self.data.parent == parent && self.data.logs == logs {
            return;
        }
        self.data.edits.clear_history();
        self.data.parent = parent;
        self.data.edits.scope = parent;
        self.data.logs = logs;
        self.data.rows = if parent.is_none() {
            0
        } else if self.data.kind == Kind::Logs {
            self.data.logs.len()
        } else {
            30
        };
        self.result = None;
        self.generation += 1;
        self.start_query();
    }
    pub fn start_query(&mut self) {
        self.find_cursor = None;
        self.query_processing_seconds = 0.0;
        self.load_seconds = None;
        self.seen_revision = self.data.edits.revision();
        self.query = None;
        self.deadline = None;
        self.valid = false;
        let mut filters = Vec::new();
        for column in &self.columns {
            if let Some(draft) = self.state.quick_search.get(&column.id) {
                match draft.compile(&column.filter) {
                    Ok(Some(filter)) => filters.push((column.id, filter)),
                    Ok(None) => {}
                    Err(error) => {
                        self.notice = Some(format!("{}: {error}", column.title));
                        return;
                    }
                }
            }
        }
        filters.sort_by_key(|(column, _)| *column);
        let scope = refresh::QueryScope {
            kind: self.data.kind,
            parent: self.data.parent,
            rows: self.data.rows,
            filters: filters.clone(),
        };
        let cleared = self.query_scope.as_ref() != Some(&scope)
            && !self.state.row_selection.checked().is_empty();
        if self.query_scope.as_ref() != Some(&scope) {
            self.resume_editor = None;
            self.generation += 1;
            self.state.bind_view(self.generation);
            self.pagination.set_page(0, self.total());
        }
        self.query_scope = Some(scope);
        self.query_sort_changed = self.query_sorts != self.state.sorts;
        self.query_sorts = self.state.sorts.clone();
        self.valid = true;
        self.notice =
            cleared.then(|| "Checked rows cleared because the query scope changed".into());
        self.query = Some(QueryJob::new(filters, self.state.sorts.clone()));
    }
    pub fn tick(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if self.seen_revision != self.data.edits.revision() && !self.dirty() {
            self.start_query();
        }
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Ime(event) = event {
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
        });
        if self.deadline.is_some_and(|at| now >= at) && !self.composing && !self.dirty() {
            self.start_query();
        }
        if let Some(job) = &mut self.query {
            let step_started = web_time::Instant::now();
            let result = job.advance(&self.data, &self.columns);
            self.query_processing_seconds += step_started.elapsed().as_secs_f64();
            if let Some(result) = result {
                self.summary = std::mem::take(&mut job.summary);
                self.load_seconds = Some(self.query_processing_seconds);
                self.query = None;
                self.publish_result(result);
            }
            ctx.request_repaint();
        }
        if let Some(deadline) = self.deadline
            && !self.composing
            && !self.dirty()
        {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                (deadline - now).max(0.0),
            ));
        }
        if self.query.is_none()
            && self.deadline.is_none()
            && self.valid
            && self.resume_editor.is_some()
        {
            self.resume_editing();
            ctx.request_repaint();
        }
        if let Some(delivery) = &self.delivery {
            if let Some(result) = delivery.poll() {
                self.notice =
                    Some(result.unwrap_or_else(|error| format!("Export failed: {error}")));
                self.delivery = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
    }
    fn total(&self) -> usize {
        self.result.as_ref().map_or(self.data.rows, Vec::len)
    }
    fn source(&self, index: usize) -> usize {
        self.result.as_ref().map_or(index, |rows| rows[index])
    }
    fn apply(&mut self, changes: Vec<CellEdit>) -> bool {
        let targets: Vec<_> = changes
            .iter()
            .map(|edit| edit.row)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let count = changes.len();
        match self.data.apply(changes, &self.columns) {
            Ok(()) => {
                self.event = Some((targets, format!("Applied {count} cell changes")));
                self.pending = None;
                self.pending_preview = None;
                self.pending_error = None;
                self.start_query();
                true
            }
            Err(error) => {
                self.pending_error = Some(error.clone());
                self.notice = Some(error);
                false
            }
        }
    }
    fn planner_rows(
        &self,
        rect: CellRect,
        columns: &[Column],
    ) -> Result<Vec<erp_grid_core::edit_planner::IndexedRow>, String> {
        let height = rect
            .bottom
            .checked_sub(rect.top)
            .filter(|n| *n > 0)
            .ok_or("Empty or reversed row range")?;
        let width = rect
            .right
            .checked_sub(rect.left)
            .filter(|n| *n > 0)
            .ok_or("Empty or reversed column range")?;
        let page = self.pagination.range(self.total());
        if rect.bottom > page.len()
            || rect.right > columns.len()
            || height.checked_mul(width).is_none_or(|cells| cells > 10_000)
        {
            return Err("Edit range exceeds the current page, columns or 10,000 cells".into());
        }
        let mut bytes = 0usize;
        (rect.top..rect.bottom)
            .map(|ordinal| {
                let source = self.source(page.start + ordinal);
                let mut values = std::collections::BTreeMap::new();
                for column in &columns[rect.left..rect.right] {
                    let value = self.data.text(source, column.id);
                    bytes = bytes.checked_add(value.len()).ok_or("Edit size overflow")?;
                    if bytes > 1024 * 1024 {
                        return Err("Edit input exceeds 1 MiB".into());
                    }
                    values.insert(column.id, value);
                }
                Ok(erp_grid_core::edit_planner::IndexedRow {
                    ordinal,
                    id: self.data.row_id(source),
                    values,
                })
            })
            .collect()
    }
    fn paste(&mut self, anchor: Cell, text: &str) -> Result<(), String> {
        let cells = parse_tsv(text).map_err(String::from)?;
        let columns: Vec<_> = self.columns.iter().filter(|c| c.visible).cloned().collect();
        let rect = CellRect {
            top: anchor.row,
            left: anchor.column,
            bottom: anchor
                .row
                .checked_add(cells.len())
                .ok_or("Paste row overflow")?,
            right: anchor
                .column
                .checked_add(cells[0].len())
                .ok_or("Paste column overflow")?,
        };
        let rows = self.planner_rows(rect, &columns)?;
        let changes = erp_grid_core::edit_planner::plan_paste(anchor, text, &columns, &rows)?;
        self.pending_preview = None;
        self.pending = (!changes.is_empty()).then_some(changes);
        self.pending_error = None;
        Ok(())
    }
    fn fill(&mut self, intent: FillIntent) -> Result<(), String> {
        let columns: Vec<_> = self.columns.iter().filter(|c| c.visible).cloned().collect();
        let source_rows = self.planner_rows(intent.source, &columns)?;
        let target_rows = self.planner_rows(intent.target, &columns)?;
        let changes = erp_grid_core::edit_planner::plan_fill(
            intent.source,
            &source_rows,
            intent.target,
            &target_rows,
            &columns,
        )?;
        self.pending_preview = None;
        self.pending = (!changes.is_empty()).then_some(changes);
        self.pending_error = None;
        Ok(())
    }
    fn row_action(&mut self, action: &str, targets: Vec<RowId>) {
        if targets.is_empty() {
            return;
        }
        let after = match action {
            "ready" => "Ready",
            "hold" => "Hold",
            _ => {
                self.notice = Some("Unknown row action".into());
                return;
            }
        };
        self.pending_preview = None;
        self.pending = Some(
            targets
                .into_iter()
                .map(|row| CellEdit {
                    row,
                    column: ColumnId(4),
                    before: self
                        .data
                        .edits
                        .value(row, ColumnId(4), || self.data.base(row, ColumnId(4))),
                    after: after.into(),
                })
                .collect(),
        );
        self.pending_error = None;
    }
    fn visible_indices(&self) -> Vec<usize> {
        self.pagination
            .range(self.total())
            .map(|index| self.source(index))
            .collect()
    }
    fn scope_indices(&self, scope: usize) -> Vec<usize> {
        match scope {
            1 => self
                .state
                .row_selection
                .checked()
                .iter()
                .filter_map(|id| self.data.index_for_id(*id))
                .collect(),
            2 => self
                .result
                .clone()
                .unwrap_or_else(|| (0..self.data.rows).collect()),
            _ => self.visible_indices(),
        }
    }
    pub fn take_detail_events(&mut self) -> Vec<(RowId, String)> {
        self.inline
            .iter_mut()
            .filter_map(|(key, view)| view.event.take().map(|(_, message)| (*key, message)))
            .collect()
    }
    fn toggle_detail(&mut self, key: RowId) {
        if !self.inline_details_enabled {
            return;
        }
        if self.expanded.contains(&key) {
            if self.inline.get(&key).is_some_and(|view| view.dirty()) {
                self.notice = Some("Finish or cancel the detail draft before collapsing".into());
            } else {
                self.expanded.remove(&key);
            }
        } else if self.expanded.len() >= 8 {
            self.notice = Some("Up to 8 open details; collapse a row first".into());
        } else {
            if !self.inline.contains_key(&key) {
                // Only collapsed clean views are evicted; applied values live in shared storage.
                while self.inline.len() >= 8 {
                    let old = self
                        .inline
                        .keys()
                        .copied()
                        .find(|id| !self.expanded.contains(id));
                    if let Some(old) = old {
                        self.inline.remove(&old);
                    } else {
                        break;
                    }
                }
                let mut view =
                    TableView::new(&format!("{}:inline:{}", self.scope, key.0), Kind::Goods);
                view.data.edits = self.detail_edits.clone();
                view.bind_parent(Some(key), Vec::new());
                self.inline.insert(key, Box::new(view));
            }
            self.expanded.insert(key);
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.tick(ui.ctx());
        self.advance_find(ui.ctx());
        self.state.bind_view(self.generation);
        let busy = self.busy();
        let dirty = self.dirty();
        self.state.quick_search_open = true;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!dirty, egui::Button::new("Reset search"))
                .clicked()
            {
                self.state.quick_search.clear();
                self.start_query();
            }
            if ui.button("Clear checked").clicked() {
                self.state.row_selection.clear_checked();
            }
            if ui
                .add_enabled(
                    !busy && !dirty && self.data.edits.can_undo(),
                    egui::Button::new("Undo"),
                )
                .clicked()
            {
                self.data.edits.undo();
                self.start_query();
            }
            if ui
                .add_enabled(
                    !busy && !dirty && self.data.edits.can_redo(),
                    egui::Button::new("Redo"),
                )
                .clicked()
            {
                self.data.edits.redo();
                self.start_query();
            }
            if ui
                .add_enabled(!busy && !dirty, egui::Button::new("Export"))
                .clicked()
            {
                self.export_open = true;
            }
            if self.named_views.is_some()
                && ui
                    .add_enabled(!busy && !dirty, egui::Button::new("Views"))
                    .clicked()
            {
                self.named_views_open = true;
            }
            if ui
                .add_enabled(!busy && !dirty, egui::Button::new("Find"))
                .clicked()
            {
                self.find_open = true;
            }
            if busy {
                ui.spinner();
            }
            if let Some(message) = self.notice.as_ref().or(self.state.message.as_ref()) {
                ui.colored_label(egui::Color32::YELLOW, "!")
                    .on_hover_text(message);
            }
        });
        self.find_ui(ui);
        if self.named_views_open {
            let current = self.preferences();
            let mut applied = None;
            egui::Window::new("Views")
                .id(egui::Id::new((&self.scope, "named-views")))
                .open(&mut self.named_views_open)
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(!busy && !dirty, |ui| {
                        if let Some(views) = &mut self.named_views {
                            applied = views.ui(ui, &current);
                        }
                    });
                });
            if let Some(preferences) = applied {
                match serde_json::to_string(&preferences)
                    .map_err(|error| error.to_string())
                    .and_then(|text| self.restore(&text))
                {
                    Ok(()) => self.named_views_open = false,
                    Err(error) => self.notice = Some(error),
                }
            }
        }
        let available = ui.available_rect_before_wrap();
        let footer = egui::Rect::from_min_max(
            egui::pos2(
                available.left(),
                (available.bottom() - 28.0).max(available.top()),
            ),
            available.max,
        );
        let total = self.total();
        let pager_id = egui::Id::new((&self.scope, "pager"));
        let pager_width = ui
            .ctx()
            .data_mut(|data| data.get_temp::<f32>(pager_id.with("width")))
            .unwrap_or(400.0)
            .min(footer.width());
        let pager_rect = egui::Rect::from_min_max(
            egui::pos2(footer.right() - pager_width, footer.top()),
            footer.max,
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(pager_rect), |ui| {
            ui.set_clip_rect(pager_rect.intersect(ui.clip_rect()));
            if show_pagination(ui, pager_id, &mut self.pagination, total, !busy && !dirty) {
                self.generation += 1;
                self.state.rebind_page(self.generation, true);
            }
        });
        self.state.bind_view(self.generation);
        let range = self.pagination.range(total);
        self.state.row_number_start = range.start as u64;
        self.state.read_only = busy || self.pending.is_some() || self.data.kind == Kind::Logs;
        self.state.fill_enabled = !self.state.read_only;
        let data = &self.data;
        let result = &self.result;
        let source = |index: usize| {
            result
                .as_ref()
                .map_or(range.start + index, |rows| rows[range.start + index])
        };
        let row_id = |index| data.row_id(source(index));
        let grid =
            egui::Rect::from_min_max(available.min, egui::pos2(available.right(), footer.top()));
        let expanded: Vec<_> = range
            .clone()
            .enumerate()
            .filter_map(|(view, _)| {
                let key = row_id(view);
                (self.inline_details_enabled && self.expanded.contains(&key)).then_some((view, key))
            })
            .collect();
        let inline = &mut self.inline;
        let details_enabled = self.inline_details_enabled && data.kind == Kind::Products;
        let badge_color = ui.visuals().widgets.inactive.bg_fill;
        let present = |cell: erp_ui_egui::CellContext<'_>| {
            use erp_ui_egui::CellPresentation;
            match cell.column.0 {
                4 if data.kind != Kind::Logs => CellPresentation::Badge {
                    label: cell.raw.into(),
                    color: badge_color,
                },
                7 if data.kind == Kind::Products => CellPresentation::Text(
                    RowId::from_uuid(cell.raw)
                        .filter(|id| (100_000..100_020).contains(&id.0))
                        .map_or_else(
                            || cell.raw.into(),
                            |id| format!("SKU option {}", id.0 - 100_000 + 1),
                        ),
                ),
                8 if data.kind == Kind::Products => CellPresentation::Action {
                    id: "open-detail".into(),
                    label: "Open".into(),
                    enabled: !busy,
                },
                _ => CellPresentation::Default,
            }
        };
        let output = ui
            .scope_builder(egui::UiBuilder::new().max_rect(grid), |ui| {
                DataGrid {
                    id: egui::Id::new(&self.scope),
                    columns: &mut self.columns,
                    rows: range.len(),
                    state: &mut self.state,
                    row_id: Some(&row_id),
                }
                .show_with_options(
                    ui,
                    |row, col| data.text(source(row), col),
                    erp_ui_egui::GridOptions {
                        details_enabled,
                        expanded: &expanded,
                        present: Some(&present),
                    },
                    |ui, key| {
                        if let Some(view) = inline.get_mut(&key) {
                            view.ui(ui);
                        }
                    },
                )
            })
            .inner;
        // Local bounded catalogue demonstrates the same request/reply contract as a remote host.
        if let Some(request) = self.state.take_entity_search() {
            let needle = request.search.to_lowercase();
            let options = (0..20)
                .map(|index| erp_ui_egui::EntityOption {
                    id: RowId(100_000 + index),
                    label: format!("SKU option {}", index + 1),
                })
                .filter(|option| option.label.to_lowercase().contains(&needle))
                .collect();
            self.state.resolve_entity_options(&request, Ok(options));
            ui.ctx().request_repaint();
        }
        if let Some(action) = &output.cell_action {
            self.state.row_selection.active = Some(action.row);
            self.notice = Some(format!("Details #{}", action.row.0));
        }
        if let Some(key) = output.toggle_detail {
            self.toggle_detail(key);
            ui.ctx().request_repaint();
        }
        // Draw statistics after selection handling so click/drag changes appear immediately.
        let range = self.pagination.range(self.total());
        let data = &self.data;
        let result = &self.result;
        self.selected_summary.update(
            self.generation,
            data.edits.revision(),
            &self.state.selection,
            &self.columns,
            range.len(),
            |row, col| {
                let index = range.start + row;
                data.text(result.as_ref().map_or(index, |rows| rows[index]), col)
            },
        );
        let stats_rect = egui::Rect::from_min_max(
            footer.min,
            egui::pos2(
                (pager_rect.left() - 8.0).max(footer.left()),
                footer.bottom(),
            ),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(stats_rect), |ui| {
            ui.set_clip_rect(stats_rect.intersect(ui.clip_rect()));
            ui.horizontal(|ui| {
                let load = if self.busy() { "Loading…".into() } else { self.load_seconds.map_or_else(|| "Local process —".into(), |s| format!("Local process {:.1} ms · {:.0}k/s", s * 1000.0, self.data.rows as f64 / s.max(0.000001) / 1000.0)) };
                ui.label(format!("Checked {} · {load}", self.state.row_selection.checked().len()))
                    .on_hover_text(self.load_seconds.map_or_else(|| "Local query is pending".into(), |s| format!("Actual local filter/sort processing time, excluding time between frames or while hidden: {:.1} ms for {} source rows; {:.0} rows/s. No network request.", s * 1000.0, self.data.rows, self.data.rows as f64 / s.max(0.000001))));
                if !self.busy() && !self.selected_summary.label.is_empty() {
                    ui.label(&self.selected_summary.label).on_hover_text("Selected numeric cells only; integer and amount values are added. Checkbox selection is separate. Maximum 100,000 cells.");
                }
                ui.menu_button("∑", |ui| {
                    for (index, label) in ["Page", "Checked", "All matches"].iter().enumerate() {
                        ui.selectable_value(&mut self.summary_scope, index, *label);
                    }
                    if self.summary_scope == 2 { ui.label(self.summary.label()); } else {
                        let mut summary = Summary::default();
                        for row in self.scope_indices(self.summary_scope) { summary.add(&self.data, row); }
                        ui.label(summary.label());
                    }
                }).response.on_hover_text("Row summary by page, checked rows or all matches");
            });
        });
        if output.quick_search_changed || output.sorts_changed {
            self.query = None;
            self.deadline = Some(ui.input(|i| i.time) + 0.25);
            self.valid = false;
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        if !busy {
            if !output.edits.is_empty() || output.edit_navigation.is_some() {
                ui.ctx().request_repaint();
            }
            self.submit_editor(output.edits, output.edit_navigation, output.edit_ticket);
            if let Some((anchor, text)) = output.paste
                && let Err(error) = self.paste(anchor, &text)
            {
                self.notice = Some(error);
            }
            if let Some(intent) = output.fill
                && let Err(error) = self.fill(intent)
            {
                self.notice = Some(error);
            }
            if let Some((action, targets)) = output.row_action {
                self.row_action(&action, targets);
            }
            if output.undo && self.data.edits.undo() {
                self.start_query();
            }
            if output.redo && self.data.edits.redo() {
                self.start_query();
            }
        }
        self.dialogs(ui);
    }
    fn dialogs(&mut self, ui: &mut egui::Ui) {
        let mut event = None;
        if let Some(changes) = &self.pending {
            if self.pending_preview.is_none() {
                match erp_ui_egui::ChangePreview::new(
                    egui::Id::new((&self.scope, "change-preview")),
                    changes.clone(),
                    self.columns
                        .iter()
                        .map(|column| (column.id, column.title.clone())),
                ) {
                    Ok(preview) => self.pending_preview = Some(preview),
                    Err(error) => self.pending_error = Some(error),
                }
            }
            let mut open = true;
            egui::Window::new("Review changes")
                .id(egui::Id::new((&self.scope, "paste-preview")))
                .default_size(egui::vec2(900.0, 300.0))
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    if let Some(error) = &self.pending_error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    if let Some(preview) = self.pending_preview.as_mut() {
                        event = preview.ui(ui);
                    }
                });
            if !open {
                event = Some(erp_ui_egui::ChangePreviewEvent::Cancel);
            }
        } else {
            self.pending_preview = None;
        }
        match event {
            Some(erp_ui_egui::ChangePreviewEvent::Cancel) => {
                self.pending = None;
                self.pending_preview = None;
                self.pending_error = None;
            }
            Some(erp_ui_egui::ChangePreviewEvent::Confirm(changes)) => {
                self.apply(changes);
            }
            None => {}
        }
        let mut export_now = false;
        egui::Window::new("Export").id(egui::Id::new((&self.scope,"export"))).open(&mut self.export_open).show(ui.ctx(),|ui|{
            for (scope,label) in ["Current page","Checked rows","All matches"].iter().enumerate(){ui.radio_value(&mut self.export_scope,scope,*label);}
            ui.horizontal(|ui|{ui.selectable_value(&mut self.export_format,Format::Csv,"CSV");ui.selectable_value(&mut self.export_format,Format::Xlsx,"Excel");ui.selectable_value(&mut self.export_format,Format::Html,"Print HTML").on_hover_text("Save a printable document, then open it in a browser to print or save as PDF");});
            if ui.button("Download").on_hover_text("Visible columns; up to 100,000 cells. Desktop saves a new file in Downloads.").clicked(){export_now=true;}
        });
        if export_now {
            let columns: Vec<_> = self.columns.iter().filter(|c| c.visible).cloned().collect();
            let count = match self.export_scope {
                1 => self.state.row_selection.checked().len(),
                2 => self.total(),
                _ => self.pagination.range(self.total()).len(),
            };
            if count.saturating_mul(columns.len()) > 100_000 {
                self.notice = Some(
                    "Export limit: 100,000 cells; narrow the query or choose fewer columns".into(),
                );
            } else {
                let mut rows = Vec::new();
                let mut bytes = 0usize;
                for row in self.scope_indices(self.export_scope) {
                    let values: Vec<_> = columns
                        .iter()
                        .map(|column| self.data.text(row, column.id))
                        .collect();
                    bytes += values.iter().map(String::len).sum::<usize>();
                    if bytes > 8_388_608 {
                        self.notice =
                            Some("Export limit: 8 MiB of cell text; narrow the scope".into());
                        return;
                    }
                    rows.push(values);
                }
                match export::encode(&columns, &rows, self.export_format) {
                    Ok(bytes) => {
                        self.delivery = Some(Delivery::start(bytes, self.export_format));
                        self.export_open = false;
                    }
                    Err(error) => self.notice = Some(error),
                }
            }
        }
    }
    /// Export only the current applied page, never drafts or an in-flight query.
    pub fn workbook_page(&self, name: &str) -> Result<export::WorkbookSheet, String> {
        if self.busy() || self.dirty() {
            return Err("Finish loading and editing before exporting".into());
        }
        let columns: Vec<_> = self.columns.iter().filter(|c| c.visible).cloned().collect();
        let indices = self.visible_indices();
        if indices.len().saturating_mul(columns.len()) > 100_000 {
            return Err("Export limit: 100,000 cells".into());
        }
        let mut rows = Vec::with_capacity(indices.len());
        let mut bytes = 0usize;
        for row in indices {
            let values: Vec<_> = columns.iter().map(|c| self.data.text(row, c.id)).collect();
            bytes = bytes.saturating_add(values.iter().map(String::len).sum::<usize>());
            if bytes > 8_388_608 {
                return Err("Export limit: 8 MiB of cell text".into());
            }
            rows.push(values);
        }
        Ok(export::WorkbookSheet {
            name: name.into(),
            columns,
            rows,
        })
    }

    pub fn preferences(&self) -> ViewPreferences {
        ViewPreferences {
            version: 1,
            scope: self.scope.clone(),
            columns: self
                .columns
                .iter()
                .map(|c| ColumnPreference {
                    id: c.id,
                    width: c.width(),
                    visible: c.visible,
                    pinned: c.pinned,
                    pinned_right: c.pinned_right,
                })
                .collect(),
            filters: self
                .state
                .quick_search
                .iter()
                .map(|(id, draft)| (*id, draft.clone()))
                .collect(),
            sorts: self.state.sorts.clone(),
            page_size: self.pagination.page_size(),
            quick_search_open: self.state.quick_search_open,
            search: None,
            expression: None,
        }
    }
    pub fn restore(&mut self, text: &str) -> Result<(), String> {
        if text.len() > 262_144 {
            return Err("View settings are too large".into());
        }
        let preferences: ViewPreferences = serde_json::from_str(text).map_err(|e| e.to_string())?;
        preferences
            .apply_columns(&self.scope, &mut self.columns)
            .map_err(String::from)?;
        self.pagination.set_page_size(preferences.page_size);
        self.state.quick_search_open = preferences.quick_search_open;
        self.state.quick_search = preferences
            .filters
            .into_iter()
            .filter(|(id, draft)| {
                self.columns
                    .iter()
                    .find(|c| c.id == *id)
                    .is_some_and(|column| draft.compile(&column.filter).is_ok())
            })
            .collect();
        self.state.sorts = preferences
            .sorts
            .into_iter()
            .filter(|rule| self.columns.iter().any(|c| c.id == rule.column))
            .take(10)
            .collect();
        self.start_query();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn finish(view: &mut TableView) {
        let ctx = egui::Context::default();
        for _ in 0..1000 {
            view.tick(&ctx);
            if view.query.is_none() {
                return;
            }
        }
        panic!("Query did not converge");
    }
    #[test]
    fn local_query_time_accumulates_only_measured_processing_steps() {
        let ctx = egui::Context::default();
        let mut view = TableView::new("timing", Kind::Products);
        view.set_rows(1);
        view.query_processing_seconds = 0.25;
        view.tick(&ctx);
        assert!(view.query.is_none());
        let measured = view.load_seconds.unwrap();
        assert!(measured >= 0.25);

        view.start_query();
        assert_eq!(view.query_processing_seconds, 0.0);
        assert!(view.load_seconds.is_none());
    }
    #[test]
    fn cross_page_checks_survive_refresh_sort_and_page_size_but_not_filter_scope() {
        let mut view = TableView::new("test", Kind::Products);
        view.set_rows(300);
        finish(&mut view);
        for id in [1, 2, 3] {
            view.state.row_selection.toggle(RowId(id));
        }
        view.pagination.set_page(1, 300);
        view.generation += 1;
        view.state.rebind_page(view.generation, true);
        for id in [101, 102] {
            view.state.row_selection.toggle(RowId(id));
        }
        view.state.focus = Some(Cell { row: 1, column: 1 });
        view.state.row_selection.active = Some(RowId(102));
        view.apply(vec![CellEdit {
            row: RowId(102),
            column: ColumnId(1),
            before: "Product 102".into(),
            after: "Changed".into(),
        }]);
        finish(&mut view);
        assert_eq!(view.pagination.page(), 1);
        assert_eq!(view.state.focus, Some(Cell { row: 1, column: 1 }));
        assert_eq!(view.state.row_selection.active, Some(RowId(102)));
        assert_eq!(view.scope_indices(1), [0, 1, 2, 100, 101]);
        view.state.sorts = vec![SortRule {
            column: ColumnId(2),
            descending: true,
        }];
        view.start_query();
        finish(&mut view);
        assert_eq!(view.state.row_selection.checked().len(), 5);
        view.pagination.set_page_size(20);
        view.generation += 1;
        view.state.rebind_page(view.generation, true);
        assert_eq!(view.state.row_selection.checked().len(), 5);
        // Column order is presentation, not a new filter scope.
        view.state
            .quick_search
            .insert(ColumnId(1), FilterDraft::Text("Product".into()));
        view.state
            .quick_search
            .insert(ColumnId(0), FilterDraft::Text("SKU".into()));
        view.start_query();
        finish(&mut view);
        assert!(view.state.row_selection.checked().is_empty());
        assert!(view.notice.as_deref().unwrap().contains("scope changed"));
        view.state.row_selection.toggle(RowId(1));
        view.columns.swap(0, 1);
        view.start_query();
        finish(&mut view);
        assert_eq!(view.state.row_selection.targets(), [RowId(1)]);
    }
    #[test]
    fn filter_sort_then_page_use_full_data_and_edits_requery_values() {
        let mut view = TableView::new("test", Kind::Products);
        view.set_rows(30);
        view.state.sorts = vec![SortRule {
            column: ColumnId(2),
            descending: true,
        }];
        view.pagination.set_page_size(10);
        view.start_query();
        finish(&mut view);
        assert_eq!(view.visible_indices()[0], 29);
        view.pagination.set_page(2, 30);
        assert_eq!(view.visible_indices().len(), 10);
        view.apply(vec![CellEdit {
            row: RowId(1),
            column: ColumnId(2),
            before: "0".into(),
            after: "9999".into(),
        }]);
        finish(&mut view);
        assert_eq!(view.pagination.page(), 2);
        assert_eq!(view.result.as_ref().unwrap()[0], 0);
        assert_eq!(view.visible_indices()[0], 10);
        view.state.quick_search.insert(
            ColumnId(3),
            FilterDraft::Money {
                min: "12.02".into(),
                max: "12.02".into(),
            },
        );
        view.start_query();
        finish(&mut view);
        assert_eq!(view.result.as_deref(), Some([2].as_slice()));
        assert_eq!(view.summary.count, 1);
        assert_eq!(view.summary.amount, 1202);
    }
    #[test]
    fn paste_preview_uses_visible_ids_and_is_atomic_and_undoable() {
        let mut view = TableView::new("test", Kind::Products);
        view.set_rows(10);
        finish(&mut view);
        view.columns[0].visible = false;
        assert!(
            view.paste(Cell { row: 0, column: 0 }, "Changed\tinvalid")
                .is_err()
        );
        assert!(view.pending.is_none());
        view.paste(Cell { row: 0, column: 0 }, "Changed\t42\nOther\t43")
            .unwrap();
        assert_eq!(view.data.text(0, ColumnId(1)), "Product 1");
        let changes = view.pending.clone().unwrap();
        view.apply(changes);
        finish(&mut view);
        assert_eq!(view.data.text(0, ColumnId(1)), "Changed");
        assert_eq!(view.data.text(1, ColumnId(2)), "43");
        assert!(view.data.edits.undo());
        assert_eq!(view.data.text(0, ColumnId(1)), "Product 1");
        assert!(view.data.edits.redo());
        assert_eq!(view.data.text(0, ColumnId(2)), "42");
    }
    #[test]
    fn fill_preview_maps_filtered_sorted_rows_without_automatic_changes() {
        let mut view = TableView::new("fill", Kind::Products);
        view.set_rows(40);
        finish(&mut view);
        view.state
            .quick_search
            .insert(ColumnId(1), FilterDraft::Text("Product".into()));
        view.state.sorts = vec![SortRule {
            column: ColumnId(2),
            descending: true,
        }];
        view.start_query();
        finish(&mut view);
        let range = view.pagination.range(view.total());
        let source_index = view.source(range.start);
        let first_target_index = view.source(range.start + 1);
        let second_target_index = view.source(range.start + 2);
        let source_value = view.data.text(source_index, ColumnId(1));
        let first_before = view.data.text(first_target_index, ColumnId(1));
        let second_before = view.data.text(second_target_index, ColumnId(1));

        view.fill(FillIntent {
            source: CellRect {
                top: 0,
                left: 1,
                bottom: 1,
                right: 2,
            },
            target: CellRect {
                top: 1,
                left: 1,
                bottom: 3,
                right: 2,
            },
        })
        .unwrap();

        assert_eq!(
            view.data.text(first_target_index, ColumnId(1)),
            first_before
        );
        assert_eq!(
            view.data.text(second_target_index, ColumnId(1)),
            second_before
        );
        let changes = view.pending.clone().unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].row, view.data.row_id(first_target_index));
        assert_eq!(changes[1].row, view.data.row_id(second_target_index));
        assert_eq!(changes[0].after, source_value);
        view.apply(changes);
        finish(&mut view);
        assert_eq!(
            view.data.text(first_target_index, ColumnId(1)),
            source_value
        );
        assert!(view.data.edits.undo());
        assert_eq!(
            view.data.text(first_target_index, ColumnId(1)),
            first_before
        );
        assert_eq!(
            view.data.text(second_target_index, ColumnId(1)),
            second_before
        );
    }

    #[test]
    fn fill_preview_infers_numeric_series_vertically_and_horizontally() {
        let mut vertical = TableView::new("fill-series-vertical", Kind::Products);
        vertical.set_rows(4);
        finish(&mut vertical);
        assert!(vertical.apply(vec![
            CellEdit {
                row: RowId(1),
                column: ColumnId(2),
                before: "0".into(),
                after: "10".into(),
            },
            CellEdit {
                row: RowId(2),
                column: ColumnId(2),
                before: "17".into(),
                after: "12".into(),
            },
        ]));
        finish(&mut vertical);
        vertical
            .fill(FillIntent {
                source: CellRect {
                    top: 0,
                    left: 2,
                    bottom: 2,
                    right: 3,
                },
                target: CellRect {
                    top: 2,
                    left: 2,
                    bottom: 4,
                    right: 3,
                },
            })
            .unwrap();
        let changes = vertical.pending.as_ref().unwrap();
        assert_eq!(changes[0].after, "14");
        assert_eq!(changes[1].after, "16");
        assert_eq!(vertical.data.text(2, ColumnId(2)), "34");

        let mut horizontal = TableView::new("fill-series-horizontal", Kind::Products);
        horizontal.set_rows(1);
        finish(&mut horizontal);
        for index in 1..=4 {
            horizontal.columns[index].edit_kind = EditKind::Integer;
            horizontal.columns[index].value_kind = ValueKind::Integer;
        }
        assert!(horizontal.apply(vec![
            CellEdit {
                row: RowId(1),
                column: ColumnId(1),
                before: "Product 1".into(),
                after: "10".into(),
            },
            CellEdit {
                row: RowId(1),
                column: ColumnId(2),
                before: "0".into(),
                after: "12".into(),
            },
        ]));
        finish(&mut horizontal);
        horizontal
            .fill(FillIntent {
                source: CellRect {
                    top: 0,
                    left: 1,
                    bottom: 1,
                    right: 3,
                },
                target: CellRect {
                    top: 0,
                    left: 3,
                    bottom: 1,
                    right: 5,
                },
            })
            .unwrap();
        let changes = horizontal.pending.as_ref().unwrap();
        assert_eq!(changes[0].after, "14");
        assert_eq!(changes[1].after, "16");
        assert_eq!(horizontal.data.text(0, ColumnId(3)), "10.00");
    }
    #[test]
    fn parent_scopes_isolate_row_ids_checks_edits_and_history() {
        let mut parent = TableView::new("main", Kind::Products);
        parent.state.row_selection.toggle(RowId(10));
        let mut child = TableView::new("goods", Kind::Goods);
        child.bind_parent(Some(RowId(10)), Vec::new());
        finish(&mut child);
        let first = child.data.row_id(0);
        assert_eq!(first, RowId(10001));
        child.state.row_selection.toggle(first);
        child.apply(vec![CellEdit {
            row: first,
            column: ColumnId(1),
            before: "Item 1".into(),
            after: "Saved child".into(),
        }]);
        finish(&mut child);
        child.bind_parent(Some(RowId(11)), Vec::new());
        finish(&mut child);
        child.state.bind_view(child.generation);
        assert!(!child.data.edits.can_undo());
        assert!(child.state.row_selection.checked().is_empty());
        assert_eq!(child.data.text(0, ColumnId(1)), "Item 1");
        assert_eq!(parent.state.row_selection.targets(), [RowId(10)]);
        child.bind_parent(Some(RowId(10)), Vec::new());
        finish(&mut child);
        assert_eq!(child.data.text(0, ColumnId(1)), "Saved child");
    }
    #[test]
    fn preferences_round_trip_and_reject_scope_and_corruption() {
        let mut view = TableView::new("merchant:one:products", Kind::Products);
        view.columns[2].visible = false;
        view.columns[1].pinned = true;
        view.columns[1].resize(286.0);
        view.pagination.set_page_size(20);
        view.state.sorts.push(SortRule {
            column: ColumnId(3),
            descending: true,
        });
        view.state
            .quick_search
            .insert(ColumnId(1), FilterDraft::Text("hello".into()));
        let text = serde_json::to_string(&view.preferences()).unwrap();
        let mut restored = TableView::new("merchant:one:products", Kind::Products);
        restored.restore(&text).unwrap();
        assert_eq!(restored.columns[0].id, ColumnId(1));
        assert_eq!(restored.columns[0].width(), 286.0);
        assert!(
            !restored
                .columns
                .iter()
                .find(|c| c.id == ColumnId(2))
                .unwrap()
                .visible
        );
        assert_eq!(restored.pagination.page_size(), 20);
        assert_eq!(restored.state.sorts, view.state.sorts);
        assert_eq!(restored.state.quick_search, view.state.quick_search);
        assert!(
            TableView::new("merchant:two:products", Kind::Products)
                .restore(&text)
                .is_err()
        );
        assert!(restored.restore("{").is_err());
    }
    #[test]
    fn checked_export_and_summary_include_later_pages() {
        let mut view = TableView::new("test", Kind::Products);
        view.set_rows(2000);
        finish(&mut view);
        view.state.row_selection.toggle(RowId(1501));
        assert_eq!(view.scope_indices(1), [1500]);
    }
    #[test]
    fn workbook_page_respects_page_visible_order_and_loading() {
        let mut view = TableView::new("test:workbook", Kind::Products);
        view.set_rows(250);
        assert!(view.workbook_page("Products").is_err());
        finish(&mut view);
        view.pagination.set_page(1, view.total());
        view.columns.swap(0, 1);
        view.columns[2].visible = false;
        let sheet = view.workbook_page("Products").unwrap();
        assert_eq!(sheet.rows.len(), 100);
        assert_eq!(sheet.columns[0].id, ColumnId(1));
        assert_eq!(sheet.rows[0][0], view.data.text(100, ColumnId(1)));
        assert_eq!(sheet.rows[0][1], view.data.text(100, ColumnId(0)));
        assert!(sheet.columns.iter().all(|column| column.id != ColumnId(2)));
    }
    #[test]
    fn debounced_query_waits_for_ime_and_new_query_cancels_old_result() {
        let ctx = egui::Context::default();
        let mut view = TableView::new("test", Kind::Products);
        view.set_rows(10);
        view.query = None;
        view.deadline = Some(1.0);
        view.valid = false;
        fn frame(ctx: &egui::Context, view: &mut TableView, time: f64, events: Vec<egui::Event>) {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |_| view.tick(ctx),
            );
            out.textures_delta.clear();
        }
        frame(
            &ctx,
            &mut view,
            2.0,
            vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "x".into(),
                active_range_chars: None,
            })],
        );
        assert!(view.deadline.is_some());
        frame(
            &ctx,
            &mut view,
            3.0,
            vec![egui::Event::Ime(egui::ImeEvent::Commit("x".into()))],
        );
        assert!(view.deadline.is_none());
        view.set_rows(10_000);
        view.state
            .quick_search
            .insert(ColumnId(0), FilterDraft::Text("00000001".into()));
        view.start_query();
        view.tick(&ctx);
        assert!(view.query.is_some());
        view.state
            .quick_search
            .insert(ColumnId(0), FilterDraft::Text("00000042".into()));
        view.start_query();
        finish(&mut view);
        assert_eq!(view.result.as_deref(), Some([41].as_slice()));
    }
    #[test]
    fn inline_views_share_committed_data_but_keep_drafts_checks_and_history_scope() {
        let mut parent = TableView::new("main", Kind::Products);
        assert!(!parent.inline_details_enabled);
        parent.toggle_detail(RowId(1));
        assert!(parent.expanded.is_empty());
        parent.inline_details_enabled = true;
        parent.toggle_detail(RowId(1));
        let mut bottom = TableView::new("bottom", Kind::Goods);
        bottom.data.edits = parent.detail_edits.clone();
        bottom.bind_parent(Some(RowId(1)), Vec::new());
        let inline = parent.inline.get_mut(&RowId(1)).unwrap();
        inline.apply(vec![CellEdit {
            row: RowId(1001),
            column: ColumnId(1),
            before: "Item 1".into(),
            after: "Shared item".into(),
        }]);
        assert_eq!(bottom.data.text(0, ColumnId(1)), "Shared item");
        bottom.apply(vec![CellEdit {
            row: RowId(1001),
            column: ColumnId(1),
            before: "Shared item".into(),
            after: "Bottom edit".into(),
        }]);
        assert_eq!(inline.data.text(0, ColumnId(1)), "Bottom edit");
        inline.state.row_selection.toggle(RowId(1001));
        assert!(bottom.state.row_selection.checked().is_empty());
        inline.pending = Some(vec![]);
        parent.toggle_detail(RowId(1));
        assert!(parent.expanded.contains(&RowId(1)));
        assert!(parent.dirty());
        parent.inline.get_mut(&RowId(1)).unwrap().discard_draft();
        parent.toggle_detail(RowId(1));
        assert!(!parent.expanded.contains(&RowId(1)));
        for id in 2..=9 {
            parent.toggle_detail(RowId(id));
        }
        parent.toggle_detail(RowId(10));
        assert_eq!(parent.expanded.len(), 8);
        assert_eq!(parent.inline.len(), 8);
        assert!(!parent.expanded.contains(&RowId(10)));
        let other = parent.inline.get_mut(&RowId(2)).unwrap();
        other.apply(vec![CellEdit {
            row: RowId(2001),
            column: ColumnId(1),
            before: "Item 1".into(),
            after: "Other parent".into(),
        }]);
        assert!(
            !bottom.data.edits.undo(),
            "undo must not target a different parent"
        );
        assert_eq!(bottom.data.text(0, ColumnId(1)), "Bottom edit");
    }

    #[test]
    fn footer_is_right_aligned_and_inline_grid_is_visible_without_parent_selection() {
        let ctx = egui::Context::default();
        let mut parent = TableView::new("main", Kind::Products);
        parent.set_rows(100);
        finish(&mut parent);
        assert!(!parent.inline_details_enabled);
        parent.toggle_detail(RowId(1));
        assert!(parent.expanded.is_empty());
        parent.inline_details_enabled = true;
        parent.toggle_detail(RowId(1));
        let mut output = None;
        for _ in 0..4 {
            let mut frame = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 800.0),
                    )),
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| parent.ui(ui));
                },
            );
            frame.textures_delta.clear();
            output = Some(frame);
        }
        let output = output.unwrap();
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text)
                    if shape
                        .clip_rect
                        .contains(text.pos + text.galley.size() / 2.0) =>
                {
                    Some((text.galley.text(), text.pos))
                }
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|(text, _)| *text == "SKU-1-001"),
            "inline child must be visible"
        );
        assert!(
            texts
                .iter()
                .any(|(text, pos)| *text == "100 / page" && pos.x > 800.0 && pos.y > 760.0),
            "main pager must align right: {texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|(text, pos)| text.starts_with("Checked 0") && pos.x < 30.0 && pos.y > 760.0)
        );
        let child_y = texts
            .iter()
            .find(|(text, _)| *text == "SKU-1-001")
            .unwrap()
            .1
            .y
            + 6.0;
        let point = egui::pos2(27.0, child_y);
        for events in [
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        ] {
            let mut frame = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| parent.ui(ui));
                },
            );
            frame.textures_delta.clear();
        }
        assert_eq!(
            parent
                .inline
                .get(&RowId(1))
                .unwrap()
                .state
                .row_selection
                .targets(),
            [RowId(1001)]
        );
        assert!(parent.state.row_selection.targets().is_empty());
    }

    #[test]
    fn shared_main_and_detail_ui_render_independent_grids() {
        let ctx = egui::Context::default();
        let mut parent = TableView::new("main", Kind::Products);
        parent.set_rows(10);
        finish(&mut parent);
        let mut child = TableView::new("child", Kind::Goods);
        child.bind_parent(Some(RowId(1)), Vec::new());
        finish(&mut child);
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::Panel::bottom("test-child")
                    .exact_size(270.0)
                    .show(root, |ui| child.ui(ui));
                egui::CentralPanel::default().show(root, |ui| parent.ui(ui));
            },
        );
        assert!(!out.shapes.is_empty());
        assert_eq!(child.data.parent, Some(RowId(1)));
        assert!(parent.state.row_selection.checked().is_empty());
        out.textures_delta.clear();
    }
}
