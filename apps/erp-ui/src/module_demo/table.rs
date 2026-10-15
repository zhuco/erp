//! Bounded local-data adapter for the module showcase; production hosts supply their own queries.
use eframe::egui;
use erp_grid_core::{Column, ColumnId, Filter, RowId, ValueKind};
use erp_ui_egui::{DataGrid, GridState};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(super) struct Row {
    pub id: RowId,
    pub cells: BTreeMap<ColumnId, String>,
}
impl Row {
    pub fn text(&self, id: ColumnId) -> String {
        self.cells.get(&id).cloned().unwrap_or_default()
    }
}

pub(super) struct Table {
    pub columns: Vec<Column>,
    pub state: GridState,
    pub rows: Vec<Row>,
    indices: Vec<usize>,
    pub page: usize,
    pub page_size: usize,
    generation: u64,
    pub revision: u64,
    bound_revision: u64,
    query_key: String,
    query: crate::workbench::query::QueryAdmission,
    reset_query: bool,
    pub external_query_input: Option<egui::Id>,
    pub error: Option<String>,
    pub locked: bool,
    summary: crate::components::SelectionSummary,
}
impl Table {
    pub fn new(columns: Vec<Column>, rows: Vec<Row>) -> Self {
        let mut state = GridState::default();
        state.metrics = erp_ui_egui::GridMetrics::new(24.0, 28.0, 13.0);
        state.theme = Some(crate::erp_theme::grid());
        state.read_only = true;
        Self {
            indices: (0..rows.len()).collect(),
            rows,
            columns,
            state,
            page: 0,
            page_size: 500,
            generation: 0,
            revision: 0,
            bound_revision: 0,
            query_key: String::new(),
            query: Default::default(),
            reset_query: false,
            external_query_input: None,
            error: None,
            locked: false,
            summary: Default::default(),
        }
    }
    pub fn selected(&self) -> Option<RowId> {
        self.state.row_selection.active
    }
    pub fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn replace(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.indices.clear();
        self.invalidate();
    }
    pub fn ready(&self) -> bool {
        self.query.ready() && self.bound_revision == self.revision && self.error.is_none()
    }
    pub fn reset(&mut self) {
        self.reset_query = true;
        self.state.quick_search.clear();
        self.state.sorts.clear();
        self.page = 0;
        self.invalidate();
    }
    pub fn targets(&self) -> Vec<RowId> {
        if !self.ready() {
            return Vec::new();
        }
        let checked = self.state.row_selection.checked();
        if checked.is_empty() {
            self.selected().into_iter().collect()
        } else {
            checked.iter().copied().collect()
        }
    }
    fn rebuild(&mut self, search: &str, tree: Option<(ColumnId, &str)>) {
        let previous_active = self.state.row_selection.active;
        let preserve = self.bound_revision != self.revision;
        self.error = None;
        let filters: Result<Vec<_>, _> = self
            .columns
            .iter()
            .filter_map(|c| {
                self.state
                    .quick_search
                    .get(&c.id)
                    .map(|draft| draft.compile(&c.filter).map(|f| (c.id, f)))
            })
            .collect();
        let filters = match filters {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e.into());
                self.indices.clear();
                self.bound_revision = self.revision;
                return;
            }
        };
        let search = search.trim().to_lowercase();
        self.indices = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                (search.is_empty()
                    || row
                        .cells
                        .values()
                        .any(|v| v.to_lowercase().contains(&search)))
                    && tree
                        .is_none_or(|(col, value)| row.cells.get(&col).is_some_and(|v| v == value))
                    && filters.iter().all(|(id, filter)| {
                        let value = row.text(*id);
                        match filter {
                            None => true,
                            Some(filter @ (Filter::Text(_) | Filter::Enum(_))) => {
                                filter.matches_text(&value)
                            }
                            Some(filter @ Filter::Money { .. }) => {
                                erp_grid_core::money_minor(&value)
                                    .ok()
                                    .flatten()
                                    .is_some_and(|n| filter.matches_number(n))
                            }
                            Some(filter @ Filter::DateTime { .. }) => {
                                erp_grid_core::DateTimeParts::parse(&value)
                                    .and_then(|p| p.key())
                                    .is_ok_and(|n| filter.matches_number(n))
                            }
                        }
                    })
            })
            .map(|(i, _)| i)
            .collect();
        self.indices.sort_by(|a, b| {
            for sort in &self.state.sorts {
                let kind = self
                    .columns
                    .iter()
                    .find(|c| c.id == sort.column)
                    .map_or(ValueKind::Text, |c| c.value_kind);
                let order = erp_grid_core::compare_values(
                    &self.rows[*a].text(sort.column),
                    &self.rows[*b].text(sort.column),
                    kind,
                    sort.descending,
                );
                if !order.is_eq() {
                    return order;
                }
            }
            self.rows[*a].id.cmp(&self.rows[*b].id)
        });
        self.page = self
            .page
            .min(self.indices.len().saturating_sub(1) / self.page_size);
        self.bound_revision = self.revision;
        self.generation = self.generation.wrapping_add(1);
        self.state.bind_view(self.generation);
        if preserve {
            self.state.row_selection.active =
                previous_active.filter(|id| self.indices.iter().any(|i| self.rows[*i].id == *id));
        }
    }
    pub fn input_changed(&mut self, now: f64, cleared: bool) {
        self.page = 0;
        if cleared {
            self.reset_query = true;
            self.query.reset(now);
        } else {
            self.query.changed(now);
        }
    }

    pub fn prepare_query(
        &mut self,
        ctx: &egui::Context,
        search: &str,
        tree: Option<(ColumnId, &str)>,
    ) {
        let now = ctx.input(|input| input.time);
        let focused = ctx.memory(|memory| memory.focused());
        let owns_ime = focused.is_some()
            && (focused == self.external_query_input || focused == self.state.query_input_focus);
        if !owns_ime {
            self.query.composition(false, now);
        }
        ctx.input(|input| {
            for event in &input.events {
                if owns_ime && let egui::Event::Ime(event) = event {
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => {
                            self.query.composition(!text.is_empty(), now)
                        }
                        egui::ImeEvent::Commit(_) => self.query.composition(false, now),
                        _ => {}
                    }
                }
            }
        });
        let key = format!("{search}|{tree:?}");
        if self.query_key != key {
            self.query_key = key;
            self.query.changed(now);
            self.page = 0;
        }
        if self.reset_query {
            self.reset_query = false;
            self.query.reset(now);
        }
    }

    fn apply_query(&mut self, search: &str, tree: Option<(ColumnId, &str)>, now: f64) {
        if self.query.due(now) || (self.query.ready() && self.bound_revision != self.revision) {
            self.query.applied();
            self.rebuild(search, tree);
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, search: &str, tree: Option<(ColumnId, &str)>) {
        self.state.theme = Some(crate::workbench::theme::grid(ui.visuals()));
        self.prepare_query(ui.ctx(), search, tree);
        if self.query.pending() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
        self.apply_query(search, tree, ui.input(|input| input.time));
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }
        let start = (self.page * self.page_size).min(self.indices.len());
        let end = (start + self.page_size).min(self.indices.len());
        let indices = &self.indices[start..end];
        self.state.row_number_start = start as u64;
        let footer_height = ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.y;
        let height = (ui.available_height() - footer_height).max(70.0);
        let output = ui
            .allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
                ui.style_mut().visuals.widgets.inactive.fg_stroke.color =
                    ui.visuals().weak_text_color();
                DataGrid {
                    id: ui.id().with("data-grid"),
                    columns: &mut self.columns,
                    rows: indices.len(),
                    state: &mut self.state,
                    row_id: Some(&|i| self.rows[indices[i]].id),
                }
                .show(ui, |i, col| self.rows[indices[i]].text(col))
            })
            .inner;
        if output.quick_search_changed {
            self.query.changed(ui.input(|i| i.time));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
            self.page = 0;
        }
        if output.sorts_changed && !self.query.pending() {
            self.query.immediate(ui.input(|i| i.time));
            ui.ctx().request_repaint();
        }
        let start = (self.page * self.page_size).min(self.indices.len());
        let end = (start + self.page_size).min(self.indices.len());
        self.summary.update(
            self.generation,
            self.revision,
            &self.state.selection,
            &self.columns,
            end - start,
            |i, col| self.rows[self.indices[start + i]].text(col),
        );
        let previous = (self.page, self.page_size);
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} 条 · 勾选 {}",
                self.indices.len(),
                self.state.row_selection.checked().len()
            ));
            ui.label(&self.summary.label);
            if !self.ready() {
                ui.label("筛选中");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !self.locked && self.ready() && end < self.indices.len(),
                        egui::Button::new("▶"),
                    )
                    .on_hover_text("下一页")
                    .clicked()
                {
                    self.page += 1;
                }
                ui.label(format!(
                    "{} / {}",
                    self.page + 1,
                    self.indices.len().div_ceil(self.page_size).max(1)
                ));
                if ui
                    .add_enabled(
                        !self.locked && self.ready() && self.page > 0,
                        egui::Button::new("◀"),
                    )
                    .on_hover_text("上一页")
                    .clicked()
                {
                    self.page -= 1;
                }
                ui.add_enabled_ui(!self.locked && self.ready(), |ui| {
                    egui::ComboBox::from_id_salt("page-size")
                        .selected_text(format!("{} 条/页", self.page_size))
                        .show_ui(ui, |ui| {
                            for size in [100, 200, 300, 500, 800, 1000, 2000] {
                                ui.selectable_value(&mut self.page_size, size, size.to_string());
                            }
                        });
                });
            });
        });
        if previous != (self.page, self.page_size) {
            if previous.1 != self.page_size {
                self.page = 0;
            }
            self.generation = self.generation.wrapping_add(1);
            self.state.rebind_page(self.generation, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn module_demo_filter_covers_all_rows_before_paging() {
        let rows = (0..120)
            .map(|i| Row {
                id: RowId(i),
                cells: [(ColumnId(1), if i >= 100 { "match" } else { "other" }.into())]
                    .into_iter()
                    .collect(),
            })
            .collect();
        let mut table = Table::new(vec![Column::new(ColumnId(1), "Name", 100.0)], rows);
        table.rebuild("match", None);
        assert_eq!(table.indices.len(), 20);
        assert_eq!(table.rows[table.indices[0]].id, RowId(100));
        table.state.row_selection.active = Some(RowId(105));
        table.invalidate();
        table.rebuild("match", None);
        assert_eq!(table.selected(), Some(RowId(105)));
        table.rebuild("other", None);
        assert_eq!(table.selected(), None);
    }

    fn query_table() -> Table {
        let mut table = Table::new(
            vec![Column::new(ColumnId(1), "Name", 100.0)],
            vec![
                Row {
                    id: RowId(1),
                    cells: [(ColumnId(1), "one".into())].into_iter().collect(),
                },
                Row {
                    id: RowId(2),
                    cells: [(ColumnId(1), "two".into())].into_iter().collect(),
                },
            ],
        );
        table.external_query_input = Some(egui::Id::new("query-test"));
        table.query_key = format!("|{:?}", None::<(ColumnId, &str)>);
        table.rebuild("", None);
        table.state.row_selection.active = Some(RowId(1));
        table
    }

    fn query_frame(
        table: &mut Table,
        ctx: &egui::Context,
        now: f64,
        events: Vec<egui::Event>,
        search: &str,
    ) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(now),
                events,
                ..Default::default()
            },
            |root| {
                if let Some(id) = table.external_query_input {
                    root.ctx().memory_mut(|memory| memory.request_focus(id));
                }
                table.prepare_query(root.ctx(), search, None);
                table.apply_query(search, None, now);
            },
        );
        output.textures_delta.clear();
    }

    #[test]
    fn module_query_search_change_immediately_blocks_previous_targets() {
        let mut table = query_table();
        let ctx = egui::Context::default();
        assert_eq!(table.targets(), [RowId(1)]);
        table.input_changed(1.0, false);
        assert!(table.targets().is_empty());
        query_frame(&mut table, &ctx, 1.0, vec![], "two");
        query_frame(&mut table, &ctx, 1.249, vec![], "two");
        assert_eq!(table.indices.len(), 2);
        assert!(table.targets().is_empty());
        query_frame(&mut table, &ctx, 1.25, vec![], "two");
        assert!(table.ready());
        assert_eq!(table.indices, [1]);
        assert_eq!(table.selected(), None);
    }

    #[test]
    fn module_query_real_ime_events_block_actions_until_final_text() {
        let mut table = query_table();
        let ctx = egui::Context::default();
        query_frame(
            &mut table,
            &ctx,
            1.0,
            vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "t".into(),
                active_range_chars: None,
            })],
            "t",
        );
        query_frame(&mut table, &ctx, 2.0, vec![], "t");
        assert!(table.targets().is_empty());
        assert_eq!(table.indices.len(), 2);
        query_frame(
            &mut table,
            &ctx,
            2.0,
            vec![egui::Event::Ime(egui::ImeEvent::Commit("two".into()))],
            "two",
        );
        query_frame(&mut table, &ctx, 2.249, vec![], "two");
        assert!(!table.ready());
        query_frame(&mut table, &ctx, 2.25, vec![], "two");
        assert_eq!(table.indices, [1]);
        assert!(table.ready());
    }

    #[test]
    fn module_query_clear_and_reset_apply_without_previous_delay() {
        let mut table = query_table();
        let ctx = egui::Context::default();
        query_frame(&mut table, &ctx, 1.0, vec![], "two");
        table.input_changed(1.01, true);
        query_frame(&mut table, &ctx, 1.01, vec![], "");
        assert_eq!(table.indices.len(), 2);
        assert!(table.ready());
        query_frame(&mut table, &ctx, 2.0, vec![], "two");
        table.reset();
        query_frame(&mut table, &ctx, 2.01, vec![], "");
        assert!(table.ready());
        assert_eq!(table.indices.len(), 2);
    }

    #[test]
    fn module_query_replaced_rows_never_use_stale_indices_while_waiting() {
        let mut table = query_table();
        let ctx = egui::Context::default();
        query_frame(&mut table, &ctx, 1.0, vec![], "two");
        table.replace(Vec::new());
        assert!(table.indices.is_empty());
        assert!(table.targets().is_empty());
        query_frame(&mut table, &ctx, 1.1, vec![], "two");
        assert!(!table.ready());
        query_frame(&mut table, &ctx, 1.25, vec![], "two");
        assert!(table.ready());
        assert!(table.indices.is_empty());
    }

    #[test]
    fn module_query_invalid_money_keeps_draft_and_blocks_actions() {
        let mut table = query_table();
        table.columns[0].filter = erp_grid_core::FilterKind::Money;
        table.state.quick_search.insert(
            ColumnId(1),
            erp_grid_core::FilterDraft::Money {
                min: "9".into(),
                max: "1".into(),
            },
        );
        table.invalidate();
        let ctx = egui::Context::default();
        query_frame(&mut table, &ctx, 1.0, vec![], "");
        assert!(table.error.is_some());
        assert!(table.targets().is_empty());
        assert_eq!(table.bound_revision, table.revision);
        assert_eq!(
            table.state.quick_search.get(&ColumnId(1)),
            Some(&erp_grid_core::FilterDraft::Money {
                min: "9".into(),
                max: "1".into()
            })
        );
    }

    #[test]
    fn module_query_parent_composition_does_not_freeze_child_query() {
        let mut parent = query_table();
        let mut child = query_table();
        let parent_input = egui::Id::new("parent-query");
        let child_input = egui::Id::new("child-query");
        parent.external_query_input = Some(parent_input);
        child.external_query_input = None;
        child.state.query_input_focus = Some(child_input);
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(1.0),
                events: vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                    text: "t".into(),
                    active_range_chars: None,
                })],
                ..Default::default()
            },
            |root| {
                root.ctx()
                    .memory_mut(|memory| memory.request_focus(parent_input));
                parent.prepare_query(root.ctx(), "t", None);
                child.prepare_query(root.ctx(), "", None);
            },
        );
        output.textures_delta.clear();
        assert!(!parent.ready());
        assert!(child.ready());
        assert_eq!(child.targets(), [RowId(1)]);
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(2.0),
                events: vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                    text: "o".into(),
                    active_range_chars: None,
                })],
                ..Default::default()
            },
            |root| {
                root.ctx()
                    .memory_mut(|memory| memory.request_focus(child_input));
                parent.prepare_query(root.ctx(), "t", None);
                child.prepare_query(root.ctx(), "", None);
                parent.apply_query("t", None, 2.0);
            },
        );
        output.textures_delta.clear();
        assert!(!child.ready());
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(2.3),
                ..Default::default()
            },
            |root| {
                root.ctx()
                    .memory_mut(|memory| memory.request_focus(child_input));
                parent.prepare_query(root.ctx(), "t", None);
                parent.apply_query("t", None, 2.3);
                child.prepare_query(root.ctx(), "", None);
            },
        );
        output.textures_delta.clear();
        assert!(parent.ready());
        assert!(!child.ready());
    }
}
