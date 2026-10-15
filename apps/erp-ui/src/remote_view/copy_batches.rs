use super::{Session, selection::FrozenSelection};
use crate::{
    http::{HttpRequest, HttpTask},
    remote_copy_batches::{self as wire, BatchStatus, StartBody},
    remote_products,
};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId, remote::RemoteRow};
use erp_ui_egui::{DataGrid, GridState};

#[path = "copy_retry_review.rs"]
mod retry_review;

#[derive(Clone, Copy)]
enum Request {
    List,
    Start(RowId),
    Status(RowId),
    Advance(RowId),
    Cancel(RowId),
    Results(RowId, Option<u64>, bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InspectionKey {
    batch: RowId,
    operation: RowId,
    source: RowId,
}
#[derive(Default)]
pub(super) struct CopyBatches {
    binding: Option<(String, RowId, u64)>,
    jobs: Vec<BatchStatus>,
    prepared: Option<StartBody>,
    task: Option<(Request, HttpTask)>,
    running: Option<RowId>,
    next_poll: f64,
    loaded: bool,
    error: Option<String>,
    open: bool,
    state: GridState,
    columns: Vec<Column>,
    results: Option<wire::BatchResults>,
    results_open: bool,
    failed_only: bool,
    result_state: GridState,
    result_columns: Vec<Column>,
    inspection_task: Option<(InspectionKey, HttpTask)>,
    inspection: Option<(InspectionKey, RemoteRow)>,
    inspection_error: Option<(InspectionKey, String)>,
    inspection_state: GridState,
    inspection_columns: Vec<Column>,
    location: Option<String>,
    retry_review: Option<retry_review::Review>,
}
impl CopyBatches {
    pub(super) fn dirty(&self) -> bool {
        self.prepared.is_some() || self.retry_review.is_some()
    }
    pub(super) fn leave_pending(&self) -> bool {
        self.running.is_some()
            || self.task.as_ref().is_some_and(|(request, _)| {
                matches!(
                    request,
                    Request::Start(_) | Request::Advance(_) | Request::Cancel(_)
                )
            })
            || self
                .jobs
                .iter()
                .any(|job| job.pending > 0 || job.unknown > 0)
    }
    pub(super) fn discard_unsent(&mut self) {
        self.prepared = None;
        self.retry_review = None;
    }
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn take_location(&mut self) -> Option<String> {
        self.location.take()
    }
    fn clear_inspection(&mut self) {
        self.inspection_task = None;
        self.inspection = None;
        self.inspection_error = None;
        self.inspection_state.clear_selection();
        self.location = None;
    }
    fn request(&mut self, session: &Session, kind: Request, ctx: &egui::Context) {
        if self.task.is_some() || self.retry_review.is_some() {
            return;
        }
        if matches!(kind, Request::Start(_) | Request::Results(..)) {
            self.clear_inspection();
        }
        let request = match kind {
            Request::List => wire::encode_list(session.tenant),
            Request::Start(_) => self
                .prepared
                .as_ref()
                .ok_or_else(|| "缺少待确认批次".to_owned())
                .and_then(|body| wire::encode_start(session.tenant, body)),
            Request::Status(id) => wire::encode_request(session.tenant, id, "status"),
            Request::Advance(id) => wire::encode_request(session.tenant, id, "advance"),
            Request::Cancel(id) => wire::encode_request(session.tenant, id, "cancel"),
            Request::Results(id, after, failed_only) => {
                wire::encode_results(session.tenant, id, after, failed_only)
            }
        };
        match request.and_then(|mut request: HttpRequest| {
            request.bearer = Some(session.token.clone());
            session
                .client
                .start(request, ctx)
                .map_err(|error| error.to_string())
        }) {
            Ok(task) => {
                self.task = Some((kind, task));
                self.error = None;
            }
            Err(error) => {
                self.error = Some(error);
                self.running = None;
            }
        }
    }
    fn poll(&mut self, session: &Session, generation: u64, ctx: &egui::Context, can_write: bool) {
        let binding = (
            session.client.base_url().to_owned(),
            session.tenant,
            generation,
        );
        if self.binding.as_ref() != Some(&binding) {
            self.clear();
            self.binding = Some(binding);
        }
        let now = ctx.input(|input| input.time);
        if let Some(result) = self.task.as_mut().and_then(|(_, task)| task.poll())
            && let Some((kind, _)) = self.task.take()
        {
            let result = result
                .map_err(|error| error.to_string())
                .and_then(|bytes| match kind {
                    Request::List => wire::decode_list(&bytes),
                    Request::Results(id, after, failed_only) => {
                        wire::decode_results(&bytes, id, after, failed_only).map(|results| {
                            self.results = Some(results);
                            self.result_state.clear_selection();
                            Vec::new()
                        })
                    }
                    Request::Start(id)
                    | Request::Status(id)
                    | Request::Advance(id)
                    | Request::Cancel(id) => {
                        wire::decode_status(&bytes, id).map(|status| vec![status])
                    }
                });
            match result {
                Ok(statuses) => {
                    if matches!(kind, Request::List) {
                        if let Some(prepared) = &self.prepared
                            && statuses
                                .iter()
                                .any(|status| status.batch_id == prepared.batch_id)
                        {
                            self.state.row_selection.active = Some(prepared.batch_id);
                            self.prepared = None;
                        }
                        self.jobs = statuses;
                        self.loaded = true;
                    } else {
                        for status in statuses {
                            if matches!(kind, Request::Start(_)) {
                                self.prepared = None;
                                self.running = Some(status.batch_id);
                            }
                            if status.terminal() && self.running == Some(status.batch_id) {
                                self.running = None;
                            }
                            if let Some(old) = self
                                .jobs
                                .iter_mut()
                                .find(|job| job.batch_id == status.batch_id)
                            {
                                *old = status;
                            } else {
                                self.state.row_selection.active = Some(status.batch_id);
                                self.jobs.insert(0, status);
                                self.jobs.truncate(20);
                            }
                        }
                    }
                    self.error = None;
                    self.next_poll = now + 1.0;
                }
                Err(error) => {
                    self.error = Some(error);
                    self.running = None;
                    self.loaded = true;
                }
            }
        }
        self.poll_inspection(session);
        if !can_write {
            self.running = None;
        }
        if self.task.is_none() {
            if !self.loaded && self.open {
                self.request(session, Request::List, ctx);
            } else if can_write
                && now >= self.next_poll
                && let Some(id) = self.running
            {
                self.request(session, Request::Advance(id), ctx);
            }
        }
        if self.task.is_some() || self.inspection_task.is_some() || self.running.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    fn selected_result(&self) -> Option<(InspectionKey, wire::BatchItem)> {
        let results = self.results.as_ref()?;
        let operation = self.result_state.row_selection.active?;
        let item = results
            .items
            .iter()
            .find(|item| item.operation_id == operation)?
            .clone();
        Some((
            InspectionKey {
                batch: results.batch_id,
                operation: item.operation_id,
                source: item.source,
            },
            item,
        ))
    }

    fn poll_inspection(&mut self, session: &Session) {
        let Some(result) = self
            .inspection_task
            .as_mut()
            .and_then(|(_, task)| task.poll())
        else {
            return;
        };
        let Some((key, _)) = self.inspection_task.take() else {
            return;
        };
        if self.selected_result().as_ref().map(|(current, _)| *current) != Some(key) {
            return;
        }
        match result
            .map_err(|error| error.to_string())
            .and_then(|bytes| remote_products::decode_product(session.tenant, key.source, &bytes))
        {
            Ok(row) => {
                self.inspection = Some((key, row));
                self.inspection_error = None;
            }
            Err(error) => {
                self.inspection = None;
                self.inspection_error = Some((key, error));
            }
        }
    }

    fn start_inspection(&mut self, session: &Session, key: InspectionKey, ctx: &egui::Context) {
        self.clear_inspection();
        if key.batch.0 == 0 || key.operation.0 == 0 || key.source.0 == 0 {
            self.inspection_error = Some((key, "原记录身份无效".into()));
            return;
        }
        match session.client.start(
            HttpRequest {
                path: format!(
                    "/v1/tenants/{}/products/{}",
                    session.tenant.to_uuid(),
                    key.source.to_uuid()
                ),
                query: vec![],
                json: None,
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            ctx,
        ) {
            Ok(task) => self.inspection_task = Some((key, task)),
            Err(error) => self.inspection_error = Some((key, error.to_string())),
        }
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &Session,
        generation: u64,
        frozen: &FrozenSelection,
        can_prepare: bool,
        can_write: bool,
    ) {
        self.poll(session, generation, ui.ctx(), can_write);
        if let Some(review) = &mut self.retry_review {
            let (close, prepared) = review.ui(ui, session, can_write && self.task.is_none());
            if close {
                self.retry_review = None;
            }
            if let Some(body) = prepared {
                let id = body.batch_id;
                self.prepared = Some(body);
                self.request(session, Request::Start(id), ui.ctx());
            }
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    can_prepare
                        && !self.dirty()
                        && self.task.is_none()
                        && !frozen.pending()
                        && frozen.count().is_some_and(|count| count > 0),
                    egui::Button::new("复制全部匹配"),
                )
                .clicked()
                && let (Some(credential), Some(count)) = (frozen.credential(), frozen.count())
            {
                self.prepared = Some(StartBody {
                    batch_id: RowId(uuid::Uuid::new_v4().as_u128()),
                    token: credential.token,
                    excluded: frozen.exclusions().iter().copied().collect(),
                    confirmed_count: count,
                });
                self.open = true;
            }
            if ui.selectable_label(self.open, "批量任务").clicked() {
                self.open = !self.open;
            }
            if self.open
                && ui
                    .add_enabled(
                        self.task.is_none() && self.retry_review.is_none(),
                        egui::Button::new("刷新任务"),
                    )
                    .clicked()
            {
                self.request(session, Request::List, ui.ctx());
            }
        });
        if let Some(body) = &self.prepared {
            let id = body.batch_id;
            let count = body.confirmed_count;
            ui.horizontal(|ui| {
                ui.label(format!("复制 {count} 个商品"));
                if ui
                    .add_enabled(
                        can_write && self.task.is_none(),
                        egui::Button::new("确认复制全部匹配"),
                    )
                    .clicked()
                {
                    self.request(session, Request::Start(id), ui.ctx());
                }
                if ui
                    .add_enabled(self.task.is_none(), egui::Button::new("关闭确认"))
                    .on_hover_text("若发送结果未知，可刷新批量任务找回原批次")
                    .clicked()
                {
                    self.prepared = None;
                }
            });
        }
        if !self.open {
            return;
        }
        ui.horizontal(|ui| {
            let id = self.state.row_selection.active;
            let pending = id
                .and_then(|id| self.jobs.iter().find(|job| job.batch_id == id))
                .is_some_and(|job| !job.terminal());
            let rejected = id
                .and_then(|id| self.jobs.iter().find(|job| job.batch_id == id))
                .is_some_and(|job| job.terminal() && job.rejected > 0);
            if ui
                .add_enabled(
                    can_prepare && rejected && self.task.is_none() && !self.dirty(),
                    egui::Button::new("重新审核失败项"),
                )
                .clicked()
                && let Some(id) = id
            {
                self.running = None;
                self.retry_review = Some(retry_review::Review::new(id));
            }
            if ui
                .add_enabled(
                    can_write && pending && self.task.is_none() && self.retry_review.is_none(),
                    egui::Button::new("继续任务"),
                )
                .clicked()
            {
                self.running = id;
                self.next_poll = 0.0;
            }
            if ui
                .add_enabled(self.running.is_some(), egui::Button::new("暂停推进"))
                .clicked()
            {
                self.running = None;
            }
            if ui
                .add_enabled(
                    id.is_some() && self.task.is_none() && self.retry_review.is_none(),
                    egui::Button::new("查询任务"),
                )
                .clicked()
                && let Some(id) = id
            {
                self.request(session, Request::Status(id), ui.ctx());
            }
            if ui
                .add_enabled(
                    id.is_some() && self.task.is_none() && self.retry_review.is_none(),
                    egui::Button::new("查看明细"),
                )
                .clicked()
                && let Some(id) = id
            {
                self.results_open = true;
                self.results = None;
                self.request(
                    session,
                    Request::Results(id, None, self.failed_only),
                    ui.ctx(),
                );
            }
            if ui
                .add_enabled(
                    can_write && pending && self.task.is_none() && self.retry_review.is_none(),
                    egui::Button::new("取消未开始项"),
                )
                .clicked()
                && let Some(id) = id
            {
                self.running = None;
                self.request(session, Request::Cancel(id), ui.ctx());
            }
            if self.task.is_some() {
                ui.spinner();
            }
            if let Some(error) = &self.error {
                ui.label(error);
            }
        });
        self.state.read_only = true;
        self.state.filtering_enabled = false;
        self.state.sorting_enabled = false;
        if self.columns.is_empty() {
            self.columns = ["批次", "总数", "成功", "拒绝", "取消", "待处理", "待确认"]
                .into_iter()
                .enumerate()
                .map(|(i, title)| {
                    Column::new(ColumnId(i as u64), title, if i == 0 { 230.0 } else { 90.0 })
                })
                .collect();
        }
        ui.allocate_ui(egui::vec2(ui.available_width(), 170.0), |ui| {
            DataGrid {
                id: ui.id().with("copy-batches"),
                columns: &mut self.columns,
                rows: self.jobs.len(),
                state: &mut self.state,
                row_id: Some(&|row| self.jobs[row].batch_id),
            }
            .show(ui, |row, column| {
                let job = &self.jobs[row];
                match column.0 {
                    0 => job.batch_id.to_uuid(),
                    1 => job.total.to_string(),
                    2 => job.applied.to_string(),
                    3 => job.rejected.to_string(),
                    4 => job.cancelled.to_string(),
                    5 => job.pending.to_string(),
                    6 => job.unknown.to_string(),
                    _ => String::new(),
                }
            });
        });
        self.result_ui(ui, session);
    }

    fn result_ui(&mut self, ui: &mut egui::Ui, session: &Session) {
        if !self.results_open || self.retry_review.is_some() {
            return;
        }
        let mut open = true;
        egui::Window::new("任务明细")
            .id(ui.id().with("copy-results"))
            .open(&mut open)
            .default_size(egui::vec2(880.0, 420.0))
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    let id = self
                        .results
                        .as_ref()
                        .map(|results| results.batch_id)
                        .or(self.state.row_selection.active);
                    let change = ui
                        .add_enabled(
                            self.task.is_none(),
                            egui::Checkbox::new(&mut self.failed_only, "仅失败"),
                        )
                        .changed();
                    let refresh = ui
                        .add_enabled(self.task.is_none(), egui::Button::new("刷新明细"))
                        .clicked();
                    if (change || refresh)
                        && let Some(id) = id
                    {
                        self.results = None;
                        self.request(
                            session,
                            Request::Results(id, None, self.failed_only),
                            ui.ctx(),
                        );
                    }
                    let next = self.results.as_ref().and_then(|results| results.next_after);
                    if ui
                        .add_enabled(
                            next.is_some() && self.task.is_none(),
                            egui::Button::new("下一页"),
                        )
                        .clicked()
                        && let Some(id) = id
                    {
                        self.results = None;
                        self.request(
                            session,
                            Request::Results(id, next, self.failed_only),
                            ui.ctx(),
                        );
                    }
                });
                if let Some(error) = &self.error {
                    ui.label(error);
                }
                let Some(results) = &self.results else {
                    if self.task.is_some() {
                        ui.spinner();
                    }
                    return;
                };
                if self.result_columns.is_empty() {
                    self.result_columns = ["原记录", "状态", "结果编码", "原因"]
                        .into_iter()
                        .enumerate()
                        .map(|(i, title)| {
                            Column::new(
                                ColumnId(i as u64),
                                title,
                                if i == 0 { 250.0 } else { 180.0 },
                            )
                        })
                        .collect();
                }
                self.result_state.read_only = true;
                self.result_state.filtering_enabled = false;
                self.result_state.sorting_enabled = false;
                ui.allocate_ui(egui::vec2(ui.available_width(), 180.0), |ui| {
                    DataGrid {
                        id: ui.id().with("rows"),
                        columns: &mut self.result_columns,
                        rows: results.items.len(),
                        state: &mut self.result_state,
                        row_id: Some(&|row| results.items[row].operation_id),
                    }
                    .show(ui, |row, column| {
                        use erp_grid_core::actions::ActionOutcome;
                        let item = &results.items[row];
                        match column.0 {
                            0 => item.source.to_uuid(),
                            1 => match &*item.state {
                                "PENDING" => "待处理",
                                "IN_FLIGHT" => "待确认",
                                "APPLIED" => "成功",
                                "REJECTED" => "拒绝",
                                "CANCELLED" => "取消",
                                _ => "未知",
                            }
                            .into(),
                            2 => item
                                .receipt
                                .as_ref()
                                .and_then(|receipt| match &receipt.outcome {
                                    ActionOutcome::Applied { values, .. } => {
                                        values.get(&ColumnId(0)).cloned()
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default(),
                            3 => item
                                .receipt
                                .as_ref()
                                .and_then(|receipt| match &receipt.outcome {
                                    ActionOutcome::Rejected { code, message } => {
                                        Some(format!("{code}: {message}"))
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default(),
                            _ => String::new(),
                        }
                    });
                });
                let selected = self.selected_result();
                let selected_key = selected.as_ref().map(|(key, _)| *key);
                if self
                    .inspection_task
                    .as_ref()
                    .map(|(key, _)| *key)
                    .or(self.inspection.as_ref().map(|(key, _)| *key))
                    .or(self.inspection_error.as_ref().map(|(key, _)| *key))
                    .is_some_and(|key| Some(key) != selected_key)
                {
                    self.clear_inspection();
                }
                ui.horizontal(|ui| {
                    let inspect = ui.add_enabled(
                        selected_key.is_some() && self.inspection_task.is_none(),
                        egui::Button::new("查看原记录"),
                    );
                    if inspect.clicked()
                        && let Some(key) = selected_key
                    {
                        self.start_inspection(session, key, ui.ctx());
                    }

                    let copy_code = selected.as_ref().and_then(|(_, item)| applied_code(item));
                    let find_copy = ui
                        .add_enabled(copy_code.is_some(), egui::Button::new("查找副本"))
                        .on_hover_text("仅按批次结果中的非空商品编码查找，编码可能不唯一");
                    if find_copy.clicked()
                        && let Some(code) = copy_code
                    {
                        self.location = Some(code);
                    }

                    let source_code = selected_key.and_then(|key| {
                        self.inspection
                            .as_ref()
                            .filter(|(loaded, _)| *loaded == key)
                            .and_then(|(_, row)| nonempty_code(row))
                    });
                    let find_source = ui
                        .add_enabled(source_code.is_some(), egui::Button::new("查找原记录"))
                        .on_hover_text(
                            "仅按最新原记录的非空商品编码查找；编码为空时不会用可能有歧义的名称代替",
                        );
                    if find_source.clicked()
                        && let Some(code) = source_code
                    {
                        self.location = Some(code);
                    }
                });
                if let Some((_, item)) = &selected {
                    ui.label(format!("原结果状态：{}", result_state_label(&item.state)));
                }
                if self.inspection_task.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("正在读取原记录");
                    });
                }
                if let Some((key, error)) = &self.inspection_error
                    && Some(*key) == selected_key
                {
                    ui.label(error);
                }
                self.ensure_inspection_columns();
                if let Some((key, row)) = &self.inspection
                    && Some(*key) == selected_key
                {
                    self.inspection_state.read_only = true;
                    self.inspection_state.filtering_enabled = false;
                    self.inspection_state.sorting_enabled = false;
                    ui.allocate_ui(egui::vec2(ui.available_width(), 100.0), |ui| {
                        DataGrid {
                            id: ui.id().with("source-product"),
                            columns: &mut self.inspection_columns,
                            rows: 1,
                            state: &mut self.inspection_state,
                            row_id: Some(&|_| row.id),
                        }
                        .show(ui, |_, column| {
                            row.cells.get(&column).cloned().unwrap_or_default()
                        });
                    });
                }
            });
        self.results_open = open;
    }

    fn ensure_inspection_columns(&mut self) {
        if self.inspection_columns.is_empty() {
            self.inspection_columns = [
                (remote_products::CODE, "编码", 120.0),
                (remote_products::NAME, "名称", 180.0),
                (remote_products::BRAND, "品牌", 110.0),
                (remote_products::CATEGORY, "分类", 110.0),
                (remote_products::STATUS, "状态", 90.0),
                (remote_products::CREATED, "创建时间", 160.0),
                (remote_products::UPDATED, "更新时间", 160.0),
            ]
            .into_iter()
            .map(|(id, title, width)| Column::new(id, title, width))
            .collect();
        }
    }
}

fn applied_code(item: &wire::BatchItem) -> Option<String> {
    use erp_grid_core::actions::ActionOutcome;
    item.receipt
        .as_ref()
        .and_then(|receipt| match &receipt.outcome {
            ActionOutcome::Applied { values, .. } => values
                .get(&remote_products::CODE)
                .map(String::as_str)
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(str::to_owned),
            _ => None,
        })
}

fn nonempty_code(row: &RemoteRow) -> Option<String> {
    row.cells
        .get(&remote_products::CODE)
        .map(String::as_str)
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
}

fn result_state_label(state: &str) -> &'static str {
    match state {
        "PENDING" => "待处理",
        "IN_FLIGHT" => "待确认",
        "APPLIED" => "成功",
        "REJECTED" => "拒绝",
        "CANCELLED" => "取消",
        _ => "未知",
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    #[test]
    fn copy_retry_review_blocks_background_batch_requests() {
        let (client, server) = crate::remote_actions::tests::server(vec![]);
        let session = Session {
            client,
            tenant: RowId(1),
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let mut view = controller(&session);
        view.retry_review = Some(retry_review::Review::new(RowId(55)));
        let ctx = egui::Context::default();
        for kind in [
            Request::List,
            Request::Status(RowId(56)),
            Request::Advance(RowId(56)),
            Request::Cancel(RowId(56)),
            Request::Results(RowId(56), None, false),
        ] {
            view.request(&session, kind, &ctx);
            assert!(view.task.is_none());
            assert!(view.retry_review.is_some());
        }
        assert!(server.join().unwrap().is_empty());
    }
    fn status(applied: u64, pending: u64, unknown: u64, cancelled: u64) -> BatchStatus {
        BatchStatus {
            batch_id: RowId(55),
            total: 2,
            applied,
            rejected: 0,
            cancelled,
            pending,
            unknown,
            cancel_requested: cancelled > 0,
        }
    }
    #[test]
    fn leave_preserves_sent_or_unknown_copy_tracking_and_discards_only_review() {
        let mut view = CopyBatches {
            jobs: vec![status(0, 0, 1, 0)],
            prepared: Some(StartBody {
                batch_id: RowId(56),
                token: RowId(2),
                excluded: Vec::new(),
                confirmed_count: 1,
            }),
            ..Default::default()
        };
        assert!(view.leave_pending());
        view.discard_unsent();
        assert!(view.prepared.is_none());
        assert_eq!(view.jobs.len(), 1);
        assert!(view.leave_pending());
    }
    fn controller(session: &Session) -> CopyBatches {
        CopyBatches {
            binding: Some((session.client.base_url().to_owned(), session.tenant, 1)),
            loaded: true,
            ..Default::default()
        }
    }
    fn wait(view: &mut CopyBatches, session: &Session, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while view.task.is_some() {
            assert!(
                Instant::now() < deadline,
                "copy batch HTTP did not finish: {:?}",
                view.error
            );
            view.poll(session, 1, ctx, false);
            std::thread::sleep(Duration::from_millis(3));
        }
    }
    fn wait_inspection(view: &mut CopyBatches, session: &Session, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while view.inspection_task.is_some() {
            assert!(
                Instant::now() < deadline,
                "source inspection HTTP did not finish: {:?}",
                view.inspection_error
            );
            view.poll(session, 1, ctx, false);
            std::thread::sleep(Duration::from_millis(3));
        }
    }
    #[test]
    fn copy_batch_controller_lost_start_recovers_server_job_without_duplicate_start() {
        let pending = status(0, 2, 0, 0);
        let done = status(2, 0, 0, 0);
        let (client, server) = crate::remote_actions::tests::server(vec![
            None,
            Some((200, serde_json::to_string(&vec![pending]).unwrap())),
            Some((200, serde_json::to_string(&done).unwrap())),
        ]);
        let session = Session {
            client,
            tenant: RowId(1),
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let mut view = controller(&session);
        view.prepared = Some(StartBody {
            batch_id: RowId(55),
            token: RowId(9),
            excluded: vec![RowId(99)],
            confirmed_count: 2,
        });
        view.request(&session, Request::Start(RowId(55)), &ctx);
        wait(&mut view, &session, &ctx);
        assert!(view.prepared.is_some());
        assert!(view.error.is_some());
        view.request(&session, Request::List, &ctx);
        wait(&mut view, &session, &ctx);
        assert!(view.prepared.is_none());
        assert_eq!(view.jobs[0].pending, 2);
        assert!(view.running.is_none());
        view.request(&session, Request::Advance(RowId(55)), &ctx);
        wait(&mut view, &session, &ctx);
        assert!(view.jobs[0].terminal());
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with("POST "));
        assert!(requests[1].starts_with("GET "));
        assert!(requests[2].lines().next().unwrap().contains("/advance"));
        let original: StartBody =
            serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(original.batch_id, RowId(55));
        assert_eq!(original.excluded, vec![RowId(99)]);
    }
    #[test]
    fn copy_batch_controller_restart_lists_jobs_and_cancel_is_post() {
        let pending = status(0, 1, 1, 0);
        let cancelled = status(0, 0, 1, 1);
        let (client, server) = crate::remote_actions::tests::server(vec![
            Some((200, serde_json::to_string(&vec![pending]).unwrap())),
            Some((200, serde_json::to_string(&cancelled).unwrap())),
        ]);
        let session = Session {
            client,
            tenant: RowId(1),
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let mut view = controller(&session);
        view.request(&session, Request::List, &ctx);
        wait(&mut view, &session, &ctx);
        assert!(view.running.is_none());
        view.request(&session, Request::Cancel(RowId(55)), &ctx);
        wait(&mut view, &session, &ctx);
        assert_eq!(view.jobs[0].cancelled, 1);
        assert_eq!(view.jobs[0].unknown, 1);
        assert!(!view.jobs[0].terminal());
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("GET "));
        assert!(requests[1].starts_with("POST "));
        assert!(requests[1].lines().next().unwrap().contains("/cancel"));
    }

    #[test]
    fn source_inspection_rejects_mismatched_product_without_changing_result_state() {
        let tenant = RowId(1);
        let batch = RowId(55);
        let operation = RowId(20);
        let source = RowId(10);
        let wrong_product = RowId(11);
        let body = serde_json::json!({
            "product_id": wrong_product,
            "tenant_id": tenant,
            "product_code": "WRONG",
            "name": "wrong product",
            "brand": null,
            "category": null,
            "status": "ACTIVE",
            "created_at": "2026-09-10T00:00:00Z",
            "updated_at": "2026-09-10T00:00:00Z"
        });
        let (client, server) =
            crate::remote_actions::tests::server(vec![Some((200, body.to_string()))]);
        let session = Session {
            client,
            tenant,
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let mut view = controller(&session);
        view.results = Some(wire::BatchResults {
            batch_id: batch,
            items: vec![wire::BatchItem {
                ordinal: 0,
                source,
                operation_id: operation,
                state: "PENDING".into(),
                receipt: None,
            }],
            next_after: None,
        });
        view.result_state.row_selection.active = Some(operation);
        let key = InspectionKey {
            batch,
            operation,
            source,
        };
        view.start_inspection(&session, key, &ctx);
        wait_inspection(&mut view, &session, &ctx);

        assert!(view.inspection.is_none());
        assert!(view.inspection_error.is_some());
        assert_eq!(view.results.as_ref().unwrap().items[0].state, "PENDING");
        assert!(view.results.as_ref().unwrap().items[0].receipt.is_none());
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with(&format!(
            "GET /erp/v1/tenants/{}/products/{}?",
            tenant.to_uuid(),
            source.to_uuid()
        )));
    }

    #[test]
    fn source_inspection_discards_valid_late_response_after_result_selection_changes() {
        let tenant = RowId(1);
        let batch = RowId(55);
        let operation = RowId(20);
        let other_operation = RowId(21);
        let source = RowId(10);
        let body = serde_json::json!({
            "product_id": source,
            "tenant_id": tenant,
            "product_code": "SOURCE-10",
            "name": "source product",
            "brand": null,
            "category": null,
            "status": "ACTIVE",
            "created_at": "2026-09-10T00:00:00Z",
            "updated_at": "2026-09-10T00:00:00Z"
        });
        let (client, server) =
            crate::remote_actions::tests::server(vec![Some((200, body.to_string()))]);
        let session = Session {
            client,
            tenant,
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let mut view = controller(&session);
        view.results = Some(wire::BatchResults {
            batch_id: batch,
            items: vec![
                wire::BatchItem {
                    ordinal: 0,
                    source,
                    operation_id: operation,
                    state: "PENDING".into(),
                    receipt: None,
                },
                wire::BatchItem {
                    ordinal: 1,
                    source: RowId(11),
                    operation_id: other_operation,
                    state: "CANCELLED".into(),
                    receipt: None,
                },
            ],
            next_after: None,
        });
        view.result_state.row_selection.active = Some(operation);
        view.start_inspection(
            &session,
            InspectionKey {
                batch,
                operation,
                source,
            },
            &ctx,
        );

        // Change the selected result before the completed HTTP response is
        // consumed. Its old identity must prevent publication.
        view.result_state.row_selection.active = Some(other_operation);
        wait_inspection(&mut view, &session, &ctx);

        assert!(view.inspection.is_none());
        assert!(view.inspection_error.is_none());
        let results = view.results.as_ref().unwrap();
        assert_eq!(results.items[0].state, "PENDING");
        assert!(results.items[0].receipt.is_none());
        assert_eq!(results.items[1].state, "CANCELLED");
        assert!(results.items[1].receipt.is_none());
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with(&format!(
            "GET /erp/v1/tenants/{}/products/{}?",
            tenant.to_uuid(),
            source.to_uuid()
        )));
    }
}
