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
    deadline: Option<f64>,
    composing: bool,
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
            deadline: None,
            composing: false,
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
        self.invalidate();
    }
    pub fn ready(&self) -> bool {
        self.deadline.is_none() && self.error.is_none()
    }
    pub fn reset(&mut self) {
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
    pub fn show(&mut self, ui: &mut egui::Ui, search: &str, tree: Option<(ColumnId, &str)>) {
        self.state.theme = Some(crate::workbench::theme::grid(ui.visuals()));
        ui.input(|input| {
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
        });
        let key = format!("{search}|{tree:?}");
        if self.query_key != key {
            self.query_key = key;
            self.deadline = Some(ui.input(|i| i.time) + 0.25);
            self.page = 0;
        }
        if self.deadline.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
        if !self.composing && self.deadline.is_some_and(|t| ui.input(|i| i.time) >= t) {
            self.deadline = None;
            self.rebuild(search, tree);
        }
        if self.bound_revision != self.revision && !self.composing {
            self.rebuild(search, tree);
        }
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
            self.deadline = Some(ui.input(|i| i.time) + 0.25);
            self.page = 0;
        }
        if output.sorts_changed {
            self.rebuild(search, tree);
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
}
