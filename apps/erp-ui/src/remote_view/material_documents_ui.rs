//! Formal shared material page. All business lists use the owned DataGrid.
use super::{
    Session,
    material_documents::{MaterialDocuments, ReadOp},
    material_model::{self as model, Command, Draft, Line},
};
use crate::remote_actions::ActionScope;
use eframe::egui;
use erp_grid_core::{ColumnId, RowId};
use erp_ui_egui::DataGrid;
use serde_json::{Value, json};
use uuid::Uuid;

fn row_id(value: &Value, key: &str) -> RowId {
    RowId(
        value[key]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .map_or(0, |id| id.as_u128()),
    )
}
fn text(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value[key].as_i64().map_or("—".into(), |n| n.to_string()))
}
fn action(kind: &str) -> &str {
    match kind {
        "ISSUE" => "领料",
        "SUPPLEMENT" => "补料",
        "RETURN" => "退料",
        "CONSUME" => "耗料",
        _ => "—",
    }
}
impl MaterialDocuments {
    fn editor_ready(&mut self) -> bool {
        match self.commit_editor() {
            Ok(()) => true,
            Err(error) => {
                self.notice = Some(error);
                false
            }
        }
    }
    fn material_name(&self, id: Uuid) -> String {
        self.context
            .as_ref()
            .and_then(|c| c.materials.iter().find(|m| m.sku_id == Some(id)))
            .map(|m| {
                format!(
                    "{} {}",
                    m.component_code,
                    m.sku_name.as_deref().unwrap_or("")
                )
            })
            .unwrap_or("—".into())
    }
    fn warehouse_name(&self, id: Uuid) -> String {
        self.context
            .as_ref()
            .and_then(|c| c.warehouses.iter().find(|w| w.warehouse_id == id))
            .map(|w| format!("{} {}", w.code.as_deref().unwrap_or(""), w.name))
            .unwrap_or("—".into())
    }
    fn line_text(&self, line: &Line, column: ColumnId) -> String {
        if column.0 == 10 {
            return line
                .batch_id
                .map(|id| {
                    self.draft
                        .batch_labels
                        .get(&id)
                        .cloned()
                        .unwrap_or("已绑定批次，待核对名称".into())
                })
                .unwrap_or("—".into());
        }
        match column.0 {
            1 => action(&line.kind).into(),
            2 => self.material_name(line.sku_id),
            3 => self.warehouse_name(line.warehouse_id),
            4 => line.unit_code.clone(),
            5 => line.qty.clone(),
            6 => line.reason.clone().unwrap_or_default(),
            7 => if line.original_issue_id.is_some() {
                "已选择"
            } else {
                "—"
            }
            .into(),
            8 => self
                .draft
                .saved
                .as_ref()
                .and_then(|s| s["effects"].as_array())
                .and_then(|e| e.iter().find(|e| e["line_id"] == json!(line.line_id)))
                .and_then(|e| e["value"]["amount_cent"].as_str())
                .unwrap_or("—")
                .into(),
            9 => self
                .draft
                .saved
                .as_ref()
                .and_then(|s| s["effects"].as_array())
                .and_then(|e| e.iter().find(|e| e["line_id"] == json!(line.line_id)))
                .and_then(|e| e["stock"]["on_hand_micros"].as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .map(model::display)
                .unwrap_or("—".into()),
            _ => String::new(),
        }
    }
    fn report(&mut self, result: Result<(), String>) {
        if let Err(e) = result {
            self.notice = Some(e);
        }
    }
    pub(super) fn ui(&mut self, ui: &mut egui::Ui, session: &Session, scope: &ActionScope) {
        if self.scope.as_ref() != Some(scope) {
            ui.label("请返回原账号处理材料草稿");
            return;
        }
        self.composing = ui.input(|i| {
            i.events
                .iter()
                .fold(self.composing, |old, event| match event {
                    egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => !text.is_empty(),
                    egui::Event::Ime(egui::ImeEvent::Commit(_)) => false,
                    _ => old,
                })
        });
        let editable = self.editable();
        let busy = self.reading.is_some();
        let mut command = None;
        let mut choose = false;
        let mut fresh = false;
        let mut add = false;
        let mut remove = false;
        let mut load = None;
        let mut sources = None;
        let mut review = false;
        let mut latest = false;
        let mut recover = false;
        ui.horizontal(|ui| {
            choose = ui
                .add_enabled(!self.dirty(), egui::Button::new("选择工单"))
                .clicked();
            fresh = ui
                .add_enabled(
                    self.ready() && !self.dirty() && !busy && self.context.is_some(),
                    egui::Button::new("新建"),
                )
                .clicked();
            load = ui
                .add_enabled(!self.dirty() && !busy, egui::Button::new("载入所选单据"))
                .clicked()
                .then_some(self.doc_state.row_selection.active)
                .flatten();
            if ui
                .add_enabled(
                    editable && self.draft.changed,
                    egui::Button::new("保存草稿"),
                )
                .clicked()
            {
                command = Some(Command::Save);
            }
            if ui
                .add_enabled(
                    editable && !self.draft.changed && self.draft.version() > 0,
                    egui::Button::new("整单确认"),
                )
                .clicked()
            {
                command = Some(Command::Confirm);
            }
            review = ui
                .add_enabled(
                    editable && self.draft.version() > 0,
                    egui::Button::new("复核来源"),
                )
                .clicked();
            latest = ui
                .add_enabled(
                    editable && self.draft.version() > 0,
                    egui::Button::new("读取最新版本"),
                )
                .clicked();
            add = ui
                .add_enabled(
                    editable && self.draft.document_id.is_some() && self.draft.lines.len() < 100,
                    egui::Button::new("加行"),
                )
                .clicked();
            remove = ui
                .add_enabled(
                    editable && !self.draft.lines.is_empty(),
                    egui::Button::new("删行"),
                )
                .clicked();
            if self.pending.is_some() {
                recover = ui
                    .add_enabled(self.writing.is_none(), egui::Button::new("查原结果"))
                    .clicked();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("刷新清单").clicked()
                    && !self.dirty()
                    && let Some(c) = &self.context
                {
                    let op = ReadOp::Documents(c.work_order.work_order_id);
                    let result = self.read(op, session, ui.ctx());
                    self.report(result);
                }
            });
            if self.reading.is_some() || self.writing.is_some() {
                ui.spinner();
            }
        });
        if let Some(c) = &self.context {
            ui.horizontal(|ui| {
                ui.label(&c.work_order.work_order_no);
                ui.label(&c.work_order.product_code);
                ui.label(&c.work_order.status);
                if let Some(s) = &self.draft.saved {
                    ui.label(format!(
                        "{} · v{}",
                        if s["posted"] == true {
                            "已确认"
                        } else {
                            "草稿"
                        },
                        self.draft.version()
                    ));
                }
            });
        }
        if let Some(notice) = &self.notice {
            ui.colored_label(ui.visuals().warn_fg_color, notice);
        }
        if self
            .context
            .as_ref()
            .is_some_and(|c| c.needs_reconciliation)
        {
            ui.colored_label(ui.visuals().warn_fg_color, "工单BOM/材料映射尚需核对");
        }
        if self.storage_failed && ui.button("复制恢复记录").clicked() {
            match self
                .checkpoint()
                .and_then(|b| String::from_utf8(b).map_err(|_| "恢复编码错误".into()))
            {
                Ok(s) => ui.ctx().copy_text(s),
                Err(e) => self.notice = Some(e),
            }
        }
        let doc_height = (ui.available_height() * 0.28).clamp(95.0, 230.0);
        let docs = &self.documents;
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), doc_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                DataGrid {
                    id: ui.id().with("material-documents"),
                    columns: &mut self.doc_columns,
                    rows: docs.len(),
                    state: &mut self.doc_state,
                    row_id: Some(&|i| row_id(&docs[i], "document_id")),
                }
                .show(ui, |i, c| match c.0 {
                    1 => if docs[i]["status"] == "POSTED" {
                        "已确认"
                    } else {
                        "草稿"
                    }
                    .into(),
                    2 => text(&docs[i], "version_no"),
                    3 => text(&docs[i], "line_count"),
                    4 => if docs[i]["source_changed"] == true {
                        "待复核"
                    } else {
                        "—"
                    }
                    .into(),
                    5 => text(&docs[i], "updated_at"),
                    _ => String::new(),
                });
            },
        );
        ui.horizontal(|ui| {
            ui.label(format!("共{}单", self.document_total));
            if ui
                .add_enabled(
                    !self.dirty() && !busy && self.doc_offset > 0,
                    egui::Button::new("上一页"),
                )
                .clicked()
            {
                self.doc_offset = self.doc_offset.saturating_sub(500);
                if let Some(c) = &self.context {
                    let r = self.read(
                        ReadOp::Documents(c.work_order.work_order_id),
                        session,
                        ui.ctx(),
                    );
                    self.report(r);
                }
            }
            if ui
                .add_enabled(
                    !self.dirty() && !busy && self.doc_offset + 500 < self.document_total,
                    egui::Button::new("下一页"),
                )
                .clicked()
            {
                self.doc_offset += 500;
                if let Some(c) = &self.context {
                    let r = self.read(
                        ReadOp::Documents(c.work_order.work_order_id),
                        session,
                        ui.ctx(),
                    );
                    self.report(r);
                }
            }
        });
        if let Some(id) = load {
            let result = self.read(ReadOp::Load(Uuid::from_u128(id.0)), session, ui.ctx());
            self.report(result);
        }
        if choose {
            self.order_picker = true;
            self.query_deadline = Some(0.0);
        }
        if fresh && let Some(c) = &self.context {
            self.draft = Draft {
                document_id: Some(Uuid::new_v4()),
                order: Some(c.work_order.clone()),
                acknowledged_source: Some(c.work_order.source()),
                ..Default::default()
            };
            self.line_state.bind_view(0);
            self.review = None;
            self.changed();
        }
        if add && self.editor_ready() {
            let result = self
                .context
                .as_ref()
                .ok_or("材料目录未就绪".to_string())
                .and_then(|c| {
                    Line::new(
                        c.materials
                            .iter()
                            .find(|m| m.direct_component && m.sku_id.is_some())
                            .ok_or("无已映射的直接材料")?,
                        c.warehouses.first().ok_or("无当前授权仓库")?,
                    )
                });
            match result {
                Ok(l) => {
                    self.draft.lines.push(l);
                    self.changed();
                }
                Err(e) => self.notice = Some(e),
            }
        }
        if remove && self.editor_ready() {
            let checked = self.line_state.row_selection.checked();
            let active = self.line_state.row_selection.active;
            self.draft.lines.retain(|l| {
                let id = RowId(l.line_id.as_u128());
                if checked.is_empty() {
                    Some(id) != active
                } else {
                    !checked.contains(&id)
                }
            });
            self.line_state.cancel_edit();
            self.changed();
        }
        let selected = self.line_state.row_selection.active.and_then(|id| {
            self.draft
                .lines
                .iter()
                .position(|l| l.line_id == Uuid::from_u128(id.0))
        });
        if let Some(index) = selected {
            let mut line = self.draft.lines[index].clone();
            let old = line.clone();
            ui.add_enabled_ui(editable, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("material-kind")
                        .selected_text(action(&line.kind))
                        .show_ui(ui, |ui| {
                            for kind in ["ISSUE", "SUPPLEMENT", "RETURN", "CONSUME"] {
                                if ui
                                    .selectable_value(&mut line.kind, kind.into(), action(kind))
                                    .changed()
                                {
                                    line.original_issue_id = None;
                                    line.batch_id = None;
                                }
                            }
                        });
                    egui::ComboBox::from_id_salt("material-sku")
                        .selected_text(self.material_name(line.sku_id))
                        .show_ui(ui, |ui| {
                            if let Some(c) = &self.context {
                                for m in &c.materials {
                                    if let Some(id) = m.sku_id
                                        && m.direct_component
                                        && ui
                                            .selectable_label(
                                                line.sku_id == id,
                                                format!(
                                                    "{} {}",
                                                    m.component_code,
                                                    m.sku_name.as_deref().unwrap_or("")
                                                ),
                                            )
                                            .clicked()
                                    {
                                        line.sku_id = id;
                                        line.unit_code = m.unit_code.clone();
                                        line.original_issue_id = None;
                                        line.batch_id = None;
                                    }
                                }
                            }
                        });
                    egui::ComboBox::from_id_salt("material-warehouse")
                        .selected_text(self.warehouse_name(line.warehouse_id))
                        .show_ui(ui, |ui| {
                            if let Some(c) = &self.context {
                                for w in &c.warehouses {
                                    if ui
                                        .selectable_value(
                                            &mut line.warehouse_id,
                                            w.warehouse_id,
                                            &w.name,
                                        )
                                        .changed()
                                    {
                                        line.original_issue_id = None;
                                        line.batch_id = None;
                                    }
                                }
                            }
                        });
                    if matches!(line.kind.as_str(), "RETURN" | "CONSUME")
                        && ui.button("选择原领料").clicked()
                    {
                        sources = self.doc_state.row_selection.active;
                    }
                    if matches!(line.kind.as_str(), "ISSUE" | "SUPPLEMENT")
                        && ui.button("选择批次").clicked()
                    {
                        self.batch_line = Some(line.line_id);
                        self.batch_offset = 0;
                        self.batch_search.clear();
                        self.batch_page = None;
                        self.batch_deadline = Some(0.0);
                    }
                    if line.batch_id.is_some()
                        && !matches!(line.kind.as_str(), "RETURN" | "CONSUME")
                        && ui
                            .button("清除所选批次")
                            .on_hover_text("批次管理货品确认时仍由服务器要求实际批次")
                            .clicked()
                    {
                        line.batch_id = None;
                    }
                });
            });
            if line != old {
                match self.commit_editor() {
                    Ok(()) => {
                        let current = &mut self.draft.lines[index];
                        current.kind = line.kind;
                        current.sku_id = line.sku_id;
                        current.warehouse_id = line.warehouse_id;
                        current.unit_code = line.unit_code;
                        current.batch_id = line.batch_id;
                        current.original_issue_id = line.original_issue_id;
                        self.changed();
                    }
                    Err(error) => self.notice = Some(error),
                }
            }
        }
        self.line_state.read_only = !editable;
        let cells = self
            .draft
            .lines
            .iter()
            .map(|line| {
                (
                    RowId(line.line_id.as_u128()),
                    (1..=10)
                        .map(|id| self.line_text(line, ColumnId(id)))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let height = (ui.available_height() - 25.0).max(60.0);
        let output = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    DataGrid {
                        id: ui.id().with(("material-lines", self.draft.document_id)),
                        columns: &mut self.line_columns,
                        rows: cells.len(),
                        state: &mut self.line_state,
                        row_id: Some(&|i| cells[i].0),
                    }
                    .show(ui, |i, c| {
                        cells[i]
                            .1
                            .get(c.0.saturating_sub(1) as usize)
                            .cloned()
                            .unwrap_or_default()
                    })
                },
            )
            .inner;
        if let Some(ticket) = output.edit_ticket {
            let result = self.edits(&output.edits);
            self.line_state.resolve_edit(ticket, result.clone());
            self.report(result);
        }
        ui.label(format!("{}行", self.draft.lines.len()));
        if let Some(id) = sources {
            let result = self.read(ReadOp::Sources(Uuid::from_u128(id.0)), session, ui.ctx());
            self.report(result);
        }
        if review {
            let result = self.read(ReadOp::Review, session, ui.ctx());
            self.report(result);
        }
        if latest {
            let result = self.read(ReadOp::Latest, session, ui.ctx());
            self.report(result);
        }
        if recover {
            let result = self.recover(session, ui.ctx());
            self.report(result);
        }
        if let Some(command) = command {
            let result = self.begin(command, None);
            self.report(result);
        }
        self.pickers(ui, session);
    }
    fn pickers(&mut self, ui: &mut egui::Ui, session: &Session) {
        self.batch_picker(ui);
        if let Some(latest) = self.latest.clone() {
            let mut open = true;
            let mut adopt = false;
            egui::Window::new("最新材料版本")
                .open(&mut open)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!(
                        "当前录单版本{} → 服务器版本{}",
                        self.draft.version(),
                        text(&latest, "version_no")
                    ));
                    adopt = ui
                        .add_enabled(
                            self.editable(),
                            egui::Button::new("采用最新版本并保留本地明细"),
                        )
                        .clicked();
                });
            if !open {
                self.latest = None;
            }
            if adopt && self.editor_ready() {
                let result = self.draft.adopt_latest(&latest);
                if result.is_ok() {
                    self.latest = None;
                    self.review = None;
                    self.changed();
                }
                self.report(result);
            }
        }
        if self.order_picker {
            let mut open = true;
            let mut choose = None;
            egui::Window::new("选择工单")
                .open(&mut open)
                .default_size(egui::vec2(700.0, 400.0))
                .show(ui.ctx(), |ui| {
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text("工单号或产品编码")
                                .char_limit(128),
                        )
                        .changed()
                    {
                        self.suspend_reads();
                        self.order_offset = 0;
                        self.query_deadline = Some(ui.input(|i| i.time) + 0.25);
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(250));
                    }
                    let rows = &self.orders;
                    let h = (ui.available_height() - 55.0).max(80.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            DataGrid {
                                id: ui.id().with("work-order-options"),
                                columns: &mut self.order_columns,
                                rows: rows.len(),
                                state: &mut self.order_state,
                                row_id: Some(&|i| RowId(rows[i].work_order_id.as_u128())),
                            }
                            .show(ui, |i, c| match c.0 {
                                1 => rows[i].work_order_no.clone(),
                                2 => rows[i].product_code.clone(),
                                3 => rows[i].status.clone(),
                                4 => rows[i]
                                    .order_qty_micros
                                    .parse::<i64>()
                                    .map(model::display)
                                    .unwrap_or("—".into()),
                                _ => String::new(),
                            });
                        },
                    );
                    ui.horizontal(|ui| {
                        let ready = self.reading.is_none()
                            && self.query_deadline.is_none()
                            && !self.composing;
                        ui.label(format!("共{}条", self.order_total));
                        if ui
                            .add_enabled(
                                ready && self.order_offset > 0,
                                egui::Button::new("上一页"),
                            )
                            .clicked()
                        {
                            self.order_offset = self.order_offset.saturating_sub(500);
                            self.query_deadline = Some(0.0);
                        }
                        if ui
                            .add_enabled(
                                ready && self.order_offset + 500 < self.order_total,
                                egui::Button::new("下一页"),
                            )
                            .clicked()
                        {
                            self.order_offset += 500;
                            self.query_deadline = Some(0.0);
                        }
                        if ui.add_enabled(ready, egui::Button::new("选择")).clicked() {
                            choose = self.order_state.row_selection.active;
                        }
                    });
                });
            if let Some(id) = choose
                && self
                    .orders
                    .iter()
                    .any(|o| o.work_order_id == Uuid::from_u128(id.0))
                && !self.dirty()
            {
                self.draft = Default::default();
                self.context = None;
                self.documents.clear();
                self.review = None;
                self.latest = None;
                self.sources.clear();
                self.source_open = false;
                self.order_picker = false;
                let result = self.read(ReadOp::Context(Uuid::from_u128(id.0)), session, ui.ctx());
                self.report(result);
            } else {
                self.order_picker = open;
            }
        }
        if let Some(review) = self.review.clone() {
            let mut open = true;
            let mut adopt = false;
            egui::Window::new("复核材料来源")
                .open(&mut open)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    for (key, title) in [
                        ("work_order_no", "工单号"),
                        ("product_code", "产品"),
                        ("order_qty_micros", "工单数量（微量）"),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(title);
                            ui.label(text(&review["saved_source"], key));
                            ui.label("→");
                            ui.label(text(&review["current_source"], key));
                        });
                    }
                    if review["saved_source"]["bom_id"] != review["current_source"]["bom_id"] {
                        ui.label("BOM版本已变更");
                    }
                    adopt = ui
                        .add_enabled(self.editable(), egui::Button::new("明确采用当前来源"))
                        .clicked();
                });
            if !open {
                self.review = None;
            }
            if adopt {
                let result = self.begin(Command::Adopt, Some(review));
                self.report(result);
            }
        }
        if self.source_open {
            let mut open = true;
            let mut chosen = None;
            let cells = self
                .sources
                .iter()
                .map(|e| {
                    let sku = Uuid::from_u128(row_id(e, "sku_id").0);
                    let wh = Uuid::from_u128(row_id(e, "warehouse_id").0);
                    (
                        row_id(e, "move_id"),
                        vec![
                            self.material_name(sku),
                            self.warehouse_name(wh),
                            text(e, "unit_code"),
                            e["qty_micros"]
                                .as_str()
                                .and_then(|s| s.parse::<i64>().ok())
                                .map(model::display)
                                .unwrap_or("—".into()),
                        ],
                    )
                })
                .collect::<Vec<_>>();
            egui::Window::new("选择原领料")
                .open(&mut open)
                .default_size(egui::vec2(650.0, 350.0))
                .show(ui.ctx(), |ui| {
                    let height = (ui.available_height() - 36.0).max(80.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            DataGrid {
                                id: ui.id().with("material-original-issues"),
                                columns: &mut self.source_columns,
                                rows: cells.len(),
                                state: &mut self.source_state,
                                row_id: Some(&|i| cells[i].0),
                            }
                            .show(ui, |i, c| {
                                cells[i]
                                    .1
                                    .get(c.0.saturating_sub(1) as usize)
                                    .cloned()
                                    .unwrap_or_default()
                            });
                        },
                    );
                    if ui
                        .add_enabled(self.editable(), egui::Button::new("采用所选原领料"))
                        .clicked()
                    {
                        chosen = self.source_state.row_selection.active;
                    }
                });
            self.source_open = open;
            if let Some(id) = chosen {
                if let Err(error) = self.commit_editor() {
                    self.notice = Some(error);
                    return;
                }
                let result = (|| {
                    let e = self
                        .sources
                        .iter()
                        .find(|e| row_id(e, "move_id") == id)
                        .ok_or("原领料已变化")?;
                    let active = self
                        .line_state
                        .row_selection
                        .active
                        .ok_or("先选择退料或耗料明细")?;
                    let line = self
                        .draft
                        .lines
                        .iter_mut()
                        .find(|l| RowId(l.line_id.as_u128()) == active)
                        .ok_or("材料明细已变化")?;
                    if !matches!(line.kind.as_str(), "RETURN" | "CONSUME")
                        || e["sku_id"] != json!(line.sku_id)
                        || e["warehouse_id"] != json!(line.warehouse_id)
                        || e["unit_code"] != line.unit_code
                    {
                        return Err("原领料与所选材料、仓库或单位不一致".into());
                    }
                    line.original_issue_id = Some(Uuid::from_u128(id.0));
                    line.batch_id = e["batch_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
                    Ok(())
                })();
                if result.is_ok() {
                    self.source_open = false;
                    self.changed();
                }
                self.report(result);
            }
        }
    }
}
