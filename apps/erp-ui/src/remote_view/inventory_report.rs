use super::Session;
use crate::http::{HttpError, HttpRequest, HttpTask};
use eframe::egui;
use erp_grid_core::{
    Column, ColumnId, RowId, ValueKind,
    report::{GroupSummary, MAX_GROUPS, MAX_TEXT_BUDGET_BYTES, ReportResult, Summary},
};
use erp_ui_egui::{
    DataGrid, GridState, TreeRowVisual,
    report_chart::{self, ChartKind},
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Measure {
    #[default]
    Quantity,
    Cost,
}

impl Measure {
    fn scale(self) -> u8 {
        if self == Self::Cost { 8 } else { 6 }
    }
}

#[derive(serde::Serialize)]
struct Request<'a> {
    schema: u8,
    warehouse_id: Option<&'a str>,
    search: Option<&'a str>,
    by_category: bool,
    measure: Measure,
}

#[derive(serde::Deserialize)]
struct WireResponse {
    schema: u32,
    amount_scale: u8,
    measure: Measure,
    valuation_source: Option<String>,
    buckets: Vec<WireBucket>,
    total: WireSummary,
    as_of: String,
}

#[derive(serde::Deserialize)]
struct WireBucket {
    warehouse_code: String,
    category: Option<String>,
    kind: String,
    summary: WireSummary,
}

#[derive(serde::Deserialize)]
struct WireSummary {
    count: u64,
    sum: String,
    min: Option<String>,
    max: Option<String>,
}

struct Decoded {
    report: ReportResult,
    labels: BTreeMap<String, String>,
    as_of: String,
}

struct Job {
    generation: u64,
    session_number: u64,
    task: HttpTask,
}

struct ReceiptRow {
    request_id: String,
    warehouse_id: String,
    sku_id: String,
    qty: String,
    on_hand: String,
    replay: bool,
    effect_id: String,
}

struct InboundDraft {
    warehouse_id: String,
    sku_id: String,
    qty: String,
    reason: String,
    request_id: String,
    task: Option<HttpTask>,
    pending_row: Option<ReceiptRow>,
    rows: Vec<ReceiptRow>,
    ids: BTreeMap<String, RowId>,
    columns: Vec<Column>,
    state: GridState,
}

pub(super) struct InventoryReport {
    open: bool,
    warehouse: String,
    search: String,
    by_category: bool,
    measure: Measure,
    composing: bool,
    deadline: Option<f64>,
    generation: u64,
    bound_session: Option<u64>,
    job: Option<Job>,
    report: Option<ReportResult>,
    chart_points: Vec<(String, i128)>,
    hierarchy: Vec<GroupSummary>,
    labels: BTreeMap<String, String>,
    visible: Vec<usize>,
    expanded: BTreeSet<Vec<String>>,
    ids: BTreeMap<Vec<String>, RowId>,
    columns: Vec<Column>,
    state: GridState,
    chart: ChartKind,
    as_of: Option<String>,
    error: Option<String>,
    delivery: Option<crate::export::Delivery>,
    delivery_feedback: Option<String>,
    inbound: InboundDraft,
}

impl Default for InventoryReport {
    fn default() -> Self {
        let mut state = GridState::default();
        state.read_only = true;
        state.filtering_enabled = false;
        state.sorting_enabled = false;
        state.quick_search_open = false;
        Self {
            open: false,
            warehouse: String::new(),
            search: String::new(),
            by_category: true,
            measure: Measure::Quantity,
            composing: false,
            deadline: None,
            generation: 0,
            bound_session: None,
            job: None,
            report: None,
            chart_points: Vec::new(),
            hierarchy: Vec::new(),
            labels: BTreeMap::new(),
            visible: Vec::new(),
            expanded: BTreeSet::new(),
            ids: BTreeMap::new(),
            columns: columns(),
            state,
            chart: ChartKind::Bar,
            as_of: None,
            error: None,
            delivery: None,
            delivery_feedback: None,
            inbound: InboundDraft::default(),
        }
    }
}

impl InventoryReport {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn open_button(&mut self, ui: &mut egui::Ui, session_number: u64) {
        if self.bound_session != Some(session_number) {
            self.clear();
            self.bound_session = Some(session_number);
        }
        if ui.button("库存报表").clicked() {
            self.open = true;
            self.invalidate(ui.input(|input| input.time), true);
        }
    }

    fn invalidate(&mut self, now: f64, immediate: bool) {
        self.generation = self.generation.wrapping_add(1);
        self.job = None;
        self.report = None;
        self.chart_points.clear();
        self.expanded.clear();
        self.hierarchy.clear();
        self.labels.clear();
        self.visible.clear();
        self.state.tree_rows.clear();
        self.state.summary_rows.clear();
        self.as_of = None;
        self.error = None;
        self.deadline = Some(if immediate { now } else { now + 0.25 });
        self.state.bind_view(self.generation);
    }

    pub(super) fn ui(&mut self, ctx: &egui::Context, session: &Session, session_number: u64) {
        if self.bound_session != Some(session_number) {
            self.clear();
            self.bound_session = Some(session_number);
            return;
        }
        self.poll_delivery(session_number);
        if self.delivery.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if !self.open {
            return;
        }
        ctx.input(|input| {
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
        self.poll();
        self.poll_inbound(session);
        let now = ctx.input(|input| input.time);
        if self.deadline.is_some_and(|deadline| deadline <= now) && !self.composing {
            self.deadline = None;
            if let Err(error) = self.start(session, ctx, session_number) {
                self.error = Some(error);
            }
        }
        if self.job.is_some() || self.deadline.is_some() {
            ctx.request_repaint_after(Duration::from_millis(25));
        }
        let mut open = self.open;
        egui::Window::new("库存报表")
            .id(egui::Id::new("remote-inventory-report"))
            .open(&mut open)
            .default_size(egui::vec2(820.0, 620.0))
            .show(ctx, |ui| self.contents(ui, session));
        self.open = open;
        if !open {
            self.job = None;
            self.deadline = None;
        }
    }

    fn contents(&mut self, ui: &mut egui::Ui, session: &Session) {
        let before = (
            self.warehouse.clone(),
            self.search.clone(),
            self.by_category,
            self.measure,
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("仓库");
            ui.add(
                egui::TextEdit::singleline(&mut self.warehouse)
                    .desired_width(230.0)
                    .char_limit(36)
                    .hint_text("全部授权仓库"),
            )
            .on_hover_text("留空查询全部授权仓库；指定时输入仓库 UUID");
            ui.label("搜索");
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .desired_width(180.0)
                    .char_limit(256),
            );
            ui.checkbox(&mut self.by_category, "按分类");
            ui.selectable_value(&mut self.measure, Measure::Quantity, "数量");
            ui.selectable_value(&mut self.measure, Measure::Cost, "成本");
            ui.selectable_value(&mut self.chart, ChartKind::Bar, "柱形");
            ui.selectable_value(&mut self.chart, ChartKind::Line, "折线");
        });
        let after = (
            self.warehouse.clone(),
            self.search.clone(),
            self.by_category,
            self.measure,
        );
        if before != after {
            self.invalidate(ui.input(|input| input.time), false);
        }
        if self.job.is_some() || self.deadline.is_some() {
            ui.spinner();
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if let Some(feedback) = &self.delivery_feedback {
            ui.label(feedback);
        }
        let export_ready = self
            .report
            .as_ref()
            .is_some_and(|report| !report.groups.is_empty())
            && self.job.is_none()
            && self.deadline.is_none()
            && !self.composing
            && self.error.is_none()
            && self.delivery.is_none();
        let mut export = None;
        ui.horizontal(|ui| {
            for (label, format, help) in [
                (
                    "CSV",
                    crate::export::Format::Csv,
                    "导出完整叶子分组及总计为 CSV",
                ),
                (
                    "Excel",
                    crate::export::Format::Xlsx,
                    "导出完整叶子分组及总计为 Excel",
                ),
                (
                    "打印",
                    crate::export::Format::Html,
                    "保存可打印 HTML；用浏览器打开后打印或另存 PDF",
                ),
            ] {
                if ui
                    .add_enabled(export_ready, egui::Button::new(label).small())
                    .on_hover_text(help)
                    .clicked()
                {
                    export = Some(format);
                }
            }
        });
        if let Some(format) = export
            && let Err(error) = self.export(format)
        {
            self.delivery_feedback = Some(error);
        }
        self.inbound_editor(ui, session);
        let Some(report) = &self.report else { return };
        if let Err(error) = report_chart::show(
            ui,
            ui.id().with("inventory-report-chart"),
            self.chart,
            &self.chart_points,
            u32::from(self.measure.scale()),
        ) {
            ui.label(error);
        }
        ui.horizontal(|ui| {
            ui.label(format!(
                "合计：{} 条 · {}",
                report.total.count,
                format_amount(report.total.sum, self.measure)
            ));
            if let Some(as_of) = &self.as_of {
                ui.label(format!("快照 {as_of}"));
            }
        });
        let output = DataGrid {
            id: ui.id().with("inventory-report-grid"),
            columns: &mut self.columns,
            rows: self.visible.len(),
            state: &mut self.state,
            row_id: Some(&|index| self.ids[&self.hierarchy[self.visible[index]].key]),
        }
        .show(ui, |index, column| {
            let group = &self.hierarchy[self.visible[index]];
            group_cell(
                group,
                column,
                self.measure,
                &self.labels,
                &report.pivot_keys,
            )
        });
        if let Some(id) = output.toggle_tree
            && let Some(key) = self
                .ids
                .iter()
                .find_map(|(key, value)| (*value == id).then(|| key.clone()))
        {
            if !self.expanded.remove(&key) {
                self.expanded.insert(key);
            }
            self.rebuild_visible();
        }
    }

    fn start(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
        session_number: u64,
    ) -> Result<(), String> {
        let warehouse = self.warehouse.trim();
        let warehouse = if warehouse.is_empty() {
            None
        } else {
            uuid::Uuid::parse_str(warehouse).map_err(|_| "仓库 UUID 无效".to_owned())?;
            Some(warehouse)
        };
        let search = self.search.trim();
        let body = serde_json::to_vec(&Request {
            schema: 2,
            warehouse_id: warehouse,
            search: (!search.is_empty()).then_some(search),
            by_category: self.by_category,
            measure: self.measure,
        })
        .map_err(|_| "无法编码库存报表请求".to_owned())?;
        let task = session
            .client
            .start(
                HttpRequest {
                    path: format!(
                        "/v1/tenants/{}/reports/inventory/aggregate",
                        session.tenant.to_uuid()
                    ),
                    query: vec![],
                    json: Some(body),
                    bearer: Some(session.token.clone()),
                    idempotency_key: None,
                },
                ctx,
            )
            .map_err(|error| error.to_string())?;
        self.job = Some(Job {
            generation: self.generation,
            session_number,
            task,
        });
        Ok(())
    }

    fn poll(&mut self) {
        let result = self.job.as_mut().and_then(|job| job.task.poll());
        let Some(result) = result else { return };
        let Some(job) = self.job.take() else { return };
        if job.generation != self.generation || self.bound_session != Some(job.session_number) {
            return;
        }
        match result {
            Ok(bytes) => match decode(&bytes, self.by_category, self.measure) {
                Ok(decoded) => {
                    self.labels = decoded.labels;
                    self.as_of = Some(decoded.as_of);
                    self.hierarchy = match crate::report_view::hierarchy::rollup(&decoded.report) {
                        Ok(groups) => groups,
                        Err(error) => {
                            self.error = Some(error);
                            return;
                        }
                    };
                    self.chart_points = decoded
                        .report
                        .groups
                        .iter()
                        .map(|group| (display_key(&group.key, &self.labels), group.summary.sum))
                        .collect();
                    self.report = Some(decoded.report);
                    self.rebuild_columns();
                    self.ids.clear();
                    for (index, group) in self.hierarchy.iter().enumerate() {
                        self.ids
                            .insert(group.key.clone(), RowId((1u128 << 127) + index as u128 + 1));
                    }
                    self.update_total();
                    self.rebuild_visible();
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            },
            Err(HttpError::Status(401)) => self.error = Some("登录已失效，请重新登录".into()),
            Err(HttpError::Status(403)) => self.error = Some("没有库存报表查看权限".into()),
            Err(HttpError::Status(422)) if self.measure == Measure::Cost => {
                self.error = Some("参考价值缺失，无法生成成本报表".into())
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn poll_delivery(&mut self, session_number: u64) {
        let result = self
            .delivery
            .as_ref()
            .and_then(crate::export::Delivery::poll);
        let Some(result) = result else { return };
        self.delivery = None;
        if self.bound_session != Some(session_number) {
            return;
        }
        self.delivery_feedback = Some(result.unwrap_or_else(|error| error));
    }

    fn export(&mut self, format: crate::export::Format) -> Result<(), String> {
        let report = self.report.as_ref().ok_or("库存报表尚未完成")?;
        if report.groups.is_empty() {
            return Err("库存报表没有可导出的结果".into());
        }
        let (columns, rows) = self.export_data(report)?;
        let bytes = if format == crate::export::Format::Xlsx {
            let sheet = crate::export::WorkbookSheet {
                name: match self.measure {
                    Measure::Quantity => "库存数量",
                    Measure::Cost => "参考价值(元)",
                }
                .into(),
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
        self.delivery_feedback = None;
        self.delivery = Some(crate::export::Delivery::start(bytes, format));
        Ok(())
    }

    fn export_data(
        &self,
        report: &ReportResult,
    ) -> Result<(Vec<Column>, Vec<Vec<String>>), String> {
        let mut columns: Vec<_> = self
            .columns
            .iter()
            .filter(|column| column.visible)
            .cloned()
            .collect();
        let amount_name = match self.measure {
            Measure::Quantity => "库存数量",
            Measure::Cost => "参考价值(元)",
        };
        for column in &mut columns {
            column.title = match column.id.0 {
                0 => self.as_of.as_ref().map_or_else(
                    || column.title.clone(),
                    |as_of| format!("{}（快照 {as_of}）", column.title),
                ),
                2 => format!("{amount_name}合计"),
                3 => format!("{amount_name}最小"),
                4 => format!("{amount_name}最大"),
                id if id >= 100 => format!("{} {amount_name}", column.title),
                _ => column.title.clone(),
            };
        }
        if columns.is_empty() {
            return Err("库存报表没有可导出的列".into());
        }
        let mut rows: Vec<Vec<String>> = report
            .groups
            .iter()
            .map(|group| {
                columns
                    .iter()
                    .map(|column| {
                        group_cell(
                            group,
                            column.id,
                            self.measure,
                            &self.labels,
                            &report.pivot_keys,
                        )
                    })
                    .collect()
            })
            .collect();
        let mut pivot_totals = BTreeMap::new();
        for (index, pivot) in report.pivot_keys.iter().enumerate() {
            let total = report.groups.iter().try_fold(0i128, |total, group| {
                total.checked_add(group.pivot.get(pivot).map_or(0, |summary| summary.sum))
            });
            pivot_totals.insert(
                ColumnId(100 + index as u64),
                total.ok_or("库存报表透视总计溢出")?,
            );
        }
        rows.push(
            columns
                .iter()
                .map(|column| match column.id.0 {
                    0 => "全部结果".into(),
                    1 => report.total.count.to_string(),
                    2 => format_amount(report.total.sum, self.measure),
                    3 => report
                        .total
                        .min
                        .map(|value| format_amount(value, self.measure))
                        .unwrap_or_default(),
                    4 => report
                        .total
                        .max
                        .map(|value| format_amount(value, self.measure))
                        .unwrap_or_default(),
                    id if id >= 100 => pivot_totals
                        .get(&column.id)
                        .map(|value| format_amount(*value, self.measure))
                        .unwrap_or_default(),
                    _ => String::new(),
                })
                .collect(),
        );
        Ok((columns, rows))
    }

    fn rebuild_visible(&mut self) {
        let parents = self
            .hierarchy
            .iter()
            .filter(|group| group.key.len() > 1)
            .map(|group| group.key[..group.key.len() - 1].to_vec())
            .collect::<BTreeSet<_>>();
        self.visible = self
            .hierarchy
            .iter()
            .enumerate()
            .filter_map(|(index, group)| {
                (1..group.key.len())
                    .all(|depth| self.expanded.contains(&group.key[..depth]))
                    .then_some(index)
            })
            .collect();
        self.state.tree_rows = self
            .visible
            .iter()
            .map(|index| {
                let group = &self.hierarchy[*index];
                let has_children = parents.contains(&group.key);
                (
                    self.ids[&group.key],
                    TreeRowVisual {
                        depth: (group.key.len() - 1) as u8,
                        expanded: self.expanded.contains(&group.key),
                        has_children,
                        loading: false,
                        error: None,
                    },
                )
            })
            .collect();
        self.generation = self.generation.wrapping_add(1);
        self.state.rebind_page(self.generation, true);
    }

    fn update_total(&mut self) {
        let Some(report) = &self.report else { return };
        let mut cells = BTreeMap::from([
            (ColumnId(0), "全部结果".into()),
            (ColumnId(1), report.total.count.to_string()),
            (ColumnId(2), format_amount(report.total.sum, self.measure)),
            (
                ColumnId(3),
                report
                    .total
                    .min
                    .map(|v| format_amount(v, self.measure))
                    .unwrap_or_default(),
            ),
            (
                ColumnId(4),
                report
                    .total
                    .max
                    .map(|v| format_amount(v, self.measure))
                    .unwrap_or_default(),
            ),
        ]);
        for (index, pivot) in report.pivot_keys.iter().enumerate() {
            let value = report.groups.iter().try_fold(0i128, |total, group| {
                total.checked_add(group.pivot.get(pivot).map_or(0, |summary| summary.sum))
            });
            if let Some(value) = value {
                cells.insert(
                    ColumnId(100 + index as u64),
                    format_amount(value, self.measure),
                );
            }
        }
        self.state.summary_rows = vec![erp_ui_egui::SummaryRow {
            label: "合计".into(),
            cells,
        }];
    }

    fn rebuild_columns(&mut self) {
        self.columns = columns();
        let amount_kind = ValueKind::Decimal {
            scale: self.measure.scale(),
        };
        for column in self.columns.iter_mut().filter(|column| column.id.0 >= 2) {
            column.value_kind = amount_kind;
        }
        if let Some(report) = &self.report {
            for (index, pivot) in report.pivot_keys.iter().enumerate() {
                let mut column = Column::new(ColumnId(100 + index as u64), pivot, 115.0);
                column.value_kind = amount_kind;
                self.columns.push(column);
            }
        }
    }
}

impl Default for InboundDraft {
    fn default() -> Self {
        Self {
            warehouse_id: String::new(),
            sku_id: String::new(),
            qty: String::new(),
            reason: String::new(),
            request_id: String::new(),
            task: None,
            pending_row: None,
            rows: Vec::new(),
            ids: BTreeMap::new(),
            columns: inbound_columns(),
            state: GridState::default(),
        }
    }
}

impl InventoryReport {
    fn inbound_editor(&mut self, ui: &mut egui::Ui, session: &Session) {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("仓库");
            ui.add(
                egui::TextEdit::singleline(&mut self.inbound.warehouse_id)
                    .desired_width(220.0)
                    .char_limit(36),
            );
            ui.label("货品");
            ui.add(
                egui::TextEdit::singleline(&mut self.inbound.sku_id)
                    .desired_width(220.0)
                    .char_limit(36),
            );
            ui.label("数量");
            ui.add(
                egui::TextEdit::singleline(&mut self.inbound.qty)
                    .desired_width(72.0)
                    .char_limit(12),
            )
            .on_hover_text("整基础单位。1 仍表示一件，不是 0.001");
            ui.label("原因");
            ui.add(
                egui::TextEdit::singleline(&mut self.inbound.reason)
                    .desired_width(120.0)
                    .char_limit(80),
            );
        });
        let busy = self.inbound.task.is_some();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!busy, egui::Button::new("入库").small())
                .on_hover_text("同一命令身份重试不会再次增加库存")
                .clicked()
                && let Err(error) = self.start_inbound(session, ui.ctx(), false)
            {
                self.error = Some(error);
            }
            if ui
                .add_enabled(
                    !busy && !self.inbound.request_id.is_empty(),
                    egui::Button::new("重试原命令").small(),
                )
                .on_hover_text("沿用上次命令身份再次提交")
                .clicked()
                && let Err(error) = self.start_inbound(session, ui.ctx(), true)
            {
                self.error = Some(error);
            }
            if busy {
                ui.spinner();
            }
        });
        if self.inbound.rows.is_empty() {
            return;
        }
        let cells: Vec<(RowId, String, String, String, String)> = self
            .inbound
            .rows
            .iter()
            .map(|row| {
                (
                    self.inbound.ids[&row.request_id],
                    row.request_id.clone(),
                    row.on_hand.clone(),
                    if row.replay {
                        "原结果".to_owned()
                    } else {
                        "已入账".to_owned()
                    },
                    row.effect_id.clone(),
                )
            })
            .collect();
        let output = DataGrid {
            id: ui.id().with("inventory-inbound-grid"),
            columns: &mut self.inbound.columns,
            rows: cells.len(),
            state: &mut self.inbound.state,
            row_id: Some(&|index| cells[index].0),
        }
        .show(ui, |index, column| match column.0 {
            0 => cells[index].1.clone(),
            1 => cells[index].2.clone(),
            2 => cells[index].3.clone(),
            _ => cells[index].4.clone(),
        });
        let _ = output;
    }

    fn start_inbound(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
        reuse: bool,
    ) -> Result<(), String> {
        let warehouse_id = self.inbound.warehouse_id.trim();
        let sku_id = self.inbound.sku_id.trim();
        uuid::Uuid::parse_str(warehouse_id).map_err(|_| "仓库 UUID 无效".to_owned())?;
        uuid::Uuid::parse_str(sku_id).map_err(|_| "货品 UUID 无效".to_owned())?;
        let qty = self.inbound.qty.trim();
        let parsed: i64 = qty.parse().map_err(|_| "数量必须是正整数".to_owned())?;
        if parsed <= 0 {
            return Err("数量必须是正整数".into());
        }
        if !reuse || self.inbound.request_id.trim().is_empty() {
            self.inbound.request_id = uuid::Uuid::new_v4().to_string();
        }
        let warehouse_id = warehouse_id.to_owned();
        let sku_id = sku_id.to_owned();
        let qty_text = parsed.to_string();
        let body = serde_json::json!({
            "warehouse_id": warehouse_id,
            "sku_id": sku_id,
            "quantity": parsed,
            "reason": self.inbound.reason.trim(),
            "request_id": self.inbound.request_id.trim(),
        });
        self.inbound.pending_row = Some(ReceiptRow {
            request_id: self.inbound.request_id.trim().to_owned(),
            warehouse_id,
            sku_id,
            qty: qty_text,
            on_hand: String::new(),
            replay: false,
            effect_id: String::new(),
        });
        let bytes = serde_json::to_vec(&body).map_err(|_| "无法编码入库请求".to_owned())?;
        let task = session
            .client
            .start(
                HttpRequest {
                    path: format!(
                        "/v1/tenants/{}/inventory/inbounds",
                        session.tenant.to_uuid()
                    ),
                    query: vec![],
                    json: Some(bytes),
                    bearer: Some(session.token.clone()),
                    idempotency_key: Some(self.inbound.request_id.trim().to_owned()),
                },
                ctx,
            )
            .map_err(|error| error.to_string())?;
        self.inbound.task = Some(task);
        self.error = None;
        Ok(())
    }

    fn poll_inbound(&mut self, session: &Session) {
        let result = self.inbound.task.as_mut().and_then(HttpTask::poll);
        let Some(result) = result else { return };
        self.inbound.task = None;
        match result {
            Ok(bytes) => match decode_inbound(&bytes, self.inbound.pending_row.take()) {
                Ok(row) => {
                    let request_id = row.request_id.clone();
                    if let Some(existing) = self
                        .inbound
                        .rows
                        .iter_mut()
                        .find(|item| item.request_id == request_id)
                    {
                        *existing = row;
                    } else {
                        self.inbound
                            .ids
                            .insert(request_id.clone(), RowId(uuid::Uuid::new_v4().as_u128()));
                        self.inbound.rows.push(row);
                    }
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            },
            Err(HttpError::Status(401)) => self.error = Some("登录已失效，请重新登录".into()),
            Err(HttpError::Status(403)) => self.error = Some("没有库存写入权限".into()),
            Err(HttpError::Status(409)) => self.error = Some("同一命令身份的内容不一致".into()),
            Err(error) => self.error = Some(error.to_string()),
        }
        let _ = session;
    }
}

#[cfg(test)]
mod inbound_contract_tests {
    use super::*;

    #[test]
    fn inbound_receipt_keeps_request_and_marks_replay() {
        let pending = ReceiptRow {
            request_id: "cmd-1".into(),
            warehouse_id: "wh".into(),
            sku_id: "sku".into(),
            qty: "1".into(),
            on_hand: String::new(),
            replay: false,
            effect_id: String::new(),
        };
        let bytes = br#"{"request_id":"cmd-1","status":"SUCCEEDED","replay":true,"effect_id":"ledger-1","record":{"warehouse_id":"wh","sku_id":"sku","on_hand":1,"reserved":0,"frozen":0,"in_transit":0,"available":1,"updated_at":"2026-10-01T00:00:00Z"}}"#;
        let row = decode_inbound(bytes, Some(pending)).expect("receipt");
        assert_eq!(row.request_id, "cmd-1");
        assert!(row.replay);
        assert_eq!(row.on_hand, "1");
        assert_eq!(row.effect_id, "ledger-1");
    }

    #[test]
    fn inbound_request_carries_the_same_command_identity() {
        let request_id = "cmd-1";
        let body = serde_json::json!({
            "warehouse_id": "11111111-1111-1111-1111-111111111111",
            "sku_id": "22222222-2222-2222-2222-222222222222",
            "quantity": 1,
            "reason": "sample",
            "request_id": request_id,
        });
        let request = crate::http::HttpRequest {
            path: "/v1/tenants/tenant/inventory/inbounds".into(),
            query: vec![],
            json: Some(serde_json::to_vec(&body).unwrap()),
            bearer: Some("token".into()),
            idempotency_key: Some(request_id.into()),
        };
        let parsed: serde_json::Value =
            serde_json::from_slice(request.json.as_ref().unwrap()).unwrap();
        assert_eq!(parsed["request_id"], request_id);
        assert_eq!(request.idempotency_key.as_deref(), Some(request_id));
        assert_eq!(parsed["quantity"], 1);
    }
}

#[derive(serde::Deserialize)]
struct InboundWire {
    request_id: String,
    replay: bool,
    effect_id: String,
    record: InboundRecordWire,
}

#[derive(serde::Deserialize)]
struct InboundRecordWire {
    on_hand: i64,
}

fn decode_inbound(bytes: &[u8], pending: Option<ReceiptRow>) -> Result<ReceiptRow, String> {
    let wire: InboundWire =
        serde_json::from_slice(bytes).map_err(|_| "入库回执无法读取".to_owned())?;
    let pending = pending.unwrap_or(ReceiptRow {
        request_id: wire.request_id.clone(),
        warehouse_id: String::new(),
        sku_id: String::new(),
        qty: String::new(),
        on_hand: String::new(),
        replay: false,
        effect_id: String::new(),
    });
    Ok(ReceiptRow {
        request_id: wire.request_id,
        warehouse_id: pending.warehouse_id,
        sku_id: pending.sku_id,
        qty: pending.qty,
        on_hand: wire.record.on_hand.to_string(),
        replay: wire.replay,
        effect_id: wire.effect_id,
    })
}

fn inbound_columns() -> Vec<Column> {
    [
        (0, "命令", 220.0),
        (1, "在手", 90.0),
        (2, "回执", 90.0),
        (3, "流水", 220.0),
    ]
    .into_iter()
    .map(|(id, title, width)| Column::new(ColumnId(id), title, width))
    .collect()
}

fn columns() -> Vec<Column> {
    [
        (0, "分组", 260.0, ValueKind::Text),
        (1, "记录数", 100.0, ValueKind::Integer),
        (2, "合计", 140.0, ValueKind::Money),
        (3, "最小", 130.0, ValueKind::Money),
        (4, "最大", 130.0, ValueKind::Money),
    ]
    .into_iter()
    .map(|(id, title, width, kind)| {
        let mut column = Column::new(ColumnId(id), title, width);
        column.value_kind = kind;
        column
    })
    .collect()
}

fn parse_summary(wire: WireSummary) -> Result<Summary, String> {
    let sum = wire.sum.parse::<i128>().map_err(|_| "库存报表数值无效")?;
    let min = wire
        .min
        .map(|value| value.parse::<i128>().map_err(|_| "库存报表最小值无效"))
        .transpose()?;
    let max = wire
        .max
        .map(|value| value.parse::<i128>().map_err(|_| "库存报表最大值无效"))
        .transpose()?;
    if (wire.count == 0 && (sum != 0 || min.is_some() || max.is_some()))
        || (wire.count > 0 && (min.is_none() || max.is_none()))
        || min.zip(max).is_some_and(|(min, max)| min > max)
    {
        return Err("库存报表汇总不一致".into());
    }
    Ok(Summary {
        count: wire.count,
        sum,
        min,
        max,
    })
}

fn decode(bytes: &[u8], by_category: bool, measure: Measure) -> Result<Decoded, String> {
    if bytes.len() > MAX_TEXT_BUDGET_BYTES {
        return Err("库存报表响应超过 8 MiB".into());
    }
    let wire: WireResponse = serde_json::from_slice(bytes).map_err(|_| "库存报表响应无效")?;
    if wire.schema != 2
        || wire.measure != measure
        || wire.amount_scale != measure.scale()
        || (measure == Measure::Cost
            && wire.valuation_source.as_deref() != Some("sku_master_estimate"))
        || (measure == Measure::Quantity && wire.valuation_source.is_some())
    {
        return Err("库存报表精度或计价来源与查询不一致".into());
    }
    if wire.buckets.len() > MAX_GROUPS || wire.as_of.is_empty() || wire.as_of.len() > 128 {
        return Err("库存报表响应范围无效".into());
    }
    let total = parse_summary(wire.total)?;
    let mut groups: BTreeMap<Vec<String>, GroupSummary> = BTreeMap::new();
    let mut labels = BTreeMap::new();
    let mut merged = Summary::default();
    let mut bucket_keys = BTreeSet::new();
    let mut pivot_keys = BTreeSet::new();
    let mut pivot_totals: BTreeMap<String, Summary> = BTreeMap::new();
    for bucket in wire.buckets {
        if !by_category && bucket.category.is_some() {
            return Err("库存报表分类与查询不一致".into());
        }
        for text in [
            &bucket.warehouse_code,
            bucket.category.as_deref().unwrap_or(""),
            &bucket.kind,
        ] {
            if text.len() > 4096 || text.contains('\0') {
                return Err("库存报表文本无效".into());
            }
        }
        let summary = parse_summary(bucket.summary)?;
        if summary.count == 0 {
            return Err("库存报表包含空分组".into());
        }
        if !bucket_keys.insert((
            bucket.warehouse_code.clone(),
            bucket.category.clone(),
            bucket.kind.clone(),
        )) {
            return Err("库存报表包含重复分组".into());
        }
        if !pivot_keys.contains(&bucket.kind) && pivot_keys.len() == 64 {
            return Err("库存报表类型列超过 64 个".into());
        }
        pivot_keys.insert(bucket.kind.clone());
        merge(
            pivot_totals.entry(bucket.kind.clone()).or_default(),
            &summary,
        )?;
        merge(&mut merged, &summary)?;
        let warehouse_key = format!("w:{}", bucket.warehouse_code);
        labels.insert(warehouse_key.clone(), bucket.warehouse_code);
        let mut key = vec![warehouse_key];
        if by_category {
            let category_key = match bucket.category {
                Some(value) => {
                    let encoded = format!("c:value:{value}");
                    labels.insert(encoded.clone(), value);
                    encoded
                }
                None => {
                    labels.insert("c:null".into(), "未分类".into());
                    "c:null".into()
                }
            };
            key.push(category_key);
        }
        let entry = groups.entry(key.clone()).or_insert_with(|| GroupSummary {
            key,
            summary: Summary::default(),
            pivot: BTreeMap::new(),
        });
        merge(&mut entry.summary, &summary)?;
        merge(entry.pivot.entry(bucket.kind).or_default(), &summary)?;
    }
    if merged != total {
        return Err("库存报表总计与分组不一致".into());
    }
    Ok(Decoded {
        report: ReportResult {
            groups: groups.into_values().collect(),
            pivot_keys: pivot_keys.into_iter().collect(),
            total,
        },
        labels,
        as_of: wire.as_of,
    })
}

fn merge(target: &mut Summary, source: &Summary) -> Result<(), String> {
    target.count = target
        .count
        .checked_add(source.count)
        .ok_or("库存报表记录数溢出")?;
    target.sum = target
        .sum
        .checked_add(source.sum)
        .ok_or("库存报表合计溢出")?;
    target.min = target.min.into_iter().chain(source.min).min();
    target.max = target.max.into_iter().chain(source.max).max();
    Ok(())
}

fn display_key(key: &[String], labels: &BTreeMap<String, String>) -> String {
    key.iter()
        .map(|part| labels.get(part).map_or(part.as_str(), String::as_str))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn group_cell(
    group: &GroupSummary,
    column: ColumnId,
    measure: Measure,
    labels: &BTreeMap<String, String>,
    pivot_keys: &[String],
) -> String {
    match column.0 {
        0 => display_key(&group.key, labels),
        1 => group.summary.count.to_string(),
        2 => format_amount(group.summary.sum, measure),
        3 => group
            .summary
            .min
            .map(|value| format_amount(value, measure))
            .unwrap_or_default(),
        4 => group
            .summary
            .max
            .map(|value| format_amount(value, measure))
            .unwrap_or_default(),
        id if id >= 100 => pivot_keys
            .get((id - 100) as usize)
            .and_then(|pivot| group.pivot.get(pivot))
            .map(|summary| format_amount(summary.sum, measure))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn format_amount(value: i128, measure: Measure) -> String {
    let mut text = erp_grid_core::decimal_text(value, measure.scale());
    if measure == Measure::Quantity {
        return text;
    }
    match text.split_once('.').map(|(_, fraction)| fraction.len()) {
        None => text.push_str(".00"),
        Some(1) => text.push('0'),
        _ => {}
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_report_scale_and_cost_source_cannot_be_silently_changed() {
        let mut wire = serde_json::json!({
            "schema":2,"amount_scale":8,"measure":"cost","valuation_source":"sku_master_estimate",
            "buckets":[{"warehouse_code":"WH","category":null,"kind":"NORMAL",
                "summary":{"count":1,"sum":"13300000","min":"13300000","max":"13300000"}}],
            "total":{"count":1,"sum":"13300000","min":"13300000","max":"13300000"},
            "as_of":"2026-10-03T00:00:00Z"
        });
        let decode_wire = |wire: &serde_json::Value, measure| {
            decode(&serde_json::to_vec(wire).unwrap(), true, measure)
        };
        let report = decode_wire(&wire, Measure::Cost).unwrap();
        assert_eq!(
            format_amount(report.report.total.sum, Measure::Cost),
            "0.133"
        );
        assert!(decode_wire(&wire, Measure::Quantity).is_err());
        wire["amount_scale"] = serde_json::json!(2);
        assert!(decode_wire(&wire, Measure::Cost).is_err());
        wire["amount_scale"] = serde_json::json!(8);
        wire["valuation_source"] = serde_json::Value::Null;
        assert!(decode_wire(&wire, Measure::Cost).is_err());
        wire["valuation_source"] = serde_json::json!("sku_master_estimate");
        wire["schema"] = serde_json::json!(1);
        assert!(decode_wire(&wire, Measure::Cost).is_err());
        assert_eq!(
            format_amount(18_446_744_073_709_551_614, Measure::Quantity),
            "18446744073709.551614"
        );
    }

    #[test]
    fn fractional_cost_exports_preserve_sub_cent_values_and_total() {
        use calamine::{Data, Reader};
        let wire = serde_json::json!({
            "schema":2,"amount_scale":8,"measure":"cost","valuation_source":"sku_master_estimate",
            "buckets":[
                {"warehouse_code":"A","category":null,"kind":"NORMAL","summary":{"count":1,"sum":"13300000","min":"13300000","max":"13300000"}},
                {"warehouse_code":"B","category":null,"kind":"NORMAL","summary":{"count":1,"sum":"1400000","min":"1400000","max":"1400000"}}
            ],"total":{"count":2,"sum":"14700000","min":"1400000","max":"13300000"},"as_of":"2026-10-03T00:00:00Z"
        });
        let decoded = decode(&serde_json::to_vec(&wire).unwrap(), false, Measure::Cost).unwrap();
        let mut view = InventoryReport {
            measure: Measure::Cost,
            labels: decoded.labels,
            report: Some(decoded.report.clone()),
            ..Default::default()
        };
        view.rebuild_columns();
        let (columns, rows) = view.export_data(&decoded.report).unwrap();
        assert!(
            columns
                .iter()
                .filter(|c| c.id.0 >= 2)
                .all(|c| c.value_kind == ValueKind::Decimal { scale: 8 })
        );
        assert_eq!(rows[0][2], "0.133");
        assert_eq!(rows[1][2], "0.014");
        assert_eq!(rows.last().unwrap()[2], "0.147");
        let csv = crate::export::encode(&columns, &rows, crate::export::Format::Csv).unwrap();
        let csv = String::from_utf8(csv).unwrap();
        assert!(csv.contains(",\"0.133\","));
        assert!(csv.contains(",\"0.147\","));
        let bytes = crate::export::encode(&columns, &rows, crate::export::Format::Xlsx).unwrap();
        let mut workbook = calamine::Xlsx::new(std::io::Cursor::new(bytes)).unwrap();
        let cells = workbook.worksheet_range("Data").unwrap();
        assert_eq!(cells.get_value((1, 2)), Some(&Data::Float(0.133)));
        assert_eq!(cells.get_value((3, 2)), Some(&Data::Float(0.147)));
    }

    #[test]
    fn decoder_preserves_null_category_and_exact_cost() {
        let text = r#"{"buckets":[{"warehouse_code":"WH","category":null,"kind":"normal","summary":{"count":1,"sum":"-5000000","min":"-5000000","max":"-5000000"}},{"warehouse_code":"WH","category":"未分类","kind":"normal","summary":{"count":1,"sum":"105000000","min":"105000000","max":"105000000"}}],"total":{"count":2,"sum":"100000000","min":"-5000000","max":"105000000"},"as_of":"2026-09-10T00:00:00Z","schema":2,"amount_scale":8,"measure":"cost","valuation_source":"sku_master_estimate"}"#;
        let decoded = decode(text.as_bytes(), true, Measure::Cost).unwrap();
        assert_eq!(decoded.report.groups.len(), 2);
        assert_ne!(decoded.report.groups[0].key, decoded.report.groups[1].key);
        assert_eq!(
            format_amount(decoded.report.total.sum, Measure::Cost),
            "1.00"
        );
        assert_eq!(format_amount(-5_000_000, Measure::Cost), "-0.05");
        assert!(decode(text.as_bytes(), false, Measure::Cost).is_err());
        assert_eq!(
            format_amount(9_007_199_254_740_993_000_000, Measure::Cost),
            "90071992547409.93"
        );
    }

    #[test]
    fn decoder_rejects_inconsistent_total_and_invalid_empty_summary() {
        let inconsistent = br#"{"buckets":[{"warehouse_code":"WH","category":null,"kind":"normal","summary":{"count":1,"sum":"5000000","min":"5000000","max":"5000000"}}],"total":{"count":2,"sum":"5000000","min":"5000000","max":"5000000"},"as_of":"x","schema":2,"amount_scale":6,"measure":"quantity","valuation_source":null}"#;
        assert!(decode(inconsistent, false, Measure::Quantity).is_err());
        let invalid =
            br#"{"buckets":[],"total":{"count":0,"sum":"1000000","min":null,"max":null},"as_of":"x","schema":2,"amount_scale":6,"measure":"quantity","valuation_source":null}"#;
        assert!(decode(invalid, false, Measure::Quantity).is_err());
    }

    #[test]
    fn query_change_discards_report_selection_and_cached_groups() {
        let mut view = InventoryReport::default();
        view.expanded.insert(vec!["old-warehouse".into()]);
        view.chart_points.push(("old".into(), 1));
        view.state.summary_rows.push(erp_ui_egui::SummaryRow {
            label: "old".into(),
            cells: BTreeMap::new(),
        });
        view.invalidate(10.0, false);
        assert_eq!(view.deadline, Some(10.25));
        assert!(view.expanded.is_empty());
        assert!(view.chart_points.is_empty());
        assert!(view.state.summary_rows.is_empty());
    }

    #[test]
    fn export_uses_all_leaf_groups_when_tree_is_collapsed_and_one_total() {
        let decoded = decode(
            br#"{"buckets":[{"warehouse_code":"WH","category":"A","kind":"normal","summary":{"count":1,"sum":"2000000","min":"2000000","max":"2000000"}},{"warehouse_code":"WH","category":"B","kind":"normal","summary":{"count":1,"sum":"3000000","min":"3000000","max":"3000000"}}],"total":{"count":2,"sum":"5000000","min":"2000000","max":"3000000"},"as_of":"2026-09-10T00:00:00Z","schema":2,"amount_scale":6,"measure":"quantity","valuation_source":null}"#,
            true,
            Measure::Quantity,
        )
        .unwrap();
        let mut view = InventoryReport {
            labels: decoded.labels,
            visible: vec![0],
            expanded: BTreeSet::new(),
            ..Default::default()
        };
        view.columns
            .push(Column::new(ColumnId(100), "normal", 115.0));
        let (_, rows) = view.export_data(&decoded.report).unwrap();
        assert_eq!(rows.len(), decoded.report.groups.len() + 1);
        assert_eq!(rows[0][0], "WH / A");
        assert_eq!(rows[1][0], "WH / B");
        assert_eq!(rows.last().unwrap(), &["全部结果", "2", "5", "2", "3", "5"]);
    }

    #[test]
    fn export_preserves_dynamic_column_order_and_exact_long_cost() {
        use calamine::{Data, Reader};

        let decoded = decode(
            br#"{"buckets":[{"warehouse_code":"WH","category":null,"kind":"normal","summary":{"count":1,"sum":"9007199254740993000000","min":"9007199254740993000000","max":"9007199254740993000000"}},{"warehouse_code":"WH","category":null,"kind":"reserved","summary":{"count":1,"sum":"7000000","min":"7000000","max":"7000000"}}],"total":{"count":2,"sum":"9007199254741000000000","min":"7000000","max":"9007199254740993000000"},"as_of":"2026-09-10T00:00:00Z","schema":2,"amount_scale":8,"measure":"cost","valuation_source":"sku_master_estimate"}"#,
            false,
            Measure::Cost,
        )
        .unwrap();
        let mut normal = Column::new(ColumnId(100), "normal", 115.0);
        normal.value_kind = ValueKind::Decimal { scale: 8 };
        let mut total = Column::new(ColumnId(2), "合计", 140.0);
        total.value_kind = ValueKind::Decimal { scale: 8 };
        let view = InventoryReport {
            measure: Measure::Cost,
            labels: decoded.labels,
            columns: vec![normal, Column::new(ColumnId(0), "分组", 260.0), total],
            ..Default::default()
        };
        let (columns, rows) = view.export_data(&decoded.report).unwrap();
        assert_eq!(
            columns
                .iter()
                .map(|column| column.title.as_str())
                .collect::<Vec<_>>(),
            ["normal 参考价值(元)", "分组", "参考价值(元)合计"]
        );
        assert_eq!(rows[0][0], "90071992547409.93");
        assert_eq!(rows.last().unwrap()[0], "90071992547409.93");
        assert_eq!(rows.last().unwrap()[2], "90071992547410.00");
        let bytes = crate::export::encode(&columns, &rows, crate::export::Format::Xlsx).unwrap();
        let mut workbook = calamine::Xlsx::new(std::io::Cursor::new(bytes)).unwrap();
        let range = workbook.worksheet_range("Data").unwrap();
        assert_eq!(
            range.get_value((1, 0)),
            Some(&Data::String("90071992547409.93".into()))
        );
        assert_eq!(
            range.get_value((2, 2)),
            Some(&Data::String("90071992547410.00".into()))
        );
    }
}
