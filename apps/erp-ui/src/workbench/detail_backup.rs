//! Explicit, independent local draft copy. Never authorizes an external write.
use super::detail::{DetailPanel, DetailSaveOutcome, DetailSavePhase, DetailSaveRequest};
use super::detail_storage::{DetailPersistReceipt, DetailStore, LoadedDetailStore};
use crate::action_store::StorageTask;
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId};
use erp_ui_egui::{DataGrid, GridState};

struct BackupView {
    state: DetailPanel,
    terminal: Option<&'static str>,
    record: String,
    version: String,
    operation: Option<uuid::Uuid>,
    rows: Vec<(String, String, String)>,
    columns: Vec<Column>,
    grid: GridState,
}

pub(super) struct DetailBackup {
    scope: String,
    enabled: bool,
    view_requested: bool,
    store: Option<DetailStore>,
    opening: Option<StorageTask<LoadedDetailStore>>,
    writing: Option<StorageTask<DetailPersistReceipt>>,
    requested: Option<Vec<u8>>,
    pub saved: Option<Vec<u8>>,
    revision: u64,
    pub error: Option<String>,
    pub confirmed: bool,
    view: Option<BackupView>,
    view_open: bool,
    clearing: bool,
    ended: bool,
}
impl DetailBackup {
    pub fn new(scope: String) -> Self {
        Self {
            scope,
            enabled: false,
            view_requested: false,
            store: None,
            opening: None,
            writing: None,
            requested: None,
            saved: None,
            revision: 0,
            error: None,
            confirmed: false,
            view: None,
            view_open: false,
            clearing: false,
            ended: false,
        }
    }
    pub fn busy(&self) -> bool {
        self.opening.is_some() || self.writing.is_some() || self.requested.is_some()
    }
    pub fn request_save(&mut self, panel: &DetailPanel) -> Result<(), String> {
        if panel.scope() != self.scope || self.busy() {
            return Err("草稿副本作用域不匹配或保存未完成".into());
        }
        self.enabled = true;
        self.requested = Some(panel.checkpoint()?.ok_or("没有需要保存的当前草稿")?);
        self.error = None;
        self.confirmed = false;
        self.ended = false;
        Ok(())
    }
    pub fn poll(&mut self, ctx: &egui::Context, panel: &DetailPanel) {
        if panel.scope() != self.scope {
            return;
        }
        if !self.enabled {
            return;
        }
        if self.store.is_none() && self.opening.is_none() && self.error.is_none() {
            match DetailStore::open_backup(self.scope.clone(), ctx) {
                Ok(task) => self.opening = Some(task),
                Err(error) => self.error = Some(error),
            }
        }
        if let Some(result) = self.opening.as_mut().and_then(StorageTask::poll) {
            self.opening = None;
            match result {
                Ok(loaded) => {
                    self.store = Some(loaded.store);
                    self.revision = loaded.revision;
                    self.saved = loaded.bytes;
                }
                Err(error) => self.error = Some(error),
            }
        }
        if let Some(result) = self.writing.as_mut().and_then(StorageTask::poll) {
            self.writing = None;
            match result {
                Ok(receipt) => {
                    self.revision = receipt.revision();
                    self.saved = self.requested.take();
                    self.confirmed = !self.clearing;
                    if self.clearing {
                        self.view = None;
                        self.view_open = false;
                        self.ended = true;
                        self.clearing = false;
                    }
                }
                Err(error) => {
                    self.requested = None;
                    self.clearing = false;
                    self.error = Some(error);
                }
            }
        }
        if self.writing.is_none()
            && self.opening.is_none()
            && self.error.is_none()
            && self.requested.is_some()
            && let Some(store) = &self.store
        {
            let bytes = self.requested.as_ref().cloned().unwrap_or_default();
            if self.saved.as_ref().is_some_and(|saved| saved != &bytes) {
                self.requested = None;
                self.error = Some("已有不同草稿副本，保留双方，未覆盖".into());
            } else {
                match DetailPanel::restore(&self.scope, &bytes)
                    .and_then(|restored| restored.persistence_write())
                    .and_then(|write| store.commit(self.revision, write, ctx))
                {
                    Ok(task) => self.writing = Some(task),
                    Err(error) => {
                        self.requested = None;
                        self.error = Some(error);
                    }
                }
            }
        }
        if self.view_requested
            && self.opening.is_none()
            && self.error.is_none()
            && self.store.is_some()
        {
            self.view_requested = false;
            if let Err(error) = self.open_view(panel) {
                self.error = Some(error);
            }
        }
        if self.error.is_some() && self.writing.is_none() && self.opening.is_none() {
            self.requested = None;
            self.view_requested = false;
        }
        if self.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
    fn open_view(&mut self, panel: &DetailPanel) -> Result<(), String> {
        let bytes = self.saved.as_ref().ok_or("没有草稿副本")?;
        let state = DetailPanel::restore(&self.scope, bytes)?;
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| "无法读取草稿副本")?;
        let record = value["record"].as_str().ok_or("缺少记录")?.to_owned();
        let version = value["version"].as_str().ok_or("缺少版本")?.to_owned();
        let changes = value["changes"].as_object().ok_or("缺少字段")?;
        let rows = changes
            .iter()
            .map(|(id, value)| {
                (
                    id.clone(),
                    panel.field_label(id),
                    value.as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let operation = value["pending"]["operation_id"]
            .as_str()
            .and_then(|id| uuid::Uuid::parse_str(id).ok());
        let mut grid = GridState::default();
        grid.read_only = true;
        grid.quick_search_open = false;
        grid.filtering_enabled = false;
        grid.sorting_enabled = false;
        self.view = Some(BackupView {
            state,
            terminal: None,
            record,
            version,
            operation,
            rows,
            columns: vec![
                Column::new(ColumnId(1), "字段", 150.0),
                Column::new(ColumnId(2), "草稿副本", 220.0),
                Column::new(ColumnId(3), "当前值", 220.0),
            ],
            grid,
        });
        self.view_open = true;
        Ok(())
    }
    pub fn begin_lookup(&mut self) -> Result<DetailSaveRequest, String> {
        if self.busy() {
            return Err("副本存储仍在处理".into());
        }
        self.view
            .as_mut()
            .ok_or("未打开草稿副本")?
            .state
            .begin_read()
    }
    pub fn resolve_original(
        &mut self,
        request: &DetailSaveRequest,
        outcome: DetailSaveOutcome,
    ) -> Result<(), String> {
        let view = self.view.as_mut().ok_or("未打开草稿副本")?;
        if view.state.save_phase() != Some(DetailSavePhase::Reading) {
            return Err("没有副本原结果回查".into());
        }
        let terminal = match &outcome {
            DetailSaveOutcome::Committed { .. } => "副本原结果已提交",
            DetailSaveOutcome::Rejected { .. } => "副本原结果已拒绝",
            DetailSaveOutcome::Unknown(_) => "副本原结果仍待核实",
        };
        view.state.resolve_save(request, outcome)?;
        view.terminal = Some(terminal);
        Ok(())
    }
    pub fn finish(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.busy() {
            return Err("副本存储仍在处理".into());
        }
        let view = self.view.as_ref().ok_or("未打开草稿副本")?;
        if view.state.has_unresolved_save() {
            return Err("副本原结果尚未确认，不能结束".into());
        }
        let mut write = view.state.persistence_write()?;
        if write.resolved.as_ref().map(|request| request.operation_id) != view.operation {
            return Err("缺少副本原终态证明".into());
        }
        write.bytes = None;
        let store = self.store.as_ref().ok_or("副本存储未加载")?;
        self.writing = Some(store.commit(self.revision, write, ctx)?);
        self.clearing = true;
        self.error = None;
        self.confirmed = false;
        ctx.request_repaint();
        Ok(())
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        panel: &mut DetailPanel,
    ) -> Option<DetailSaveRequest> {
        let mut lookup = false;
        let mut finish = false;
        let mut adopt = false;
        let busy = self.busy();
        if ui
            .add_enabled(
                panel.is_dirty() && !self.busy(),
                egui::Button::new("保存当前草稿副本"),
            )
            .clicked()
            && let Err(error) = self.request_save(panel)
        {
            self.error = Some(error);
        }
        if self.busy() {
            ui.label("草稿副本保存中");
        }
        if self.ended {
            ui.label("草稿副本已结束");
        }
        if self.confirmed {
            ui.label("草稿副本已提交并核对");
        }
        if ui
            .add_enabled(!self.busy(), egui::Button::new("查看草稿副本"))
            .clicked()
        {
            self.enabled = true;
            self.view_requested = true;
            self.error = None;
            if self.store.is_none() {
                ui.ctx().request_repaint();
            }
        }
        if self.view_open {
            egui::Window::new("草稿副本")
                .id(ui.id().with("detail-local-backup"))
                .open(&mut self.view_open)
                .resizable(true)
                .default_size([600.0, 300.0])
                .show(ui.ctx(), |ui| {
                    if let Some(view) = &mut self.view {
                        ui.label(format!(
                            "记录 {} · 版本 {} · revision {}",
                            view.record, view.version, self.revision
                        ));
                        if let Some(operation) = view.operation {
                            ui.label(format!("原本地操作 {}", operation));
                        }
                        if view.state.save_phase() == Some(DetailSavePhase::Unknown)
                            && ui
                                .add_enabled(!busy, egui::Button::new("回查副本原结果"))
                                .clicked()
                        {
                            lookup = true;
                        }
                        if view.state.save_phase() == Some(DetailSavePhase::Reading) {
                            ui.label("副本原结果回查中");
                        }
                        if let Some(status) = view.terminal {
                            ui.label(status);
                        }
                        if !view.state.has_unresolved_save() && view.state.is_dirty() {
                            ui.label(format!("当前记录 {} · 版本 {}", panel.bound_record().unwrap_or("—"), panel.bound_version()));
                            if ui.add_enabled(!busy && !panel.is_dirty() && panel.bound_record() == Some(view.record.as_str()), egui::Button::new("按当前版本采用副本值"))
                                .on_hover_text("仅采用字段值，当前已有修改或未决保存时不可用；不发送、不复制旧请求身份")
                                .clicked() { adopt = true; }
                        }
                        let finish_label = if view.operation.is_some() {
                            "确认原终态，结束此副本"
                        } else {
                            "确认结束此草稿副本"
                        };
                        if !view.state.has_unresolved_save()
                            && ui
                                .add_enabled(!busy, egui::Button::new(finish_label))
                                .clicked()
                        {
                            finish = true;
                        }
                        let height = ui.available_height().max(120.0);
                        ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
                            DataGrid {
                                id: ui.id().with("backup-data-grid"),
                                columns: &mut view.columns,
                                rows: view.rows.len(),
                                state: &mut view.grid,
                                row_id: Some(&|i| RowId(i as u128 + 1)),
                            }
                            .show(ui, |i, column| {
                                match column.0 {
                                    1 => view.rows[i].1.clone(),
                                    2 => view.rows[i].2.clone(),
                                    _ => panel.field_value(&view.rows[i].0).unwrap_or("—").to_owned(),
                                }
                            });
                        });
                    }
                });
        }

        if adopt && let Some(view) = &self.view {
            match panel.adopt_backup(&view.state) {
                Ok(()) => {
                    self.error = None;
                    self.view_open = false;
                    ui.ctx().request_repaint();
                }
                Err(error) => self.error = Some(error),
            }
        }
        if finish && let Err(error) = self.finish(ui.ctx()) {
            self.error = Some(error);
        }
        let request = if lookup {
            match self.begin_lookup() {
                Ok(request) => Some(request),
                Err(error) => {
                    self.error = Some(error);
                    None
                }
            }
        } else {
            None
        };
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }
        request
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::super::detail::DetailField;
    use super::super::detail_storage::wait_storage;
    use super::*;
    use std::path::PathBuf;

    fn panel() -> DetailPanel {
        let mut panel = DetailPanel::new("backup-lifecycle".into());
        panel.bind(
            "one",
            "0",
            &[DetailField {
                id: "name".into(),
                label: "Name".into(),
                value: "old".into(),
                editable: true,
                required: true,
            }],
        );
        panel.edit_value("name", "new".into()).expect("edit");
        panel
    }
    fn loaded() -> (DetailBackup, DetailPanel, PathBuf, egui::Context) {
        loaded_with_request(true)
    }
    fn loaded_with_request(pending: bool) -> (DetailBackup, DetailPanel, PathBuf, egui::Context) {
        let ctx = egui::Context::default();
        let root = std::env::current_dir()
            .expect("cwd")
            .join("qa/acceptance/detail-durable-native")
            .join(uuid::Uuid::new_v4().to_string());
        let mut current = panel();
        current.begin_save().expect("current request");
        let primary = wait_storage(
            DetailStore::open_at(root.clone(), current.scope().into(), &ctx).expect("primary"),
        )
        .expect("loaded");
        wait_storage(
            primary
                .store
                .commit(0, current.persistence_write().expect("primary write"), &ctx)
                .expect("primary commit"),
        )
        .expect("primary stored");
        let mut copy = panel();
        if pending {
            copy.begin_save().expect("copy request");
        }
        let store = wait_storage(
            DetailStore::open_backup_at(root.clone(), copy.scope().into(), &ctx).expect("backup"),
        )
        .expect("loaded backup");
        wait_storage(
            store
                .store
                .commit(0, copy.persistence_write().expect("copy write"), &ctx)
                .expect("copy commit"),
        )
        .expect("stored copy");
        let loaded = wait_storage(
            DetailStore::open_backup_at(root.clone(), copy.scope().into(), &ctx).expect("readback"),
        )
        .expect("read copy");
        let mut backup = DetailBackup::new(copy.scope().into());
        backup.enabled = true;
        backup.store = Some(loaded.store);
        backup.revision = loaded.revision;
        backup.saved = loaded.bytes;
        backup.open_view(&current).expect("view");
        (backup, current, root, ctx)
    }
    fn settle(backup: &mut DetailBackup, current: &DetailPanel, ctx: &egui::Context) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while backup.busy() {
            assert!(std::time::Instant::now() < deadline, "storage deadline");
            backup.poll(ctx, current);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    #[test]
    fn detail_backup_lifecycle_unknown_original_cannot_finish_or_rewrite() {
        let (mut backup, current, root, ctx) = loaded();
        let before = backup.saved.clone();
        assert!(backup.finish(&ctx).is_err());
        let request = backup.begin_lookup().expect("original lookup");
        backup
            .resolve_original(&request, DetailSaveOutcome::Unknown("uncertain".into()))
            .expect("unknown");
        assert!(backup.finish(&ctx).is_err());
        assert_eq!(backup.saved, before);
        let read = wait_storage(
            DetailStore::open_backup_at(root, current.scope().into(), &ctx).expect("read"),
        )
        .expect("stored");
        assert_eq!(read.revision, 1);
        assert_eq!(read.bytes, before);
    }
    #[test]
    fn detail_backup_lifecycle_committed_confirm_cas_preserves_primary_and_reuses_slot() {
        let (mut backup, current, root, ctx) = loaded();
        let peer = wait_storage(
            DetailStore::open_backup_at(root.clone(), current.scope().into(), &ctx).expect("peer"),
        )
        .expect("peer read");
        let request = backup.begin_lookup().expect("lookup");
        backup
            .resolve_original(
                &request,
                DetailSaveOutcome::Committed {
                    version: "1".into(),
                    values: [("name".into(), "new".into())].into_iter().collect(),
                },
            )
            .expect("terminal");
        assert_eq!(backup.revision, 1);
        assert!(backup.saved.is_some(), "terminal alone must not clear");
        backup.finish(&ctx).expect("explicit finish");
        settle(&mut backup, &current, &ctx);
        assert!(backup.error.is_none());
        assert!(backup.ended);
        assert_eq!(backup.revision, 2);
        let cleared = wait_storage(
            DetailStore::open_backup_at(root.clone(), current.scope().into(), &ctx)
                .expect("cleared read"),
        )
        .expect("cleared");
        assert_eq!(cleared.revision, 2);
        assert!(cleared.bytes.is_none());
        let primary = wait_storage(
            DetailStore::open_at(root, current.scope().into(), &ctx).expect("primary read"),
        )
        .expect("primary");
        assert_eq!(primary.revision, 1);
        assert_eq!(primary.bytes, current.checkpoint().expect("current bytes"));
        assert!(
            wait_storage(
                peer.store
                    .commit(1, current.persistence_write().expect("old write"), &ctx)
                    .expect("old commit")
            )
            .is_err()
        );
        let mut next = panel();
        next.edit_value("name", "next draft".into()).expect("next");
        backup.request_save(&next).expect("reuse");
        settle(&mut backup, &next, &ctx);
        assert!(backup.error.is_none());
        assert_eq!(backup.revision, 3);
        assert_eq!(backup.saved, next.checkpoint().expect("next bytes"));
    }
    #[test]
    fn detail_backup_lifecycle_rejected_original_requires_explicit_confirmation() {
        let (mut backup, current, _, ctx) = loaded();
        let request = backup.begin_lookup().expect("lookup");
        backup
            .resolve_original(
                &request,
                DetailSaveOutcome::Rejected {
                    code: "VERSION_CONFLICT".into(),
                    message: "changed".into(),
                },
            )
            .expect("authoritative rejection");
        assert!(backup.saved.is_some());
        assert_eq!(backup.revision, 1);
        backup.finish(&ctx).expect("explicit finish");
        settle(&mut backup, &current, &ctx);
        assert_eq!(backup.revision, 2);
        assert!(backup.saved.is_none());
    }
    #[test]
    fn detail_backup_lifecycle_foreign_or_changed_receipt_never_releases_copy() {
        let (mut backup, _, _, ctx) = loaded();
        let before = backup.saved.clone();
        let request = backup.begin_lookup().expect("lookup");
        let mut foreign = request.clone();
        foreign.operation_id = uuid::Uuid::new_v4();
        let outcome = DetailSaveOutcome::Rejected {
            code: "INVALID".into(),
            message: "no".into(),
        };
        assert!(backup.resolve_original(&foreign, outcome.clone()).is_err());
        let mut changed = request;
        changed.changes.insert("name".into(), "another".into());
        assert!(backup.resolve_original(&changed, outcome).is_err());
        assert!(backup.finish(&ctx).is_err());
        assert_eq!(backup.revision, 1);
        assert_eq!(backup.saved, before);
    }
    #[test]
    fn detail_backup_lifecycle_stale_finish_retains_copy_and_terminal_proof() {
        let (mut backup, current, root, ctx) = loaded();
        let before = backup.saved.clone();
        let request = backup.begin_lookup().expect("lookup");
        backup
            .resolve_original(
                &request,
                DetailSaveOutcome::Rejected {
                    code: "INVALID".into(),
                    message: "no".into(),
                },
            )
            .expect("terminal");
        let peer = wait_storage(
            DetailStore::open_backup_at(root, current.scope().into(), &ctx).expect("peer"),
        )
        .expect("peer read");
        let restored = DetailPanel::restore(current.scope(), before.as_ref().expect("bytes"))
            .expect("restore");
        wait_storage(
            peer.store
                .commit(
                    1,
                    restored.persistence_write().expect("same original"),
                    &ctx,
                )
                .expect("peer commit"),
        )
        .expect("peer stored");
        backup.finish(&ctx).expect("attempt finish");
        settle(&mut backup, &current, &ctx);
        assert!(backup.error.is_some());
        assert!(!backup.ended);
        assert_eq!(backup.saved, before);
        assert_eq!(backup.revision, 1);
        assert!(
            backup
                .view
                .as_ref()
                .expect("retained")
                .state
                .persistence_write()
                .expect("proof")
                .resolved
                .is_some()
        );
    }
    #[test]
    fn detail_backup_lifecycle_unsubmitted_draft_can_end_explicitly_without_primary_changes() {
        let (mut backup, current, root, ctx) = loaded_with_request(false);
        assert!(backup.begin_lookup().is_err());
        assert!(backup.saved.is_some());
        backup.finish(&ctx).expect("explicit draft confirmation");
        settle(&mut backup, &current, &ctx);
        assert!(backup.error.is_none());
        assert!(backup.ended);
        assert_eq!(backup.revision, 2);
        let primary = wait_storage(
            DetailStore::open_at(root, current.scope().into(), &ctx).expect("primary read"),
        )
        .expect("primary");
        assert_eq!(primary.revision, 1);
        assert_eq!(primary.bytes, current.checkpoint().expect("original"));
    }
}
