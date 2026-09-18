use eframe::egui;
use erp_grid_core::{
    Column, ColumnId, Pagination, RowId, ValueKind,
    formula::FormulaGraph,
    report::{ReportAccumulator, ReportResult},
};
use erp_ui_egui::{
    DataGrid, GridState,
    report_chart::{self, ChartKind},
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "report_hierarchy.rs"]
pub(crate) mod hierarchy;

const ROWS: usize = 10_000;
const WAREHOUSES: [&str; 4] = ["North", "South", "East", "West"];
const CATEGORIES: [&str; 3] = ["Clothing", "Home", "Accessories"];

struct Record {
    id: RowId,
    group: Vec<String>,
    cells: BTreeMap<ColumnId, String>,
}
struct Job {
    next: usize,
    formula: FormulaGraph,
    report: ReportAccumulator,
    records: Vec<Record>,
}
pub struct ReportView {
    warehouse: usize,
    category: usize,
    by_category: bool,
    formula: String,
    deadline: Option<f64>,
    composing: bool,
    job: Option<Job>,
    result: Option<ReportResult>,
    records: Vec<Record>,
    hierarchy: Vec<erp_grid_core::report::GroupSummary>,
    visible_groups: Vec<usize>,
    expanded_groups: BTreeSet<Vec<String>>,
    columns: Vec<Column>,
    detail_columns: Vec<Column>,
    state: GridState,
    detail_state: GridState,
    pager: Pagination,
    detail_indices: Vec<usize>,
    selected: Option<RowId>,
    group_ids: BTreeMap<Vec<String>, RowId>,
    generation: u64,
    pivot: bool,
    chart: ChartKind,
    error: Option<String>,
    elapsed: Option<f64>,
    processing_seconds: f64,
    delivery: Option<crate::export::Delivery>,
}
impl Default for ReportView {
    fn default() -> Self {
        let mut state = GridState::default();
        state.read_only = true;
        state.filtering_enabled = false;
        state.sorting_enabled = false;
        state.quick_search_open = false;
        let mut detail_state = GridState::default();
        detail_state.read_only = true;
        detail_state.filtering_enabled = false;
        detail_state.sorting_enabled = false;
        detail_state.quick_search_open = false;
        Self {
            warehouse: 0,
            category: 0,
            by_category: false,
            formula: "[2] * [3]".into(),
            deadline: Some(0.0),
            composing: false,
            job: None,
            result: None,
            records: Vec::new(),
            hierarchy: Vec::new(),
            visible_groups: Vec::new(),
            expanded_groups: BTreeSet::new(),
            columns: Vec::new(),
            detail_columns: [
                "SKU",
                "Warehouse",
                "Quantity",
                "Unit cost",
                "Unit price",
                "Category",
                "Value",
                "Margin",
            ]
            .into_iter()
            .enumerate()
            .map(|(i, title)| Column::new(ColumnId(i as u64), title, 130.0))
            .collect(),
            state,
            detail_state,
            pager: Pagination::default(),
            detail_indices: Vec::new(),
            selected: None,
            group_ids: BTreeMap::new(),
            generation: 0,
            pivot: false,
            chart: ChartKind::Bar,
            error: None,
            elapsed: None,
            processing_seconds: 0.0,
            delivery: None,
        }
    }
}
fn money(value: i128) -> String {
    let value_abs = value.unsigned_abs();
    format!(
        "{}{}.{:02}",
        if value < 0 { "-" } else { "" },
        value_abs / 100,
        value_abs % 100
    )
}
fn cents(value: i128) -> i128 {
    value / 100
        + if (value % 100).unsigned_abs() >= 50 {
            value.signum()
        } else {
            0
        }
}
impl ReportView {
    fn invalidate(&mut self, now: f64) {
        self.job = None;
        self.result = None;
        self.records.clear();
        self.hierarchy.clear();
        self.visible_groups.clear();
        self.state.tree_rows.clear();
        self.state.summary_rows.clear();
        self.detail_indices.clear();
        self.selected = None;
        self.deadline = Some(now + 0.25);
        self.error = None;
        self.elapsed = None;
        self.generation += 1;
        self.state.bind_view(self.generation);
        self.detail_state.bind_view(self.generation);
    }
    fn start(&mut self, _now: f64) -> Result<(), String> {
        let formula = FormulaGraph::new(
            &[ColumnId(2), ColumnId(3), ColumnId(4)],
            &[
                (ColumnId(6), self.formula.clone()),
                (ColumnId(7), "[2] * [4] - [6]".into()),
            ],
        )?;
        self.job = Some(Job {
            next: 0,
            formula,
            report: ReportAccumulator::new(4096, 64).map_err(|error| error.to_string())?,
            records: Vec::new(),
        });
        self.processing_seconds = 0.0;
        self.deadline = None;
        Ok(())
    }
    fn advance(&mut self) -> Result<(), String> {
        if self.job.is_none() {
            return Ok(());
        }
        let started = web_time::Instant::now();
        let result = self.advance_chunk();
        self.processing_seconds += started.elapsed().as_secs_f64();
        if result.is_ok() && self.result.is_some() {
            self.elapsed = Some(self.processing_seconds);
        }
        result
    }

    fn advance_chunk(&mut self) -> Result<(), String> {
        let Some(job) = &mut self.job else {
            return Ok(());
        };
        for index in job.next..(job.next + 256).min(ROWS) {
            job.next = index + 1;
            let warehouse = index % 4;
            let category = (index / 4) % 3;
            if (self.warehouse != 0 && self.warehouse - 1 != warehouse)
                || (self.category != 0 && self.category - 1 != category)
            {
                continue;
            }
            let quantity = (index % 50 + 1) as i128;
            let cost = (100 + index % 20_000) as i128;
            let price = cost + 250;
            let calculated = job.formula.evaluate(|column| match column.0 {
                2 => Some(quantity * 10_000),
                3 => Some(cost * 100),
                4 => Some(price * 100),
                _ => None,
            })?;
            let amount = cents(
                *calculated
                    .get(&ColumnId(6))
                    .ok_or("Missing calculated value [6]")?,
            );
            let margin = cents(
                *calculated
                    .get(&ColumnId(7))
                    .ok_or("Missing calculated margin [7]")?,
            );
            let group = if self.by_category {
                vec![WAREHOUSES[warehouse].into(), CATEGORIES[category].into()]
            } else {
                vec![WAREHOUSES[warehouse].into()]
            };
            job.report
                .push(
                    group.clone(),
                    format!("{:02}", (index / 12) % 12 + 1),
                    amount,
                )
                .map_err(|error| error.to_string())?;
            job.records.push(Record {
                id: RowId(index as u128 + 1),
                group,
                cells: BTreeMap::from([
                    (ColumnId(0), format!("SKU-{:06}", index + 1)),
                    (ColumnId(1), WAREHOUSES[warehouse].into()),
                    (ColumnId(2), quantity.to_string()),
                    (ColumnId(3), money(cost)),
                    (ColumnId(4), money(price)),
                    (ColumnId(5), CATEGORIES[category].into()),
                    (ColumnId(6), money(amount)),
                    (ColumnId(7), money(margin)),
                ]),
            });
        }
        if job.next == ROWS
            && let Some(job) = self.job.take()
        {
            self.result = Some(job.report.finish());
            self.records = job.records;
            if let Some(result) = &self.result {
                self.hierarchy = hierarchy::rollup(result)?;
                for group in &self.hierarchy {
                    let id = RowId((1u128 << 127) + self.group_ids.len() as u128 + 1);
                    self.group_ids.entry(group.key.clone()).or_insert(id);
                }
            }
            self.rebuild_group_rows();
            self.rebuild_columns();
            self.update_fixed_total()?;
            self.select_detail(None);
        }
        Ok(())
    }
    fn rebuild_group_rows(&mut self) {
        self.visible_groups = self
            .hierarchy
            .iter()
            .enumerate()
            .filter_map(|(index, group)| {
                (1..group.key.len())
                    .all(|depth| self.expanded_groups.contains(&group.key[..depth]))
                    .then_some(index)
            })
            .collect();
        self.state.tree_rows = self
            .visible_groups
            .iter()
            .map(|index| {
                let group = &self.hierarchy[*index];
                let has_children = self.hierarchy.get(index + 1).is_some_and(|next| {
                    next.key.len() > group.key.len() && next.key.starts_with(&group.key)
                });
                (
                    self.group_ids[&group.key],
                    erp_ui_egui::TreeRowVisual {
                        depth: (group.key.len() - 1) as u8,
                        expanded: self.expanded_groups.contains(&group.key),
                        has_children,
                        loading: false,
                        error: None,
                    },
                )
            })
            .collect();
        self.generation += 1;
        self.state.rebind_page(self.generation, true);
    }
    fn update_fixed_total(&mut self) -> Result<(), String> {
        let Some(result) = &self.result else {
            self.state.summary_rows.clear();
            return Ok(());
        };
        let mut cells = BTreeMap::from([
            (ColumnId(0), "All filtered".into()),
            (ColumnId(1), result.total.count.to_string()),
            (ColumnId(2), money(result.total.sum)),
            (ColumnId(3), result.total.min.map(money).unwrap_or_default()),
            (ColumnId(4), result.total.max.map(money).unwrap_or_default()),
        ]);
        for month in 1..=12 {
            let key = format!("{month:02}");
            let sum = result.groups.iter().try_fold(0i128, |sum, group| {
                sum.checked_add(group.pivot.get(&key).map_or(0, |value| value.sum))
                    .ok_or("Fixed total overflow")
            })?;
            cells.insert(ColumnId(100 + month), money(sum));
        }
        self.state.summary_rows = vec![erp_ui_egui::SummaryRow {
            label: "Total".into(),
            cells,
        }];
        Ok(())
    }
    fn rebuild_columns(&mut self) {
        if !self.columns.is_empty()
            && self.columns.iter().any(|column| column.id.0 >= 100) == self.pivot
        {
            return;
        }
        self.columns = vec![
            Column::new(ColumnId(0), "Group", 230.0),
            Column::new(ColumnId(1), "Rows", 100.0),
            Column::new(ColumnId(2), "Value", 140.0),
        ];
        if self.pivot {
            for month in 1..=12 {
                self.columns.push(Column::new(
                    ColumnId(100 + month),
                    format!("{month:02}"),
                    115.0,
                ));
            }
        } else {
            self.columns
                .push(Column::new(ColumnId(3), "Minimum", 130.0));
            self.columns
                .push(Column::new(ColumnId(4), "Maximum", 130.0));
        }
        for column in &mut self.columns {
            column.value_kind = if column.id.0 == 0 {
                ValueKind::Text
            } else if column.id.0 == 1 {
                ValueKind::Integer
            } else {
                ValueKind::Money
            };
        }
    }
    fn select_detail(&mut self, selected: Option<RowId>) {
        self.selected = selected;
        let key = self
            .group_ids
            .iter()
            .find_map(|(key, id)| (Some(*id) == selected).then_some(key));
        self.detail_indices = self
            .records
            .iter()
            .enumerate()
            .filter_map(|(i, row)| (key.is_none_or(|key| row.group.starts_with(key))).then_some(i))
            .collect();
        self.pager.set_page(0, self.detail_indices.len());
        self.generation += 1;
        self.detail_state.bind_view(self.generation);
    }
    fn group_cell(group: &erp_grid_core::report::GroupSummary, column: ColumnId) -> String {
        match column.0 {
            0 => group.key.join(" / "),
            1 => group.summary.count.to_string(),
            2 => money(group.summary.sum),
            3 => group.summary.min.map(money).unwrap_or_default(),
            4 => group.summary.max.map(money).unwrap_or_default(),
            n if (101..=112).contains(&n) => group
                .pivot
                .get(&format!("{:02}", n - 100))
                .map(|value| money(value.sum))
                .unwrap_or_default(),
            _ => String::new(),
        }
    }
    fn export(&mut self, format: crate::export::Format) -> Result<(), String> {
        let result = self.result.as_ref().ok_or("Report is not ready")?;
        let (columns, rows) = self.summary_export_data(result)?;
        let bytes = if format == crate::export::Format::Xlsx {
            let sheet = crate::export::WorkbookSheet {
                name: "Report summary".into(),
                columns,
                rows,
            };
            let options = crate::export::SheetOptions {
                bold_rows: vec![sheet.rows.len() - 1],
                ..Default::default()
            };
            crate::export::encode_workbook_with_options(&[sheet], &[options])?
        } else {
            crate::export::encode(&columns, &rows, format)?
        };
        self.delivery = Some(crate::export::Delivery::start(bytes, format));
        Ok(())
    }

    fn summary_export_data(
        &self,
        result: &ReportResult,
    ) -> Result<(Vec<Column>, Vec<Vec<String>>), String> {
        let columns: Vec<_> = self
            .columns
            .iter()
            .filter(|column| column.visible)
            .cloned()
            .collect();
        let mut rows: Vec<_> = result
            .groups
            .iter()
            .map(|group| {
                columns
                    .iter()
                    .map(|column| Self::group_cell(group, column.id))
                    .collect()
            })
            .collect();
        let mut monthly = BTreeMap::new();
        for month in 1..=12 {
            let key = format!("{month:02}");
            let sum = result.groups.iter().try_fold(0i128, |sum, group| {
                sum.checked_add(group.pivot.get(&key).map_or(0, |value| value.sum))
                    .ok_or("Report monthly total overflow")
            })?;
            monthly.insert(month, sum);
        }
        rows.push(
            columns
                .iter()
                .map(|column| match column.id.0 {
                    0 => "Total".into(),
                    1 => result.total.count.to_string(),
                    2 => money(result.total.sum),
                    3 => result.total.min.map(money).unwrap_or_default(),
                    4 => result.total.max.map(money).unwrap_or_default(),
                    n if (101..=112).contains(&n) => money(monthly[&(n - 100)]),
                    _ => String::new(),
                })
                .collect(),
        );
        Ok((columns, rows))
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let now = ui.input(|input| input.time);
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
        let before = (
            self.warehouse,
            self.category,
            self.by_category,
            self.formula.clone(),
        );
        let old_pivot = self.pivot;
        let mut export_requested = false;
        let mut print_requested = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Inventory fixture").on_hover_text("10,000 local inventory records. Summaries cover the complete filtered fixture, not the displayed detail page.");
            egui::ComboBox::from_id_salt("report-warehouse").selected_text(if self.warehouse == 0 { "All warehouses" } else { WAREHOUSES[self.warehouse-1] }).show_ui(ui, |ui| { ui.selectable_value(&mut self.warehouse,0,"All warehouses"); for (i,label) in WAREHOUSES.iter().enumerate() { ui.selectable_value(&mut self.warehouse,i+1,*label); } });
            egui::ComboBox::from_id_salt("report-category").selected_text(if self.category == 0 { "All categories" } else { CATEGORIES[self.category-1] }).show_ui(ui, |ui| { ui.selectable_value(&mut self.category,0,"All categories"); for (i,label) in CATEGORIES.iter().enumerate() { ui.selectable_value(&mut self.category,i+1,*label); } });
            ui.checkbox(&mut self.by_category,"Group category"); ui.checkbox(&mut self.pivot,"Pivot months");
            ui.add(egui::TextEdit::singleline(&mut self.formula).desired_width(175.0).char_limit(4096)).on_hover_text("Calculated value: [2] quantity, [3] unit cost, [4] unit price. + - * / and parentheses. [6] is this value; margin [7] depends on [6]. Cycles are rejected. Round each row to cents before summing.");
            ui.selectable_value(&mut self.chart,ChartKind::Bar,"Bars"); ui.selectable_value(&mut self.chart,ChartKind::Line,"Line");
            export_requested = ui.add_enabled(self.result.is_some() && self.delivery.is_none(),egui::Button::new("Export summary")).clicked();
            print_requested = ui.add_enabled(self.result.is_some() && self.delivery.is_none(),egui::Button::new("Print document")).on_hover_text("Save a printable HTML document; open it in a browser to choose a printer or PDF").clicked();
        });
        if before
            != (
                self.warehouse,
                self.category,
                self.by_category,
                self.formula.clone(),
            )
        {
            self.invalidate(now);
        }
        if old_pivot != self.pivot {
            self.rebuild_columns();
            self.generation += 1;
            self.state.bind_view(self.generation);
        }
        if (export_requested || print_requested)
            && self.result.is_some()
            && let Err(error) = self.export(if print_requested {
                crate::export::Format::Html
            } else {
                crate::export::Format::Xlsx
            })
        {
            self.error = Some(error);
        }
        if self.deadline.is_some_and(|deadline| deadline <= now)
            && !self.composing
            && let Err(error) = self.start(now)
        {
            self.deadline = None;
            self.error = Some(error);
        }
        if let Err(error) = self.advance() {
            self.job = None;
            self.result = None;
            self.error = Some(error);
        }
        if self.job.is_some() || self.deadline.is_some() {
            ui.ctx().request_repaint();
            ui.spinner();
        }
        if let Some(delivery) = &self.delivery {
            if let Some(result) = delivery.poll() {
                self.error = Some(result.unwrap_or_else(|error| error));
                self.delivery = None;
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
        if let Some(error) = &self.error {
            ui.label(error);
        }
        if self.state.row_selection.active != self.selected {
            self.select_detail(self.state.row_selection.active);
        }
        let Some(result) = &self.result else {
            return;
        };
        let points: Vec<_> = if self.pivot {
            let points: Result<Vec<_>, String> = (1..=12)
                .map(|month| {
                    let key = format!("{month:02}");
                    let sum = result.groups.iter().try_fold(0i128, |sum, group| {
                        sum.checked_add(group.pivot.get(&key).map_or(0, |value| value.sum))
                            .ok_or_else(|| "Monthly chart total overflow".to_owned())
                    })?;
                    Ok((key, sum))
                })
                .collect();
            match points {
                Ok(points) => points,
                Err(error) => {
                    ui.label(error);
                    Vec::new()
                }
            }
        } else {
            result
                .groups
                .iter()
                .map(|group| (group.key.join(" / "), group.summary.sum))
                .collect()
        };
        if let Err(error) = report_chart::show(ui, ui.id().with("chart"), self.chart, &points, 2) {
            ui.label(error);
        }
        ui.horizontal(|ui| {
            ui.label(format!("Checked {}", self.state.row_selection.checked().len())).on_hover_text("Group checks are independent; collapsing preserves checked children. They do not select underlying records.");
            ui.label(format!(
                "All filtered: {} rows · {}",
                result.total.count,
                money(result.total.sum)
            ));
            if let Some(elapsed) = self.elapsed {
                ui.label(format!("{:.1} ms", elapsed * 1000.0));
            }
        });
        let available = ui.available_rect_before_wrap();
        let split = available.top() + (available.height() * 0.45).max(0.0);
        let group_output = ui
            .scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                    available.min,
                    egui::pos2(available.right(), split),
                )),
                |ui| {
                    DataGrid {
                        id: ui.id().with("report-summary"),
                        columns: &mut self.columns,
                        rows: self.visible_groups.len(),
                        state: &mut self.state,
                        row_id: Some(&|i| {
                            self.group_ids[&self.hierarchy[self.visible_groups[i]].key]
                        }),
                    }
                    .show(ui, |i, column| {
                        Self::group_cell(&self.hierarchy[self.visible_groups[i]], column)
                    })
                },
            )
            .inner;
        if let Some(id) = group_output.toggle_tree
            && let Some(key) = self
                .group_ids
                .iter()
                .find_map(|(key, value)| (*value == id).then(|| key.clone()))
        {
            if !self.expanded_groups.remove(&key) {
                self.expanded_groups.insert(key);
            }
            self.rebuild_group_rows();
            ui.ctx().request_repaint();
        }
        let footer = (available.bottom() - 28.0).max(split);
        let range = self.pager.range(self.detail_indices.len());
        self.detail_state.row_number_start = range.start as u64;
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                egui::pos2(available.left(), split),
                egui::pos2(available.right(), footer),
            )),
            |ui| {
                DataGrid {
                    id: ui.id().with("report-details"),
                    columns: &mut self.detail_columns,
                    rows: range.len(),
                    state: &mut self.detail_state,
                    row_id: Some(&|i| self.records[self.detail_indices[range.start + i]].id),
                }
                .show(ui, |i, column| {
                    self.records[self.detail_indices[range.start + i]]
                        .cells
                        .get(&column)
                        .cloned()
                        .unwrap_or_default()
                });
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                egui::pos2(available.left(), footer),
                available.max,
            )),
            |ui| {
                if erp_ui_egui::show_pagination(
                    ui,
                    ui.id().with("report-pager"),
                    &mut self.pager,
                    self.detail_indices.len(),
                    true,
                ) {
                    self.generation += 1;
                    self.detail_state.rebind_page(self.generation, true);
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_view_renders_summary_and_detail_in_both_modes() {
        let mut view = ReportView::default();
        view.start(0.0).unwrap();
        while view.job.is_some() {
            view.advance().unwrap();
        }
        let ctx = egui::Context::default();
        for pivot in [false, true] {
            view.pivot = pivot;
            view.rebuild_columns();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 900.0),
                    )),
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| view.ui(ui));
                },
            );
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect();
            assert!(texts.iter().any(|text| text.contains("All filtered:")));
            assert!(
                texts.iter().any(|text| text.contains("SKU-000001")),
                "Detail grid should paint within available area"
            );
            output.textures_delta.clear();
        }
    }
    #[test]
    fn report_hierarchy_expands_subtotals_and_parent_details_without_clearing_checks() {
        let mut view = ReportView {
            by_category: true,
            ..Default::default()
        };
        view.start(0.0).unwrap();
        while view.job.is_some() {
            view.advance().unwrap();
        }
        assert_eq!(view.visible_groups.len(), 4);
        let parent_key = vec!["North".to_owned()];
        let parent = view.group_ids[&parent_key];
        view.select_detail(Some(parent));
        assert_eq!(view.detail_indices.len(), 2500);
        view.expanded_groups.insert(parent_key.clone());
        view.rebuild_group_rows();
        assert_eq!(view.visible_groups.len(), 7);
        let child_key = vec!["North".to_owned(), "Home".to_owned()];
        let child = view.group_ids[&child_key];
        view.state.row_selection.set_checked(child, true);
        view.expanded_groups.remove(&parent_key);
        view.rebuild_group_rows();
        assert_eq!(view.visible_groups.len(), 4);
        assert!(view.state.row_selection.checked().contains(&child));
        assert!(!view.state.tree_rows.contains_key(&child));
        view.select_detail(Some(child));
        assert!(
            view.detail_indices
                .iter()
                .all(|index| view.records[*index].group == child_key)
        );
    }
    #[test]
    fn report_dependent_margin_and_fixed_total_share_complete_query_scope() {
        let mut view = ReportView::default();
        view.start(0.0).unwrap();
        while view.job.is_some() {
            view.advance().unwrap();
        }
        assert_eq!(view.records[0].cells[&ColumnId(7)], "2.50");
        assert_eq!(view.state.summary_rows.len(), 1);
        assert_eq!(view.state.summary_rows[0].cells[&ColumnId(1)], "10000");
        assert_eq!(
            view.state.summary_rows[0].cells[&ColumnId(2)],
            money(view.result.as_ref().unwrap().total.sum)
        );
        view.formula = "[7]".into();
        view.invalidate(1.0);
        assert!(view.state.summary_rows.is_empty());
        assert!(view.start(2.0).is_err());
        assert!(view.job.is_none());
    }
    #[test]
    fn report_fixture_totals_match_detail_values_and_filter_scope() {
        let mut view = ReportView {
            warehouse: 1,
            by_category: true,
            ..Default::default()
        };
        view.start(0.0).unwrap();
        let mut frames = 0;
        while view.job.is_some() {
            view.advance().unwrap();
            frames += 1;
            assert!(frames <= 40);
        }
        let report = view.result.as_ref().unwrap();
        assert_eq!(report.total.count, 2500);
        assert_eq!(report.groups.len(), 3);
        let expected: i128 = (0..ROWS)
            .filter(|i| i % 4 == 0)
            .map(|i| (i % 50 + 1) as i128 * (100 + i % 20_000) as i128)
            .sum();
        assert_eq!(report.total.sum, expected);
        assert_eq!(view.records.len(), 2500);
        assert!(frames > 1);
        assert_eq!(
            report.groups.iter().map(|g| g.summary.sum).sum::<i128>(),
            expected
        );
        assert_eq!(cents(-50), -1);
        assert_eq!(money(-1), "-0.01");
        view.formula = "[2] / 0".into();
        view.invalidate(1.0);
        view.start(2.0).unwrap();
        assert!(view.advance().is_err());
        assert!(view.result.is_none());
    }

    #[test]
    fn report_export_keeps_column_order_and_appends_original_total() {
        let mut view = ReportView {
            warehouse: 1,
            by_category: true,
            pivot: true,
            ..Default::default()
        };
        view.start(0.0).unwrap();
        while view.job.is_some() {
            view.advance().unwrap();
        }
        view.rebuild_columns();
        let result = view.result.as_ref().unwrap();
        let (columns, rows) = view.summary_export_data(result).unwrap();
        assert_eq!(
            columns
                .iter()
                .map(|column| column.title.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Group", "Rows", "Value", "01", "02", "03", "04", "05", "06", "07", "08", "09",
                "10", "11", "12"
            ]
        );
        let total = rows.last().unwrap();
        assert_eq!(total[0], "Total");
        assert_eq!(total[1], result.total.count.to_string());
        assert_eq!(total[2], money(result.total.sum));
        for month in 1..=12 {
            let key = format!("{month:02}");
            let expected = result.groups.iter().fold(0i128, |sum, group| {
                sum + group.pivot.get(&key).map_or(0, |value| value.sum)
            });
            assert_eq!(total[month + 2], money(expected));
        }
    }
}
