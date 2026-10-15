use super::Session;
use crate::http::{DownloadTask, HttpError, HttpRequest, HttpTask};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId, SortRule};
use erp_ui_egui::{DataGrid, GridState};
use serde::Deserialize;
use std::collections::BTreeSet;
mod create;

const PAGE_SIZE: u64 = 50;
const MAX_TEXT: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
struct ExportJob {
    id: RowId,
    job_type: String,
    status: String,
    progress: i32,
    attempts: i32,
    last_error: String,
    created_at: String,
    updated_at: String,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct WirePage {
    items: Vec<WireJob>,
    total: i64,
}

#[derive(Deserialize)]
struct WireJob {
    job_id: String,
    tenant_id: String,
    #[serde(rename = "type")]
    job_type: String,
    status: String,
    progress: i32,
    attempts: i32,
    last_error: String,
    created_at: String,
    updated_at: String,
    params: serde_json::Value,
    #[serde(rename = "output_url")]
    _ignored_output_url: serde_json::Value,
}

fn bounded_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => value.len() <= MAX_TEXT,
        serde_json::Value::Array(values) => values.iter().all(bounded_value),
        serde_json::Value::Object(values) => values
            .iter()
            .all(|(key, value)| key.len() <= MAX_TEXT && bounded_value(value)),
        _ => true,
    }
}

fn decode_page(tenant: RowId, bytes: &[u8]) -> Result<(Vec<ExportJob>, u64), String> {
    let page: WirePage = serde_json::from_slice(bytes).map_err(|_| "导出任务响应无效")?;
    if page.items.len() > PAGE_SIZE as usize || page.total < 0 {
        return Err("导出任务响应超出限制".into());
    }
    let total = u64::try_from(page.total).map_err(|_| "导出任务总数无效")?;
    let mut ids = BTreeSet::new();
    let mut jobs = Vec::with_capacity(page.items.len());
    for item in page.items {
        let id = RowId::from_uuid(&item.job_id).ok_or("导出任务身份无效")?;
        let item_tenant = RowId::from_uuid(&item.tenant_id).ok_or("商家身份无效")?;
        if item_tenant != tenant || !ids.insert(id) {
            return Err("导出任务身份不匹配或重复".into());
        }
        if !matches!(
            item.status.as_str(),
            "PENDING" | "RUNNING" | "DONE" | "FAILED"
        ) || !(0..=100).contains(&item.progress)
            || item.attempts < 0
            || [
                &item.job_type,
                &item.status,
                &item.last_error,
                &item.created_at,
                &item.updated_at,
            ]
            .iter()
            .any(|value| value.len() > MAX_TEXT)
            || !bounded_value(&item.params)
        {
            return Err("导出任务字段无效".into());
        }
        jobs.push(ExportJob {
            id,
            job_type: item.job_type,
            status: item.status,
            progress: item.progress,
            attempts: item.attempts,
            last_error: item.last_error,
            created_at: item.created_at,
            updated_at: item.updated_at,
            params: item.params,
        });
    }
    Ok((jobs, total))
}

fn download_path(id: RowId) -> String {
    format!("/exports/jobs/{}/file", id.to_uuid())
}

#[derive(Default)]
pub(super) struct Exports {
    creation: create::Create,
    open: bool,
    binding: Option<(String, RowId, u64)>,
    jobs: Vec<ExportJob>,
    total: u64,
    offset: u64,
    status: Option<String>,
    task: Option<HttpTask>,
    download: Option<DownloadTask>,
    loaded: bool,
    next_poll: f64,
    feedback: Option<String>,
    download_feedback: Option<String>,
    permission_expired: bool,
    state: GridState,
    columns: Vec<Column>,
}

impl Exports {
    pub fn creating(&self) -> bool {
        self.creation.pending()
    }

    pub fn create(
        &mut self,
        session: &Session,
        request: &erp_grid_core::remote::PageRequest,
        ctx: &egui::Context,
    ) {
        self.open = true;
        self.creation.begin(session, request, ctx);
    }

    pub fn create_frozen(
        &mut self,
        session: &Session,
        token: RowId,
        excluded: &BTreeSet<RowId>,
        sorts: &[SortRule],
        ctx: &egui::Context,
    ) {
        self.open = true;
        self.creation
            .begin_frozen(session, token, excluded, sorts, ctx);
    }

    pub fn create_checked(
        &mut self,
        session: &Session,
        checked: &BTreeSet<RowId>,
        sorts: &[SortRule],
        ctx: &egui::Context,
    ) {
        self.open = true;
        self.creation.begin_checked(session, checked, sorts, ctx);
    }
    pub fn open(&mut self) {
        self.open = true;
    }

    pub(super) fn suspend_reads(&mut self) {
        self.task = None;
        self.open = false;
        self.loaded = false;
        self.next_poll = 0.0;
    }

    /// File delivery outlives the visible jobs window, but not its account scope.
    pub(super) fn tick_download(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.download.as_mut().and_then(DownloadTask::poll) {
            self.download = None;
            match result {
                Ok(location) => self.download_feedback = Some(format!("下载完成：{location}")),
                Err(HttpError::Status(401 | 403)) => {
                    self.permission_expired = true;
                    self.download_feedback = Some("下载权限已失效".into());
                }
                Err(error) => self.download_feedback = Some(error.to_string()),
            }
        }
        if self.download.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn busy(&self) -> bool {
        self.download.is_some()
    }

    fn request(&mut self, session: &Session, ctx: &egui::Context) {
        if self.task.is_some() || !self.open {
            return;
        }
        let mut query = vec![
            ("offset".into(), self.offset.to_string()),
            ("limit".into(), PAGE_SIZE.to_string()),
        ];
        if let Some(status) = &self.status {
            query.push(("status".into(), status.clone()));
        }
        match session.client.start(
            HttpRequest {
                path: "/exports/jobs".into(),
                query,
                json: None,
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            ctx,
        ) {
            Ok(task) => {
                self.task = Some(task);
                self.feedback = None;
            }
            Err(error) => {
                self.feedback = Some(error.to_string());
                self.next_poll = ctx.input(|input| input.time) + 2.0;
            }
        }
    }

    fn begin_download(&mut self, session: &Session, id: RowId, ctx: &egui::Context) {
        let filename = format!("erp-export-{}.csv", id.to_uuid());
        match session.client.download(
            HttpRequest {
                path: download_path(id),
                query: Vec::new(),
                json: None,
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            &filename,
            ctx,
        ) {
            Ok(task) => {
                self.download = Some(task);
                self.download_feedback = Some("正在下载".into());
            }
            Err(error) => self.download_feedback = Some(error.to_string()),
        }
    }

    fn poll(&mut self, session: &Session, generation: u64, ctx: &egui::Context) {
        let binding = (
            session.client.base_url().to_owned(),
            session.tenant,
            generation,
        );
        if self.binding.is_none() {
            self.binding = Some(binding);
        } else if self.binding.as_ref() != Some(&binding) {
            self.clear();
            self.binding = Some(binding);
            return;
        }
        if let Some(result) = self.task.as_mut().and_then(HttpTask::poll) {
            self.task = None;
            match result {
                Ok(bytes) => match decode_page(session.tenant, &bytes) {
                    Ok((jobs, total)) => {
                        let active = self.state.row_selection.active;
                        self.jobs = jobs;
                        self.total = total;
                        self.loaded = true;
                        self.state.clear_selection();
                        self.state.row_selection.active =
                            active.filter(|id| self.jobs.iter().any(|job| job.id == *id));
                        self.state
                            .row_selection
                            .retain_checked(|id| self.jobs.iter().any(|job| job.id == *id));
                        self.feedback = None;
                        self.next_poll = ctx.input(|input| input.time) + 2.0;
                    }
                    Err(error) => {
                        self.feedback = Some(error);
                        self.next_poll = ctx.input(|input| input.time) + 2.0;
                    }
                },
                Err(HttpError::Status(401 | 403)) => self.permission_expired = true,
                Err(error) => {
                    self.feedback = Some(error.to_string());
                    self.next_poll = ctx.input(|input| input.time) + 2.0;
                }
            }
        }
        self.tick_download(ctx);
        let now = ctx.input(|input| input.time);
        let unfinished = self
            .jobs
            .iter()
            .any(|job| matches!(job.status.as_str(), "PENDING" | "RUNNING"));
        if self.open && self.task.is_none() && now >= self.next_poll && (!self.loaded || unfinished)
        {
            self.request(session, ctx);
        }
        if self.open && (self.task.is_some() || self.download.is_some() || unfinished) {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    pub(super) fn ui(&mut self, ui: &mut egui::Ui, session: &Session, generation: u64) -> bool {
        self.poll(session, generation, ui.ctx());
        if !self.open {
            return std::mem::take(&mut self.permission_expired);
        }
        let ctx = ui.ctx().clone();
        egui::Window::new("导出任务")
            .id(egui::Id::new("remote-export-jobs"))
            .default_size(egui::vec2(980.0, 430.0))
            .show(&ctx, |ui| self.window_ui(ui, session));
        std::mem::take(&mut self.permission_expired)
    }

    fn window_ui(&mut self, ui: &mut egui::Ui, session: &Session) {
        let (created, expired) = self.creation.ui(ui, session);
        self.permission_expired |= expired;
        if created {
            self.loaded = false;
            self.offset = 0;
            self.status = None;
            self.next_poll = 0.0;
            self.task = None;
        }
        ui.horizontal(|ui| {
            let before = self.status.clone();
            egui::ComboBox::from_id_salt("export-status")
                .selected_text(self.status.as_deref().unwrap_or("全部状态"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.status, None, "全部状态");
                    for status in ["PENDING", "RUNNING", "DONE", "FAILED"] {
                        ui.selectable_value(&mut self.status, Some(status.into()), status);
                    }
                });
            if self.status != before {
                self.offset = 0;
                self.loaded = false;
                self.next_poll = 0.0;
                self.task = None;
                self.jobs.clear();
                self.total = 0;
                self.state.row_selection.clear();
                self.state.clear_selection();
            }
            if ui
                .add_enabled(self.task.is_none(), egui::Button::new("刷新"))
                .clicked()
            {
                self.loaded = false;
                self.next_poll = 0.0;
                self.request(session, ui.ctx());
            }
            if let Some(task) = &self.download {
                ui.label(format!("已下载 {} 字节", task.progress()));
                if ui.button("取消下载").clicked() {
                    task.cancel();
                    self.download = None;
                    self.download_feedback = Some("下载已取消".into());
                }
            }
            if ui
                .button("关闭")
                .on_hover_text(if self.download.is_some() {
                    "关闭窗口并取消当前下载"
                } else {
                    "关闭导出任务"
                })
                .clicked()
            {
                if let Some(task) = self.download.take() {
                    task.cancel();
                    self.download_feedback = Some("下载已取消".into());
                }
                self.task = None;
                self.open = false;
            }
        });

        self.state.read_only = true;
        self.state.filtering_enabled = false;
        self.state.sorting_enabled = false;
        self.state.row_number_start = self.offset;
        if self.columns.is_empty() {
            self.columns = [
                ("任务", 230.0),
                ("类型", 150.0),
                ("状态", 100.0),
                ("进度", 80.0),
                ("尝试", 70.0),
                ("错误", 240.0),
                ("创建时间", 180.0),
                ("更新时间", 180.0),
                ("参数", 260.0),
            ]
            .into_iter()
            .enumerate()
            .map(|(index, (title, width))| Column::new(ColumnId(index as u64), title, width))
            .collect();
        }
        ui.allocate_ui(egui::vec2(ui.available_width(), 300.0), |ui| {
            DataGrid {
                id: ui.id().with("export-jobs-grid"),
                columns: &mut self.columns,
                rows: self.jobs.len(),
                state: &mut self.state,
                row_id: Some(&|row| self.jobs[row].id),
            }
            .show(ui, |row, column| {
                let job = &self.jobs[row];
                match column.0 {
                    0 => job.id.to_uuid(),
                    1 => job.job_type.clone(),
                    2 => job.status.clone(),
                    3 => format!("{}%", job.progress),
                    4 => job.attempts.to_string(),
                    5 => job.last_error.clone(),
                    6 => job.created_at.clone(),
                    7 => job.updated_at.clone(),
                    8 => job.params.to_string(),
                    _ => String::new(),
                }
            });
        });

        ui.horizontal(|ui| {
            let selected = self
                .state
                .row_selection
                .active
                .and_then(|id| self.jobs.iter().find(|job| job.id == id))
                .filter(|job| job.status == "DONE")
                .map(|job| job.id);
            if ui
                .add_enabled(
                    selected.is_some() && self.download.is_none() && self.task.is_none(),
                    egui::Button::new("下载所选结果"),
                )
                .clicked()
                && let Some(id) = selected
            {
                self.begin_download(session, id, ui.ctx());
            }
            let end = self.offset.saturating_add(self.jobs.len() as u64);
            ui.label(format!(
                "{}–{} / {}",
                if self.total == 0 { 0 } else { self.offset + 1 },
                end,
                self.total
            ));
            if ui
                .add_enabled(
                    self.task.is_none() && self.offset > 0,
                    egui::Button::new("上一页"),
                )
                .clicked()
            {
                self.offset = self.offset.saturating_sub(PAGE_SIZE);
                self.loaded = false;
                self.next_poll = 0.0;
                self.jobs.clear();
                self.state.row_selection.clear();
                self.state.clear_selection();
            }
            if ui
                .add_enabled(
                    self.task.is_none() && self.offset.saturating_add(PAGE_SIZE) < self.total,
                    egui::Button::new("下一页"),
                )
                .clicked()
            {
                self.offset = self.offset.saturating_add(PAGE_SIZE);
                self.loaded = false;
                self.next_poll = 0.0;
                self.jobs.clear();
                self.state.row_selection.clear();
                self.state.clear_selection();
            }
            if self.task.is_some() {
                ui.spinner();
            }
            if let Some(feedback) = &self.feedback {
                ui.label(feedback);
            }
            if let Some(feedback) = &self.download_feedback {
                ui.label(feedback);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaving_export_list_cancels_reads_and_retains_view_context() {
        let ctx = egui::Context::default();
        let client = crate::http::HttpClient::new("http://127.0.0.1:1").unwrap();
        let mut exports = Exports {
            open: true,
            loaded: true,
            offset: 100,
            status: Some("DONE".into()),
            download_feedback: Some("Previous download completed".into()),
            task: Some(
                client
                    .start(
                        HttpRequest {
                            path: "/fixture".into(),
                            query: vec![],
                            json: None,
                            bearer: None,
                            idempotency_key: None,
                        },
                        &ctx,
                    )
                    .unwrap(),
            ),
            ..Default::default()
        };
        exports.suspend_reads();
        assert!(exports.task.is_none());
        assert!(!exports.open);
        assert!(!exports.loaded);
        assert_eq!(exports.offset, 100);
        assert_eq!(exports.status.as_deref(), Some("DONE"));
        assert_eq!(
            exports.download_feedback.as_deref(),
            Some("Previous download completed")
        );
        exports.open();
        assert!(exports.open);
        assert!(!exports.loaded);
    }

    #[test]
    fn decoder_ignores_output_url_and_builds_download_path_from_validated_id() {
        let tenant = RowId(1);
        let job = RowId(2);
        let bytes = serde_json::to_vec(&serde_json::json!({
            "items": [{
                "job_id": job.to_uuid(), "tenant_id": tenant.to_uuid(),
                "type": "ORDER_EXPORT", "status": "DONE", "progress": 100,
                "attempts": 1, "last_error": "", "created_at": "2026-09-10T00:00:00Z",
                "updated_at": "2026-09-10T00:00:01Z", "params": {},
                "output_url": "https://attacker.invalid/stolen.csv"
            }],
            "total": 1
        }))
        .unwrap();
        let (jobs, total) = decode_page(tenant, &bytes).unwrap();
        assert_eq!(total, 1);
        assert_eq!(jobs[0].id, job);
        assert_eq!(
            download_path(jobs[0].id),
            format!("/exports/jobs/{}/file", job.to_uuid())
        );
    }

    #[test]
    fn decoder_rejects_foreign_tenant_and_duplicate_ids() {
        let tenant = RowId(1);
        let job = RowId(2);
        let item = serde_json::json!({
            "job_id": job.to_uuid(), "tenant_id": RowId(3).to_uuid(),
            "type": "ORDER_EXPORT", "status": "DONE", "progress": 100,
            "attempts": 1, "last_error": "", "created_at": "now", "updated_at": "now",
            "params": {}, "output_url": null
        });
        let foreign =
            serde_json::to_vec(&serde_json::json!({ "items": [item.clone()], "total": 1 }))
                .unwrap();
        assert!(decode_page(tenant, &foreign).is_err());
        let mut own = item;
        own["tenant_id"] = serde_json::Value::String(tenant.to_uuid());
        let duplicate =
            serde_json::to_vec(&serde_json::json!({ "items": [own.clone(), own], "total": 2 }))
                .unwrap();
        assert!(decode_page(tenant, &duplicate).is_err());
    }
}
