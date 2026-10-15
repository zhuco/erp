//! Material page lifecycle: storage acknowledgement precedes POST and unlock.
use super::{Session, material_model as model};
use crate::{
    action_store::{
        StorageTask,
        order_recovery::{LoadedOrderRecovery, OrderRecoveryStore},
    },
    http::{HttpRequest, HttpTask},
    remote_actions::ActionScope,
};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId};
use erp_ui_egui::GridState;
use model::{Checkpoint, Command, Context, Draft, Prepared, Rejection, WorkOrder};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) enum ReadOp {
    Orders,
    Context(Uuid),
    Documents(Uuid),
    Load(Uuid),
    Review,
    Latest,
    Sources(Uuid),
    Batches(Uuid, Uuid, Uuid, u64),
}
enum AfterStorage {
    Draft,
    Send,
    Complete,
    Discard,
}
pub(super) struct MaterialDocuments {
    pub draft: Draft,
    pub pending: Option<Prepared>,
    finished: Option<Draft>,
    finish_rejection: Option<Rejection>,
    last_rejection: Option<Rejection>,
    pub scope: Option<ActionScope>,
    store: Option<OrderRecoveryStore>,
    opening: Option<StorageTask<LoadedOrderRecovery>>,
    storing: Option<(u64, AfterStorage, StorageTask<u64>)>,
    revision: u64,
    serial: u64,
    needs_store: bool,
    pub storage_failed: bool,
    pub writing: Option<HttpTask>,
    pub reading: Option<(ReadOp, u64, HttpTask)>,
    generation: u64,
    pub context: Option<Context>,
    pub orders: Vec<WorkOrder>,
    pub documents: Vec<Value>,
    pub document_total: u64,
    pub review: Option<Value>,
    pub latest: Option<Value>,
    pub sources: Vec<Value>,
    pub source_open: bool,
    pub batch_line: Option<Uuid>,
    pub batch_page: Option<model::BatchPage>,
    pub batch_search: String,
    pub batch_offset: u64,
    pub batch_deadline: Option<f64>,
    pub batch_state: GridState,
    pub batch_columns: Vec<Column>,
    pub search: String,
    pub order_offset: u64,
    pub order_total: u64,
    pub doc_offset: u64,
    pub order_picker: bool,
    pub query_deadline: Option<f64>,
    pub composing: bool,
    pub notice: Option<String>,
    pub order_state: GridState,
    pub doc_state: GridState,
    pub line_state: GridState,
    pub source_state: GridState,
    pub order_columns: Vec<Column>,
    pub doc_columns: Vec<Column>,
    pub line_columns: Vec<Column>,
    pub source_columns: Vec<Column>,
}
fn columns(spec: &[(u64, &str, f32)]) -> Vec<Column> {
    spec.iter()
        .map(|(id, name, width)| {
            let mut c = Column::new(ColumnId(*id), *name, *width);
            c.filter = erp_grid_core::FilterKind::None;
            c
        })
        .collect()
}
fn state() -> GridState {
    let mut s = GridState::default();
    s.metrics = erp_ui_egui::GridMetrics::legacy();
    s.theme = Some(crate::erp_theme::grid());
    s.read_only = true;
    s.sorting_enabled = false;
    s
}
impl Default for MaterialDocuments {
    fn default() -> Self {
        let mut lines = columns(&[
            (1, "动作", 80.0),
            (2, "材料", 180.0),
            (3, "仓库", 150.0),
            (4, "单位", 70.0),
            (5, "数量", 120.0),
            (6, "原因", 180.0),
            (7, "原领料", 130.0),
            (8, "原金额（分）", 130.0),
            (9, "原库存结果", 140.0),
            (10, "批次", 170.0),
        ]);
        for c in &mut lines {
            c.edit_kind = if matches!(c.id.0, 5 | 6) {
                erp_grid_core::EditKind::Text
            } else {
                erp_grid_core::EditKind::ReadOnly
            };
            if c.id.0 == 5 {
                c.value_kind = erp_grid_core::ValueKind::Decimal { scale: 6 };
            }
        }
        Self {
            draft: Default::default(),
            pending: None,
            finished: None,
            finish_rejection: None,
            last_rejection: None,
            scope: None,
            store: None,
            opening: None,
            storing: None,
            revision: 0,
            serial: 0,
            needs_store: false,
            storage_failed: false,
            writing: None,
            reading: None,
            generation: 0,
            context: None,
            orders: vec![],
            documents: vec![],
            document_total: 0,
            review: None,
            latest: None,
            sources: vec![],
            source_open: false,
            batch_line: None,
            batch_page: None,
            batch_search: String::new(),
            batch_offset: 0,
            batch_deadline: None,
            batch_state: state(),
            batch_columns: columns(&[
                (1, "批号", 190.0),
                (2, "状态", 110.0),
                (3, "到期时间", 220.0),
                (4, "实物", 120.0),
                (5, "预占", 120.0),
                (6, "可用", 120.0),
            ]),
            search: String::new(),
            order_offset: 0,
            order_total: 0,
            doc_offset: 0,
            order_picker: false,
            query_deadline: None,
            composing: false,
            notice: None,
            order_state: state(),
            doc_state: state(),
            line_state: state(),
            source_state: state(),
            order_columns: columns(&[
                (1, "工单号", 190.0),
                (2, "产品", 170.0),
                (3, "状态", 95.0),
                (4, "工单数量", 150.0),
            ]),
            doc_columns: columns(&[
                (1, "状态", 90.0),
                (2, "版本", 70.0),
                (3, "明细数", 80.0),
                (4, "来源变更", 100.0),
                (5, "更新时间", 220.0),
            ]),
            line_columns: lines,
            source_columns: columns(&[
                (1, "材料", 180.0),
                (2, "仓库", 150.0),
                (3, "单位", 70.0),
                (4, "原领数量", 130.0),
            ]),
        }
    }
}
impl MaterialDocuments {
    pub fn dirty(&self) -> bool {
        self.draft.changed
            || self.pending.is_some()
            || self.finished.is_some()
            || self.storing.is_some()
            || self.line_state.is_editing()
    }
    pub fn blocked(&self) -> Option<&'static str> {
        if self.pending.is_some() {
            Some("材料原请求结果未确认，不能丢弃或重复提交")
        } else if self.storing.is_some() {
            Some("材料恢复记录正在提交，请等待")
        } else {
            None
        }
    }
    pub fn editable(&self) -> bool {
        self.ready() && !self.draft.posted()
    }
    pub fn ready(&self) -> bool {
        self.store.is_some()
            && !self.storage_failed
            && self.storing.is_none()
            && self.pending.is_none()
            && self.finished.is_none()
    }
    pub fn hold(&mut self) {
        self.line_state.hold_edit();
    }
    pub fn suspend_reads(&mut self) {
        self.reading = None;
        self.query_deadline = None;
        self.batch_deadline = None;
        self.generation = self.generation.wrapping_add(1);
    }
    pub fn refresh(&mut self) {
        self.suspend_reads();
        self.query_deadline = Some(0.0);
    }
    pub fn changed(&mut self) {
        let batch_ids = self
            .draft
            .lines
            .iter()
            .filter_map(|l| l.batch_id)
            .collect::<std::collections::BTreeSet<_>>();
        self.draft
            .batch_labels
            .retain(|id, _| batch_ids.contains(id));
        self.notice = None;
        self.draft.changed = true;
        self.serial = self.serial.wrapping_add(1);
        self.needs_store = true;
    }
    pub fn discard(&mut self, ctx: &egui::Context) -> Result<bool, String> {
        if self.pending.is_some() {
            return Err("原材料请求仍未确认，不能丢弃".into());
        }
        if self.storing.is_some() {
            return Ok(false);
        }
        if !self.dirty() {
            return Ok(true);
        }
        if self.store.is_none() || self.storage_failed {
            return Err("材料恢复存储未就绪，不能确认清除".into());
        }
        self.draft = Default::default();
        self.line_state.cancel_edit();
        self.serial = self.serial.wrapping_add(1);
        self.needs_store = true;
        self.start_storage(ctx)?;
        Ok(false)
    }
    pub fn commit_editor(&mut self) -> Result<(), String> {
        if let Some(edit) = self.line_state.edit_draft() {
            let col = self
                .line_columns
                .iter()
                .find(|c| c.id == edit.column)
                .cloned()
                .ok_or("编辑列已变化")?;
            if let Some((edit, ticket)) = self.line_state.prepare_edit_commit(&col)? {
                let result = self.edits(&[edit]);
                self.line_state.resolve_edit(ticket, result.clone());
                result?;
            }
        }
        Ok(())
    }
    pub fn edits(&mut self, edits: &[erp_grid_core::CellEdit]) -> Result<(), String> {
        if !self.editable() {
            return Err("材料输入已冻结".into());
        }
        let mut candidate = self.draft.clone();
        for edit in edits {
            let line = candidate
                .lines
                .iter_mut()
                .find(|l| RowId(l.line_id.as_u128()) == edit.row)
                .ok_or("材料行已变化")?;
            let before = match edit.column.0 {
                5 => line.qty.as_str(),
                6 => line.reason.as_deref().unwrap_or(""),
                _ => return Err("请使用真实对象选择器修改此字段".into()),
            };
            if before != edit.before {
                return Err("材料行版本已变化".into());
            }
            match edit.column.0 {
                5 => {
                    model::quantity(&edit.after)?;
                    line.qty = edit.after.clone();
                }
                6 => {
                    if edit.after.len() > 4096 {
                        return Err("原因长度超限".into());
                    }
                    line.reason =
                        (!edit.after.trim().is_empty()).then(|| edit.after.trim().to_owned());
                }
                _ => {}
            }
        }
        self.draft = candidate;
        self.changed();
        Ok(())
    }
    pub fn save_for_leave(&mut self) -> Result<bool, String> {
        if let Some(e) = self.blocked() {
            return Err(e.into());
        }
        self.commit_editor()?;
        if !self.draft.changed {
            return Ok(true);
        }
        self.begin(Command::Save, None)?;
        Ok(false)
    }
    pub fn begin(&mut self, command: Command, review: Option<Value>) -> Result<(), String> {
        if !self.editable() {
            return Err("材料写入或恢复存储尚未就绪".into());
        }
        self.commit_editor()?;
        let scope = self.scope.as_ref().ok_or("材料账号归属未就绪")?;
        self.pending = Some(self.draft.prepare(command, scope, review)?);
        self.latest = None;
        self.review = None;
        self.source_open = false;
        self.batch_line = None;
        self.batch_page = None;
        self.serial = self.serial.wrapping_add(1);
        self.needs_store = true;
        self.notice = None;
        self.suspend_reads();
        Ok(())
    }
    pub fn checkpoint(&self) -> Result<Vec<u8>, String> {
        let value = Checkpoint {
            schema: 1,
            scope: self.scope.clone().ok_or("材料账号未就绪")?,
            draft: self.finished.as_ref().unwrap_or(&self.draft).clone(),
            pending: if self.finished.is_some() {
                None
            } else {
                self.pending.clone()
            },
            rejected: self
                .finish_rejection
                .as_ref()
                .or(self.last_rejection.as_ref())
                .cloned(),
        };
        let bytes = serde_json::to_vec(&value).map_err(|_| "材料恢复编码失败")?;
        if bytes.len() > model::MAX_BYTES {
            return Err("材料恢复记录超过1MiB".into());
        }
        Ok(bytes)
    }
    fn start_storage(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let after = if self.finished.is_some() {
            AfterStorage::Complete
        } else if self.pending.is_some() && self.writing.is_none() {
            AfterStorage::Send
        } else if !self.draft.changed {
            AfterStorage::Discard
        } else {
            AfterStorage::Draft
        };
        let task = self.store.as_ref().ok_or("恢复存储未就绪")?.commit(
            self.revision,
            Some(&self.checkpoint()?),
            ctx,
        )?;
        self.storing = Some((self.serial, after, task));
        self.needs_store = false;
        Ok(())
    }
    pub fn root(&self, session: &Session) -> String {
        format!("/v1/tenants/{}/manufacturing", session.tenant.to_uuid())
    }
    fn request(
        &self,
        session: &Session,
        path: String,
        query: Vec<(String, String)>,
        body: Option<Value>,
        ctx: &egui::Context,
    ) -> Result<HttpTask, String> {
        let key = body
            .as_ref()
            .and_then(|v| v["request_id"].as_str())
            .map(str::to_owned);
        session
            .client
            .start(
                HttpRequest {
                    path,
                    query,
                    json: body
                        .map(|v| serde_json::to_vec(&v))
                        .transpose()
                        .map_err(|_| "材料请求编码失败")?,
                    bearer: Some(session.token.clone()),
                    idempotency_key: key,
                },
                ctx,
            )
            .map_err(|e| e.to_string())
    }
    fn send(&mut self, session: &Session, ctx: &egui::Context) -> Result<(), String> {
        let p = self.pending.as_ref().ok_or("原材料请求丢失")?;
        let order = p.context["work_order_id"].as_str().ok_or("原工单丢失")?;
        let document = p.context["document_id"].as_str().ok_or("原材料单丢失")?;
        let path = match p.command {
            Command::Save => format!("work-orders/{order}/material-documents"),
            Command::Adopt => {
                format!("work-orders/{order}/material-documents/{document}/source-adoptions")
            }
            Command::Confirm => {
                format!("work-orders/{order}/material-documents/{document}/confirm")
            }
        };
        self.writing = Some(self.request(
            session,
            format!("{}/{path}", self.root(session)),
            vec![("rejection_envelope".into(), "true".into())],
            Some(p.body.clone()),
            ctx,
        )?);
        Ok(())
    }
    pub fn recover(&mut self, session: &Session, ctx: &egui::Context) -> Result<(), String> {
        if self.writing.is_some() || self.storing.is_some() {
            return Err("原材料结果仍在读取或持久化".into());
        }
        let p = self.pending.as_ref().ok_or("无未决材料请求")?;
        self.writing = Some(self.request(
            session,
            format!(
                "{}/work-orders/{}/material-documents/{}",
                self.root(session),
                p.context["work_order_id"].as_str().ok_or("原工单丢失")?,
                p.context["document_id"].as_str().ok_or("原材料单丢失")?
            ),
            vec![("request_id".into(), p.request.clone())],
            None,
            ctx,
        )?);
        Ok(())
    }
    pub fn read(
        &mut self,
        operation: ReadOp,
        session: &Session,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        self.reading = None;
        self.generation = self.generation.wrapping_add(1);
        let (path, query) =
            match &operation {
                ReadOp::Orders => (
                    "work-order-options".into(),
                    vec![
                        ("query".into(), self.search.clone()),
                        ("open_only".into(), "false".into()),
                        ("offset".into(), self.order_offset.to_string()),
                        ("limit".into(), "500".into()),
                    ],
                ),
                ReadOp::Context(id) => (format!("work-orders/{id}/material-context"), vec![]),
                ReadOp::Batches(line, warehouse, sku, offset) => {
                    let order = self
                        .draft
                        .order
                        .as_ref()
                        .ok_or("真实工单未就绪")?
                        .work_order_id;
                    if !self.draft.lines.iter().any(|l| {
                        l.line_id == *line && l.warehouse_id == *warehouse && l.sku_id == *sku
                    }) {
                        return Err("材料行已经变化".into());
                    }
                    (
                        format!("work-orders/{order}/material-batch-options"),
                        vec![
                            ("warehouse_id".into(), warehouse.to_string()),
                            ("sku_id".into(), sku.to_string()),
                            ("query".into(), self.batch_search.clone()),
                            ("offset".into(), offset.to_string()),
                            ("limit".into(), "500".into()),
                        ],
                    )
                }
                ReadOp::Documents(id) => (
                    format!("work-orders/{id}/material-documents"),
                    vec![
                        ("offset".into(), self.doc_offset.to_string()),
                        ("limit".into(), "500".into()),
                    ],
                ),
                ReadOp::Load(id) | ReadOp::Sources(id) => {
                    let order = self
                        .context
                        .as_ref()
                        .ok_or("工单未就绪")?
                        .work_order
                        .work_order_id;
                    (
                        format!("work-orders/{order}/material-documents/{id}"),
                        vec![],
                    )
                }
                ReadOp::Review | ReadOp::Latest => {
                    let o = self.draft.order.as_ref().ok_or("工单未就绪")?.work_order_id;
                    let d = self.draft.document_id.ok_or("材料单未就绪")?;
                    (
                        format!(
                            "work-orders/{o}/material-documents/{d}{}",
                            if matches!(operation, ReadOp::Review) {
                                "/source-review"
                            } else {
                                ""
                            }
                        ),
                        vec![],
                    )
                }
            };
        self.reading = Some((
            operation,
            self.generation,
            self.request(
                session,
                format!("{}/{path}", self.root(session)),
                query,
                None,
                ctx,
            )?,
        ));
        Ok(())
    }
    fn read_result(
        &mut self,
        op: ReadOp,
        value: Value,
        session: &Session,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        match op {
            ReadOp::Orders => {
                let rows: Vec<WorkOrder> = serde_json::from_value(value["data"].clone())
                    .map_err(|_| "工单目录格式错误")?;
                if rows.len() > 500 {
                    return Err("工单页超限".into());
                }
                self.order_total = value["total"].as_u64().ok_or("工单总数缺失")?;
                self.orders = rows;
                self.order_state.rebind_page(self.generation, false);
            }
            ReadOp::Context(order) => {
                // Context replacement cannot retain another material line's batch quote.
                self.batch_line = None;
                self.batch_page = None;
                let c: Context = serde_json::from_value(value).map_err(|_| "材料目录格式错误")?;
                if c.work_order.work_order_id != order
                    || c.materials.len() > 1000
                    || c.warehouses.len() > 1000
                {
                    return Err("材料目录归属或预算错误".into());
                }
                self.context = Some(c);
                self.doc_offset = 0;
                self.read(ReadOp::Documents(order), session, ctx)?;
            }
            ReadOp::Documents(order) => {
                if self
                    .context
                    .as_ref()
                    .is_none_or(|c| c.work_order.work_order_id != order)
                {
                    return Ok(());
                }
                let rows = value["data"]
                    .as_array()
                    .filter(|r| r.len() <= 500)
                    .ok_or("材料单页格式错误")?;
                if rows.iter().any(|r| r["work_order_id"] != json!(order)) {
                    return Err("材料清单工单不一致".into());
                }
                self.document_total = value["total"].as_u64().ok_or("材料清单总数缺失")?;
                self.documents = rows.clone();
                self.doc_state.rebind_page(self.generation, false);
            }
            ReadOp::Load(id) => {
                if self.dirty() {
                    return Err("保留当前材料草稿后再载入另一单据".into());
                }
                let order = self
                    .context
                    .as_ref()
                    .ok_or("工单未就绪")?
                    .work_order
                    .clone();
                let d = Draft::from_snapshot(&value["snapshot"], order)?;
                if d.document_id != Some(id) {
                    return Err("所选材料单身份不一致".into());
                }
                self.draft = d;
                self.review = None;
                self.latest = None;
                self.serial = self.serial.wrapping_add(1);
                self.needs_store = true;
                self.line_state.bind_view(self.generation);
            }
            ReadOp::Review => {
                self.draft.validate_review(&value)?;
                self.review = Some(value);
            }
            ReadOp::Batches(line, warehouse, sku, offset) => {
                let current = self
                    .draft
                    .lines
                    .iter()
                    .find(|l| l.line_id == line && l.warehouse_id == warehouse && l.sku_id == sku)
                    .ok_or("所选材料行已经变化")?;
                if self.batch_line != Some(line)
                    || self.batch_offset != offset
                    || self.pending.is_some()
                {
                    return Ok(());
                }
                let page: model::BatchPage =
                    serde_json::from_value(value).map_err(|_| "批次响应格式无效")?;
                page.validate(
                    self.draft.order.as_ref().ok_or("工单未就绪")?.work_order_id,
                    current,
                    offset,
                )?;
                for row in &page.data {
                    if let Some(name) = &row.batch_no
                        && current.batch_id == Some(row.batch_id)
                    {
                        self.draft.batch_labels.insert(row.batch_id, name.clone());
                    }
                }
                self.batch_page = Some(page);
                self.batch_state.rebind_page(self.generation, false);
            }
            ReadOp::Latest => {
                if self.pending.is_some() {
                    return Err("最新状态不能替代原未决请求结果".into());
                }
                let mut probe = self.draft.clone();
                probe.adopt_latest(&value["snapshot"])?;
                self.latest = Some(value["snapshot"].clone());
            }
            ReadOp::Sources(id) => {
                let s = &value["snapshot"];
                let order = self
                    .context
                    .as_ref()
                    .ok_or("工单未就绪")?
                    .work_order
                    .clone();
                let d = Draft::from_snapshot(s, order)?;
                if d.document_id != Some(id) || !d.posted() {
                    return Err("原领料须来自实际已确认材料单".into());
                }
                self.sources = s["effects"]
                    .as_array()
                    .ok_or("原领料效果缺失")?
                    .iter()
                    .filter(|e| matches!(e["kind"].as_str(), Some("ISSUE" | "SUPPLEMENT")))
                    .cloned()
                    .collect();
                self.source_open = true;
                self.source_state.bind_view(self.generation);
            }
        }
        Ok(())
    }
    fn stage(&mut self, response: &Value) -> Result<(), String> {
        let prepared = self.pending.as_ref().ok_or("原材料请求丢失")?;
        if let Some(rejection) = model::rejected(prepared, response)? {
            self.finish_rejection = Some(rejection);
            self.finished = Some(self.draft.clone());
        } else {
            self.finished = Some(model::matched(&self.draft, prepared, response)?);
            self.finish_rejection = None;
        }
        self.needs_store = true;
        Ok(())
    }
    pub fn tick(&mut self, ctx: &egui::Context, session: &Session, scope: ActionScope) {
        if self.scope.as_ref() != Some(&scope) {
            if self.dirty() {
                self.notice = Some("另一账号的材料草稿仍需原账号处理".into());
                return;
            }
            *self = Self::default();
            self.scope = Some(scope.clone());
        }
        if self.store.is_none() && self.opening.is_none() && !self.storage_failed {
            match OrderRecoveryStore::open_material(scope.clone(), ctx) {
                Ok(t) => self.opening = Some(t),
                Err(e) => {
                    self.notice = Some(e);
                    self.storage_failed = true;
                }
            }
        }
        if let Some(result) = self.opening.as_mut().and_then(StorageTask::poll) {
            self.opening = None;
            let result = result.and_then(|loaded| {
                if let Some(bytes) = loaded.bytes.as_deref() {
                    let record = Checkpoint::decode(bytes, &scope)?;
                    self.draft = record.draft;
                    self.pending = record.pending;
                    self.last_rejection = record.rejected;
                }
                self.store = Some(loaded.store);
                self.revision = loaded.revision;
                Ok(())
            });
            match result {
                Ok(()) => {
                    if self.pending.is_some()
                        && let Err(e) = self.recover(session, ctx)
                    {
                        self.notice = Some(e);
                    }
                    if let Some(order) = self.draft.order.as_ref()
                        && let Err(e) =
                            self.read(ReadOp::Context(order.work_order_id), session, ctx)
                    {
                        self.notice = Some(e);
                    }
                }
                Err(e) => {
                    self.notice = Some(e);
                    self.storage_failed = true;
                }
            }
        }
        if let Some(result) = self.storing.as_mut().and_then(|(_, _, t)| t.poll())
            && let Some((serial, after, _)) = self.storing.take()
        {
            match result {
                Ok(revision) => {
                    self.revision = revision;
                    self.needs_store |= serial != self.serial;
                    match after {
                        AfterStorage::Send => {
                            if let Err(e) = self.send(session, ctx) {
                                self.notice = Some(e);
                            }
                        }
                        AfterStorage::Complete => {
                            if let Some(draft) = self.finished.take() {
                                self.draft = draft;
                            }
                            self.pending = None;
                            self.review = None;
                            self.notice = if let Some(r) = self.finish_rejection.take() {
                                let message = format!("拒绝已记录，输入保留：{}", r.message);
                                self.last_rejection = Some(r);
                                Some(message)
                            } else {
                                Some(
                                    if self.draft.posted() {
                                        "整单已确认"
                                    } else if self.draft.changed {
                                        "来源已采用，本地明细仍待保存"
                                    } else {
                                        "草稿已保存"
                                    }
                                    .into(),
                                )
                            };
                            if let Some(context) = &self.context
                                && let Err(error) = self.read(
                                    ReadOp::Documents(context.work_order.work_order_id),
                                    session,
                                    ctx,
                                )
                            {
                                self.notice = Some(error);
                            }
                        }
                        AfterStorage::Draft | AfterStorage::Discard => {}
                    }
                }
                Err(e) => {
                    self.storage_failed = true;
                    self.notice = Some(e);
                }
            }
        }
        if let Some(result) = self.writing.as_mut().and_then(HttpTask::poll) {
            self.writing = None;
            let result = result
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<Value>(&bytes)
                        .map_err(|_| "材料写入结果格式错误".into())
                })
                .and_then(|r| self.stage(&r));
            if let Err(e) = result {
                self.notice = Some(format!("原请求结果未确认：{e}"));
            }
        }
        if let Some(result) = self.reading.as_mut().and_then(|(_, _, t)| t.poll())
            && let Some((op, generation, _)) = self.reading.take()
        {
            let result = result.map_err(|e| e.to_string()).and_then(|b| {
                serde_json::from_slice::<Value>(&b).map_err(|_| "材料查询结果格式错误".into())
            });
            if generation == self.generation {
                match result.and_then(|v| self.read_result(op, v, session, ctx)) {
                    Ok(()) => {}
                    Err(e) => self.notice = Some(e),
                }
            }
        }
        if self.needs_store
            && self.storing.is_none()
            && self.store.is_some()
            && !self.storage_failed
            && let Err(e) = self.start_storage(ctx)
        {
            self.storage_failed = true;
            self.notice = Some(e);
        }
        if self.order_picker
            && !self.composing
            && self
                .query_deadline
                .is_some_and(|d| ctx.input(|i| i.time) >= d)
        {
            self.query_deadline = None;
            if let Err(e) = self.read(ReadOp::Orders, session, ctx) {
                self.notice = Some(e);
            }
        }
        if self.batch_line.is_some()
            && !self.composing
            && self
                .batch_deadline
                .is_some_and(|d| ctx.input(|i| i.time) >= d)
        {
            self.batch_deadline = None;
            if let Some(line) = self
                .draft
                .lines
                .iter()
                .find(|l| Some(l.line_id) == self.batch_line)
            {
                let operation = ReadOp::Batches(
                    line.line_id,
                    line.warehouse_id,
                    line.sku_id,
                    self.batch_offset,
                );
                if let Err(error) = self.read(operation, session, ctx) {
                    self.notice = Some(error);
                }
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    async fn wait(task: &mut StorageTask<u64>) -> Result<u64, String> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(r) = task.poll() {
                return r;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("owned storage task deadline".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }
    #[tokio::test]
    async fn material_ui_rejection_unlocks_only_after_actual_sqlite_ack() -> Result<(), String> {
        let root = tempfile::tempdir().map_err(|e| e.to_string())?;
        let (draft, scope) = super::super::material_tests::fixture()?;
        let loaded = OrderRecoveryStore::open_material_test(scope.clone(), root.path()).await?;
        let p = draft.prepare(Command::Save, &scope, None)?;
        let mut page = MaterialDocuments {
            draft,
            pending: Some(p.clone()),
            scope: Some(scope.clone()),
            store: Some(loaded.store.clone()),
            revision: loaded.revision,
            ..Default::default()
        };
        let response = json!({"code":"material_document_rejected_before_commit","committed":false,"request_id":p.request,"fields":p.body,"context":p.context,"message":"owned rejection"});
        page.stage(&response)?;
        assert!(page.pending.is_some());
        assert!(!page.editable());
        let ctx = egui::Context::default();
        page.start_storage(&ctx)?;
        assert!(page.pending.is_some());
        let (_, _, task) = page.storing.as_mut().ok_or("storage")?;
        // Observe via the page tick so storage completion, not an in-memory assignment, unlocks.
        let session = Session {
            client: crate::http::HttpClient::new("https://owned-material.qa.invalid")
                .map_err(|e| e.to_string())?,
            tenant: RowId(1),
            token: "not persisted".into(),
            expires: web_time::Instant::now() + std::time::Duration::from_secs(60),
        };
        let _ = task;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while page.pending.is_some() {
            page.tick(&ctx, &session, scope.clone());
            if tokio::time::Instant::now() >= deadline {
                return Err("storage ack deadline".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(page.editable());
        assert_eq!(page.draft.lines[0].qty, "0.250000");
        let reopened = OrderRecoveryStore::open_material_test(scope.clone(), root.path()).await?;
        let record = Checkpoint::decode(reopened.bytes.as_deref().ok_or("stored record")?, &scope)?;
        assert!(record.pending.is_none());
        assert_eq!(
            record
                .rejected
                .as_ref()
                .ok_or("rejection")?
                .original
                .request,
            p.request
        );
        loaded.store.close_test().await;
        reopened.store.close_test().await;
        Ok(())
    }
    #[tokio::test]
    async fn material_ui_sqlite_reopen_preserves_pending_and_stale_writer_cannot_replace()
    -> Result<(), String> {
        let root = tempfile::tempdir().map_err(|e| e.to_string())?;
        let (draft, scope) = super::super::material_tests::fixture()?;
        let a = OrderRecoveryStore::open_material_test(scope.clone(), root.path()).await?;
        let b = OrderRecoveryStore::open_material_test(scope.clone(), root.path()).await?;
        let pending = draft.prepare(Command::Save, &scope, None)?;
        let record = Checkpoint {
            schema: 1,
            scope: scope.clone(),
            draft,
            pending: Some(pending.clone()),
            rejected: None,
        };
        let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
        let ctx = egui::Context::default();
        let mut first = a.store.commit(a.revision, Some(&bytes), &ctx)?;
        assert_eq!(wait(&mut first).await?, 1);
        let mut stale = b.store.commit(b.revision, None, &ctx)?;
        assert!(wait(&mut stale).await.is_err());
        let reopened = OrderRecoveryStore::open_material_test(scope.clone(), root.path()).await?;
        let recovered = Checkpoint::decode(reopened.bytes.as_deref().ok_or("record")?, &scope)?;
        assert_eq!(
            recovered.pending.as_ref().ok_or("pending")?.body,
            pending.body
        );
        assert_eq!(reopened.revision, 1);
        a.store.close_test().await;
        b.store.close_test().await;
        reopened.store.close_test().await;
        Ok(())
    }
}
