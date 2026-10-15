//! Shared native/Web trade editor; HTTP sends only after durable checkpoint commit.
use super::Session;
use crate::{
    action_store::{
        StorageTask,
        order_recovery::{LoadedOrderRecovery, OrderRecoveryStore},
    },
    http::{HttpRequest, HttpTask},
    remote_actions::ActionScope,
};
use eframe::egui;
use erp_grid_core::{
    CellEdit, Column, ColumnId, EditKind, FilterKind, RowId, ValueKind, compare_values,
};
use erp_ui_egui::{DataGrid, GridState};
use serde_json::Value;

use super::trade_sales_model as model;
use model::{Checkpoint, Draft, Line, Prepared, Rejection};

enum AfterStorage {
    Draft,
    Send,
    Complete,
    Discard,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn trade_sales_rejection_unlocks_editing_only_after_real_storage_ack()
    -> Result<(), String> {
        let root = tempfile::tempdir().map_err(|_| "test directory")?;
        let client = crate::http::HttpClient::new("https://owned-trade.qa.invalid")
            .map_err(|e| e.to_string())?;
        let scope = ActionScope::new(&client, RowId(1), RowId(2))?;
        let loaded = OrderRecoveryStore::open_trade_test(scope.clone(), root.path()).await?;
        let mut trade = TradeSales {
            scope: Some(scope.clone()),
            store: Some(loaded.store.clone()),
            revision: loaded.revision,
            ..Default::default()
        };
        trade.draft.number = "SO-ACK".into();
        trade.draft.customer = "C-1".into();
        trade.draft.lines[0].sku = "S-1".into();
        trade.draft.lines[0].address = "Dock".into();
        trade.draft.changed = true;
        let request = uuid::Uuid::new_v4().to_string();
        let prepared = Prepared {
            body: trade.draft.payload(&request)?,
            request,
        };
        trade.pending = Some(prepared.clone());
        let reply = serde_json::json!({"code":"trade_document_rejected_before_commit","committed":false,"request_id":prepared.request,"fields":prepared.body,"message":"customer unavailable"});
        trade.stage_command_response(&reply)?;
        assert!(trade.pending.is_some());
        assert!(trade.blocked().is_some());
        let bytes = trade.checkpoint()?;
        let staged = Checkpoint::decode(&bytes, &scope)?;
        assert!(staged.pending.is_none());
        assert!(staged.rejected.is_some());
        let ctx = egui::Context::default();
        let mut write = loaded.store.commit(loaded.revision, Some(&bytes), &ctx)?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let revision = loop {
            if let Some(result) = write.poll() {
                break result?;
            }
            if tokio::time::Instant::now() > deadline {
                return Err("storage acknowledgement timeout".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        };
        trade.revision = revision;
        trade.finish_command_storage();
        assert!(trade.pending.is_none());
        assert!(trade.blocked().is_none());
        assert!(trade.draft.changed);
        assert_eq!(trade.draft.customer, "C-1");
        assert!(trade.last_rejection.is_some());
        let restored = OrderRecoveryStore::open_trade_test(scope.clone(), root.path()).await?;
        let checkpoint = Checkpoint::decode(
            restored.bytes.as_deref().ok_or("checkpoint missing")?,
            &scope,
        )?;
        assert!(checkpoint.pending.is_none());
        assert_eq!(
            checkpoint
                .rejected
                .ok_or("rejection missing")?
                .original
                .request,
            prepared.request
        );
        loaded.store.close_test().await;
        restored.store.close_test().await;
        Ok(())
    }
    #[test]
    fn trade_sales_unbound_rejection_preserves_pending_request() -> Result<(), String> {
        let mut trade = TradeSales::default();
        trade.draft.number = "SO-1".into();
        trade.draft.customer = "C-1".into();
        trade.draft.lines[0].sku = "S-1".into();
        trade.draft.lines[0].address = "Dock".into();
        let request = uuid::Uuid::new_v4().to_string();
        let prepared = Prepared {
            body: trade.draft.payload(&request)?,
            request,
        };
        trade.pending = Some(prepared.clone());
        let reply = serde_json::json!({"code":"trade_document_rejected_before_commit","committed":false,"request_id":uuid::Uuid::new_v4(),"fields":prepared.body,"message":"wrong identity"});
        assert!(trade.stage_command_response(&reply).is_err());
        assert_eq!(
            trade.pending.as_ref().ok_or("pending missing")?.request,
            prepared.request
        );
        assert!(trade.finished.is_none());
        assert!(trade.blocked().is_some());
        Ok(())
    }
}
enum Network {
    Save,
    Recover,
    Lookup(u64),
}
pub(super) struct TradeSales {
    draft: Draft,
    pending: Option<Prepared>,
    finished: Option<Draft>,
    finish_rejection: Option<Rejection>,
    last_rejection: Option<Rejection>,
    scope: Option<ActionScope>,
    store: Option<OrderRecoveryStore>,
    opening: Option<StorageTask<LoadedOrderRecovery>>,
    storing: Option<(u64, AfterStorage, StorageTask<u64>)>,
    revision: u64,
    serial: u64,
    needs_store: bool,
    storage_failed: bool,
    network: Option<(Network, HttpTask)>,
    unknown: bool,
    state: GridState,
    columns: Vec<Column>,
    indices: Vec<usize>,
    view_keys: Vec<RowId>,
    bound_order: Option<RowId>,
    generation: u64,
    search: String,
    search_deadline: Option<f64>,
    lookup: String,
    lookup_deadline: Option<f64>,
    query_generation: u64,
    composing: bool,
    notice: Option<String>,
    summary: crate::components::SelectionSummary,
}
impl Default for TradeSales {
    fn default() -> Self {
        let mut state = GridState::default();
        state.metrics = erp_ui_egui::GridMetrics::legacy();
        state.theme = Some(crate::erp_theme::grid());
        let columns = [
            (1, "货品编码", 160.0),
            (2, "基准数量", 110.0),
            (3, "单价", 110.0),
            (4, "折扣", 100.0),
            (5, "税额", 100.0),
            (6, "交期", 125.0),
            (7, "收货地址", 240.0),
            (8, "来源", 90.0),
        ]
        .into_iter()
        .map(|(id, title, width)| {
            let mut column = Column::new(ColumnId(id), title, width);
            column.filter = FilterKind::None;
            column.edit_kind = match id {
                3..=5 => EditKind::Money,
                6 => EditKind::Date,
                8 => EditKind::Choice(vec!["转售".into(), "自产".into()]),
                _ => EditKind::Text,
            };
            column.value_kind = match id {
                2 => ValueKind::Decimal { scale: 6 },
                3..=5 => ValueKind::Money,
                _ => ValueKind::Text,
            };
            column
        })
        .collect();
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
            network: None,
            unknown: false,
            state,
            columns,
            indices: vec![0],
            view_keys: Vec::new(),
            bound_order: None,
            generation: 0,
            search: String::new(),
            search_deadline: None,
            lookup: String::new(),
            lookup_deadline: None,
            query_generation: 0,
            composing: false,
            notice: None,
            summary: Default::default(),
        }
    }
}
impl TradeSales {
    pub(super) fn dirty(&self) -> bool {
        self.draft.changed
            || self.pending.is_some()
            || self.finished.is_some()
            || self.state.is_editing()
            || self.storing.is_some()
    }
    pub(super) fn blocked(&self) -> Option<&'static str> {
        if self.pending.is_some() {
            Some("销售保存结果未确认，请查原结果；不能丢弃原请求。")
        } else if self.storing.is_some() {
            Some("销售草稿正在保存，请等待。")
        } else {
            None
        }
    }
    pub(super) fn refresh(&mut self) {
        self.suspend_reads();
        if self.lookup.trim().is_empty() && self.draft.version.is_some() {
            self.lookup = self.draft.number.clone();
        }
        if !self.lookup.trim().is_empty() {
            self.lookup_deadline = Some(0.0);
        }
    }
    pub(super) fn hold(&mut self) {
        self.state.hold_edit();
    }
    pub(super) fn suspend_reads(&mut self) {
        self.query_generation = self.query_generation.wrapping_add(1);
        self.lookup_deadline = None;
        if matches!(&self.network, Some((Network::Lookup(_), _))) {
            self.network = None;
        }
    }
    pub(super) fn discard(&mut self, ctx: &egui::Context) -> Result<bool, String> {
        if self.pending.is_some() {
            return Err("原销售请求结果未确认，不能丢弃".into());
        }
        if self.storing.is_some() {
            return Ok(false);
        }
        if !self.dirty() {
            return Ok(true);
        }
        if self.storage_failed || self.store.is_none() {
            return Err("销售存储未就绪，无法确认清除草稿".into());
        }
        self.state.cancel_edit();
        self.draft = Default::default();
        self.serial = self.serial.wrapping_add(1);
        self.needs_store = true;
        self.notice = None;
        self.rebuild();
        self.start_storage(ctx)?;
        Ok(false)
    }
    fn changed(&mut self) {
        self.serial = self.serial.wrapping_add(1);
        self.draft.changed = true;
        self.needs_store = true;
        self.rebuild();
    }
    fn rebuild(&mut self) {
        let keyword = self.search.trim().to_lowercase();
        self.indices = self
            .draft
            .lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                if keyword.is_empty()
                    || line.sku.to_lowercase().contains(&keyword)
                    || line.address.to_lowercase().contains(&keyword)
                {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();
        self.indices.sort_by(|a, b| {
            for rule in &self.state.sorts {
                if let Some(column) = self.columns.iter().find(|column| column.id == rule.column) {
                    let order = compare_values(
                        &self.draft.lines[*a].text(rule.column),
                        &self.draft.lines[*b].text(rule.column),
                        column.value_kind,
                        rule.descending,
                    );
                    if !order.is_eq() {
                        return order;
                    }
                }
            }
            self.draft.lines[*a]
                .number
                .cmp(&self.draft.lines[*b].number)
        });
        let keys = self
            .indices
            .iter()
            .map(|index| self.draft.row_id(&self.draft.lines[*index]))
            .collect::<Vec<_>>();
        self.generation = self.generation.wrapping_add(1);
        if self.bound_order != Some(self.draft.order) {
            self.state.bind_view(self.generation);
            self.bound_order = Some(self.draft.order);
        } else if self.view_keys != keys {
            self.state.rebind_page(self.generation, false);
        } else {
            self.state.adopt_generation(self.generation);
        }
        self.view_keys = keys;
    }
    fn edits(&mut self, edits: &[CellEdit]) -> Result<(), String> {
        let mut draft = self.draft.clone();
        let order = draft.order;
        for edit in edits {
            let line = draft
                .lines
                .iter_mut()
                .find(|line| RowId(order.0 ^ u128::from(line.number.unsigned_abs())) == edit.row)
                .ok_or("明细已变化")?;
            if line.text(edit.column) != edit.before {
                return Err("明细版本已变化".into());
            }
            line.set(edit.column, &edit.after)?;
        }
        self.draft = draft;
        self.serial = self.serial.wrapping_add(1);
        self.draft.changed = true;
        self.needs_store = true;
        Ok(())
    }
    fn commit_editor(&mut self) -> Result<(), String> {
        if let Some(draft) = self.state.edit_draft() {
            let column = self
                .columns
                .iter()
                .find(|column| column.id == draft.column)
                .cloned()
                .ok_or("编辑列已变化")?;
            if let Some((edit, ticket)) = self.state.prepare_edit_commit(&column)? {
                let result = self.edits(&[edit]);
                self.state.resolve_edit(ticket, result.clone());
                result?;
                self.rebuild();
            }
        }
        Ok(())
    }
    pub(super) fn save_for_leave(&mut self) -> Result<bool, String> {
        if let Some(reason) = self.blocked() {
            return Err(reason.into());
        }
        self.commit_editor()?;
        if !self.draft.changed {
            return Ok(true);
        }
        if self.store.is_none() || self.storage_failed {
            return Err("销售恢复存储尚未就绪".into());
        }
        let request = uuid::Uuid::new_v4().to_string();
        let body = self.draft.payload(&request)?;
        self.pending = Some(Prepared { request, body });
        self.serial = self.serial.wrapping_add(1);
        self.needs_store = true;
        self.unknown = false;
        self.notice = None;
        Ok(false)
    }
    fn checkpoint(&self) -> Result<Vec<u8>, String> {
        let scope = self.scope.clone().ok_or("销售账号归属未就绪")?;
        let checkpoint = Checkpoint {
            schema: 1,
            scope,
            draft: self.finished.as_ref().unwrap_or(&self.draft).clone(),
            rejected: self
                .finish_rejection
                .as_ref()
                .or(self.last_rejection.as_ref())
                .cloned(),
            pending: if self.finished.is_some() {
                None
            } else {
                self.pending.clone()
            },
        };
        let bytes = serde_json::to_vec(&checkpoint).map_err(|_| "销售恢复记录编码失败")?;
        if bytes.len() > model::MAX_BYTES {
            return Err("销售草稿超过1MiB，请减少明细后保存".into());
        }
        Ok(bytes)
    }
    fn start_storage(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let bytes = self.checkpoint()?;
        let store = self.store.as_ref().ok_or("销售存储未就绪")?;
        let task = store.commit(self.revision, Some(&bytes), ctx)?;
        let after = if self.finished.is_some() {
            AfterStorage::Complete
        } else if self.pending.is_some() && !self.unknown {
            AfterStorage::Send
        } else if !self.draft.changed {
            AfterStorage::Discard
        } else {
            AfterStorage::Draft
        };
        self.storing = Some((self.serial, after, task));
        self.needs_store = false;
        Ok(())
    }
    fn stage_command_response(&mut self, response: &Value) -> Result<(), String> {
        let prepared = self
            .pending
            .as_ref()
            .ok_or_else(|| "原请求丢失".to_string())?;
        if let Some(rejection) = model::rejected(prepared, response)? {
            self.finish_rejection = Some(rejection);
            self.finished = Some(self.draft.clone());
        } else {
            let document = response.get("document").unwrap_or(response);
            self.finished = Some(model::matches(prepared, document)?);
            self.finish_rejection = None;
        }
        self.needs_store = true;
        Ok(())
    }
    fn finish_command_storage(&mut self) {
        if let Some(draft) = self.finished.take() {
            self.draft = draft;
        }
        self.pending = None;
        self.unknown = false;
        if let Some(rejection) = self.finish_rejection.take() {
            self.notice = Some(format!("保存被拒绝，输入已保留：{}", rejection.message));
            self.last_rejection = Some(rejection);
        } else {
            self.notice = Some("销售单已保存".into());
        }
        self.rebuild();
    }
    fn request(
        &self,
        session: &Session,
        path: String,
        body: Option<Value>,
        key: Option<String>,
        ctx: &egui::Context,
    ) -> Result<HttpTask, String> {
        session
            .client
            .start(
                HttpRequest {
                    path,
                    query: if body.is_some() {
                        vec![("rejection_envelope".into(), "true".into())]
                    } else {
                        vec![]
                    },
                    json: body
                        .map(|value| serde_json::to_vec(&value))
                        .transpose()
                        .map_err(|_| "销售请求编码失败")?,
                    bearer: Some(session.token.clone()),
                    idempotency_key: key,
                },
                ctx,
            )
            .map_err(|e| e.to_string())
    }
    fn send(&mut self, session: &Session, ctx: &egui::Context) -> Result<(), String> {
        let prepared = self.pending.as_ref().ok_or("原销售请求缺失")?;
        let task = self.request(
            session,
            format!(
                "/v1/tenants/{}/trade-sales-orders",
                session.tenant.to_uuid()
            ),
            Some(prepared.body.clone()),
            Some(prepared.request.clone()),
            ctx,
        )?;
        self.network = Some((Network::Save, task));
        Ok(())
    }
    fn recover(&mut self, session: &Session, ctx: &egui::Context) -> Result<(), String> {
        let request = &self.pending.as_ref().ok_or("原销售请求缺失")?.request;
        let task = self.request(
            session,
            format!(
                "/v1/tenants/{}/trade-sales-order-commands/{request}",
                session.tenant.to_uuid()
            ),
            None,
            None,
            ctx,
        )?;
        self.network = Some((Network::Recover, task));
        Ok(())
    }
    pub(super) fn tick(&mut self, ctx: &egui::Context, session: &Session, scope: ActionScope) {
        if self.scope.as_ref() != Some(&scope) {
            if self.dirty() {
                self.notice = Some("存在另一账号的销售草稿，请返回原账号处理".into());
                return;
            }
            self.suspend_reads();
            self.store = None;
            self.opening = None;
            self.revision = 0;
            self.scope = Some(scope.clone());
            self.draft = Default::default();
            self.storage_failed = false;
        }
        if self.store.is_none() && self.opening.is_none() && !self.storage_failed {
            match OrderRecoveryStore::open_trade(scope.clone(), ctx) {
                Ok(task) => self.opening = Some(task),
                Err(e) => {
                    self.notice = Some(e);
                    self.storage_failed = true;
                }
            }
        }
        if let Some(result) = self.opening.as_mut().and_then(StorageTask::poll) {
            self.opening = None;
            match result.and_then(|loaded| {
                if let Some(bytes) = loaded.bytes.as_deref() {
                    let checkpoint = Checkpoint::decode(bytes, &scope)?;
                    self.draft = checkpoint.draft;
                    self.pending = checkpoint.pending;
                    self.last_rejection = checkpoint.rejected;
                    self.unknown = self.pending.is_some();
                }
                self.revision = loaded.revision;
                self.store = Some(loaded.store);
                self.rebuild();
                Ok(())
            }) {
                Ok(()) => {
                    if self.pending.is_some()
                        && let Err(e) = self.recover(session, ctx)
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
        if let Some(result) = self.storing.as_mut().and_then(|(_, _, task)| task.poll())
            && let Some((serial, after, _)) = self.storing.take()
        {
            match result {
                Ok(revision) => {
                    self.revision = revision;
                    self.needs_store |= serial != self.serial;
                    match after {
                        AfterStorage::Send => {
                            if let Err(e) = self.send(session, ctx) {
                                self.unknown = true;
                                self.notice = Some(e);
                            }
                        }
                        AfterStorage::Complete => self.finish_command_storage(),
                        AfterStorage::Draft => {}
                        AfterStorage::Discard => {
                            if serial == self.serial {
                                self.draft.changed = false;
                            }
                        }
                    }
                }
                Err(e) => {
                    self.notice = Some(e);
                    self.storage_failed = true;
                }
            }
        }
        if let Some(result) = self.network.as_mut().and_then(|(_, task)| task.poll())
            && let Some((operation, _)) = self.network.take()
        {
            match result.map_err(|e| e.to_string()).and_then(|bytes| {
                serde_json::from_slice::<Value>(&bytes).map_err(|_| "销售响应格式错误".into())
            }) {
                Ok(response) => {
                    let result = match operation {
                        Network::Save | Network::Recover => self.stage_command_response(&response),
                        Network::Lookup(generation) => {
                            if generation != self.query_generation || self.dirty() {
                                Ok(())
                            } else {
                                Draft::decode(&response).map(|draft| {
                                    self.draft = draft;
                                    self.needs_store = true;
                                    self.serial = self.serial.wrapping_add(1);
                                    self.rebuild();
                                })
                            }
                        }
                    };
                    if let Err(e) = result {
                        self.unknown = self.pending.is_some();
                        self.notice = Some(e);
                    }
                }
                Err(e) => {
                    self.unknown = self.pending.is_some();
                    self.notice = Some(e);
                }
            }
        }
        if self.needs_store
            && self.storing.is_none()
            && !self.storage_failed
            && self.store.is_some()
            && let Err(e) = self.start_storage(ctx)
        {
            self.notice = Some(e);
            self.storage_failed = true;
        }
        if self
            .search_deadline
            .is_some_and(|deadline| ctx.input(|i| i.time) >= deadline)
            && !self.composing
        {
            self.search_deadline = None;
            self.state.row_selection.clear();
            self.rebuild();
        }
        if self.network.is_none()
            && !self.dirty()
            && !self.composing
            && self
                .lookup_deadline
                .is_some_and(|deadline| ctx.input(|i| i.time) >= deadline)
        {
            self.lookup_deadline = None;
            let task = session.client.start(
                HttpRequest {
                    path: format!(
                        "/v1/tenants/{}/trade-sales-order-lookup",
                        session.tenant.to_uuid()
                    ),
                    query: vec![("document_no".into(), self.lookup.trim().into())],
                    json: None,
                    bearer: Some(session.token.clone()),
                    idempotency_key: None,
                },
                ctx,
            );
            match task {
                Ok(task) => self.network = Some((Network::Lookup(self.query_generation), task)),
                Err(e) => self.notice = Some(e.to_string()),
            }
        }
        if self.network.is_some() || self.storing.is_some() || self.opening.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(25));
        }
    }

    pub(super) fn ui(&mut self, ui: &mut egui::Ui, session: &Session, scope: &ActionScope) {
        if self.scope.as_ref() != Some(scope) {
            ui.label("请返回草稿所属账号后继续");
            return;
        }
        self.state.theme = Some(crate::erp_theme::grid());
        let ready = self.store.is_some() && !self.storage_failed;
        let clearing = matches!(self.storing.as_ref(), Some((_, AfterStorage::Discard, _)));
        let editable = ready
            && !clearing
            && self.pending.is_none()
            && self.finished.is_none()
            && self
                .draft
                .status
                .as_deref()
                .is_none_or(|status| matches!(status, "SUBMITTED" | "FULFILLING"));
        self.state.read_only = !editable;
        self.composing = ui.input(|input| {
            input
                .events
                .iter()
                .fold(self.composing, |old, event| match event {
                    egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => !text.is_empty(),
                    egui::Event::Ime(egui::ImeEvent::Commit(_)) => false,
                    _ => old,
                })
        });
        let mut save = false;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("查单");
            let response = ui.add_enabled(
                !self.dirty(),
                egui::TextEdit::singleline(&mut self.lookup)
                    .desired_width(180.0)
                    .hint_text("单据号"),
            );
            if response.changed() {
                self.suspend_reads();
                self.query_generation = self.query_generation.wrapping_add(1);
                self.lookup_deadline =
                    (!self.lookup.trim().is_empty()).then(|| ui.input(|i| i.time) + 0.25);
            }
            if ui
                .add_enabled(editable && !self.dirty(), egui::Button::new("新建"))
                .clicked()
            {
                self.draft = Default::default();
                self.lookup.clear();
                self.suspend_reads();
                self.notice = None;
                self.rebuild();
            }
            if ui
                .add_enabled(editable, egui::Button::new("保存"))
                .clicked()
            {
                save = true;
            }
            if self.pending.is_some()
                && self.network.is_none()
                && ui.button("查原结果").clicked()
                && let Err(e) = self.recover(session, ui.ctx())
            {
                self.notice = Some(e);
            }
            if self.pending.is_some()
                && self.unknown
                && self.network.is_none()
                && !self.storage_failed
                && self.finished.is_none()
                && ui
                    .button("重试原请求")
                    .on_hover_text("保留原请求号和原字段，由服务器幂等处理")
                    .clicked()
                && let Err(e) = self.send(session, ui.ctx())
            {
                self.notice = Some(e);
            }
            if self.storage_failed && ui.button("复制恢复记录").clicked() {
                match self.checkpoint() {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(text) => ui.ctx().copy_text(text),
                        Err(_) => self.notice = Some("恢复记录编码错误".into()),
                    },
                    Err(e) => self.notice = Some(e),
                }
            }
            if self.opening.is_some() || self.storing.is_some() || self.network.is_some() {
                ui.spinner();
            }
        });
        ui.horizontal(|ui| {
            ui.label("单据号");
            changed |= ui
                .add_enabled(
                    editable && self.draft.version.is_none(),
                    egui::TextEdit::singleline(&mut self.draft.number)
                        .desired_width(180.0)
                        .char_limit(128),
                )
                .changed();
            ui.label("客户编码");
            changed |= ui
                .add_enabled(
                    editable,
                    egui::TextEdit::singleline(&mut self.draft.customer)
                        .desired_width(140.0)
                        .char_limit(128),
                )
                .changed();
            ui.add_enabled_ui(editable && self.draft.version.is_none(), |ui| {
                egui::ComboBox::from_id_salt("trade-currency")
                    .selected_text(&self.draft.currency)
                    .show_ui(ui, |ui| {
                        for currency in ["CNY", "USD", "EUR", "HKD"] {
                            changed |= ui
                                .selectable_value(
                                    &mut self.draft.currency,
                                    currency.into(),
                                    currency,
                                )
                                .changed();
                        }
                    });
            });
            if let Some(version) = self.draft.version {
                ui.label(format!("版本 {version}"));
            }
        });
        if changed {
            self.changed();
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    editable && self.draft.lines.len() < 1000,
                    egui::Button::new("增行"),
                )
                .clicked()
            {
                if let Some(number) = self
                    .draft
                    .lines
                    .iter()
                    .map(|line| line.number)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                {
                    self.draft.lines.push(Line::new(number));
                    self.changed();
                } else {
                    self.notice = Some("行身份已达上限".into());
                }
            }
            if ui
                .add_enabled(
                    editable && self.search_deadline.is_none(),
                    egui::Button::new("删行"),
                )
                .clicked()
            {
                let checked = self.state.row_selection.checked();
                let active = self.state.row_selection.active;
                let order = self.draft.order;
                self.draft.lines.retain(|line| {
                    let id = RowId(order.0 ^ u128::from(line.number.unsigned_abs()));
                    if checked.is_empty() {
                        Some(id) != active
                    } else {
                        !checked.contains(&id)
                    }
                });
                self.changed();
            }
            ui.label("明细搜索");
            if ui
                .add(egui::TextEdit::singleline(&mut self.search).desired_width(170.0))
                .changed()
            {
                self.search_deadline = Some(ui.input(|i| i.time) + 0.25);
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(250));
            }
        });
        let height = (ui.available_height() - 28.0).max(80.0);
        let output = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let rows = &self.draft.lines;
                    let indices = &self.indices;
                    let order = self.draft.order;
                    DataGrid {
                        id: ui.id().with(("trade-lines", order)),
                        columns: &mut self.columns,
                        rows: indices.len(),
                        state: &mut self.state,
                        row_id: Some(&|index| {
                            RowId(order.0 ^ u128::from(rows[indices[index]].number.unsigned_abs()))
                        }),
                    }
                    .show(ui, |index, column| rows[indices[index]].text(column))
                },
            )
            .inner;
        if output.sorts_changed {
            self.rebuild();
        }
        if let Some(ticket) = output.edit_ticket {
            let result = if editable {
                self.edits(&output.edits)
            } else {
                Err("销售单正在提交，暂不能修改".into())
            };
            self.state.resolve_edit(ticket, result.clone());
            if let Err(e) = result {
                self.notice = Some(e);
            } else {
                self.rebuild();
            }
        }
        if save && let Err(e) = self.save_for_leave() {
            self.notice = Some(e);
        }
        self.summary.update(
            self.generation,
            self.serial,
            &self.state.selection,
            &self.columns,
            self.indices.len(),
            |index, column| self.draft.lines[self.indices[index]].text(column),
        );
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} 行 · 勾选 {}",
                self.draft.lines.len(),
                self.state.row_selection.checked().len()
            ));
            ui.label(&self.summary.label);
            if self.pending.is_some() {
                ui.label("原保存待确认");
            } else if self.draft.changed {
                ui.label("未保存");
            } else if let Some(status) = self.draft.status.as_deref() {
                ui.label(match status {
                    "FULFILLED" => "已下达",
                    "CANCELLED" => "已取消",
                    "FULFILLING" => "执行中",
                    _ => "已提交",
                });
            }
            if let Some(error) = &self.notice {
                ui.label(error);
            }
        });
    }
}
