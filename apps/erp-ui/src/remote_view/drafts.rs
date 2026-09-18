//! Explicit draft storage and recovery; never sends business writes.
use super::{Session, draft_restore::Restore};
use crate::{
    action_store::{ActionStore, DraftStore, LoadedDraft, StorageTask},
    edit_drafts::{self, DraftCell, DraftSnapshot},
    remote_paste::Preview,
};
use eframe::egui;
use erp_grid_core::{Column, RowId};

pub(super) enum Intent {
    Save(Option<RowId>),
    Apply(Option<RowId>, Preview),
    ExpireSession,
}
#[derive(Default)]
pub(super) struct Drafts {
    generation: Option<u64>,
    parent: Option<RowId>,
    open: bool,
    opening: Option<StorageTask<LoadedDraft>>,
    store: Option<DraftStore>,
    revision: u64,
    stored: bool,
    snapshot: Option<DraftSnapshot>,
    saving: Option<(StorageTask<u64>, Option<Vec<u8>>)>,
    restore: Option<Restore>,
    message: Option<String>,
}
impl Drafts {
    pub(super) fn busy(&self) -> bool {
        self.opening.is_some() || self.saving.is_some() || self.restore.is_some()
    }
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn open(
        &mut self,
        store: &ActionStore,
        parent: Option<RowId>,
        generation: u64,
        ctx: &egui::Context,
    ) {
        if self.busy() {
            return;
        }
        self.clear();
        self.generation = Some(generation);
        self.parent = parent;
        self.open = true;
        let grid = parent.map_or_else(
            || "remote-products.main".to_owned(),
            |id| format!("remote-products.skus.{}", id.to_uuid()),
        );
        match store.open_drafts(&grid, 1, ctx) {
            Ok(task) => self.opening = Some(task),
            Err(error) => self.message = Some(error),
        }
    }
    pub(super) fn save(
        &mut self,
        rows: Result<Option<Vec<DraftCell>>, String>,
        ctx: &egui::Context,
    ) {
        let result = rows
            .and_then(|rows| rows.ok_or("当前没有可暂存的未提交草稿".into()))
            .and_then(|rows| {
                if rows.iter().any(|row| row.parent != self.parent) {
                    return Err("草稿与当前子表范围不匹配".into());
                }
                let store = self.store.as_ref().ok_or("草稿存储尚未就绪")?;
                edit_drafts::encode(store.scope(), super::unix_seconds(), &rows)
            });
        match result {
            Ok(bytes) => self.commit(Some(bytes), ctx),
            Err(error) => self.message = Some(error),
        }
    }
    fn commit(&mut self, bytes: Option<Vec<u8>>, ctx: &egui::Context) {
        if self.busy() {
            return;
        }
        let Some(store) = &self.store else {
            return;
        };
        match store.commit(self.revision, bytes.as_deref(), super::unix_seconds(), ctx) {
            Ok(task) => {
                self.saving = Some((task, bytes));
                self.message = None;
            }
            Err(error) => self.message = Some(error),
        }
    }
    pub(super) fn fail(&mut self, error: String) {
        self.message = Some(error);
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &Session,
        generation: u64,
        can_restore: bool,
        columns: &[Column],
    ) -> Option<Intent> {
        if self.generation.is_some_and(|old| old != generation) {
            self.clear();
        }
        if let Some(result) = self.opening.as_mut().and_then(StorageTask::poll) {
            self.opening = None;
            match result {
                Ok(loaded) => {
                    self.revision = loaded.revision;
                    self.stored = loaded.bytes.is_some();
                    self.snapshot =
                        loaded.bytes.as_ref().and_then(|bytes| {
                            match edit_drafts::decode(
                                loaded.store.scope(),
                                super::unix_seconds(),
                                bytes,
                            ) {
                                Ok(snapshot) => Some(snapshot),
                                Err(error) => {
                                    self.message = Some(error);
                                    None
                                }
                            }
                        });
                    self.store = Some(loaded.store);
                }
                Err(error) => self.message = Some(error),
            }
        }
        if let Some(result) = self.saving.as_mut().and_then(|(task, _)| task.poll())
            && let Some((_, bytes)) = self.saving.take()
        {
            match result {
                Ok(revision) => {
                    self.revision = revision;
                    self.stored = bytes.is_some();
                    self.snapshot =
                        self.store
                            .as_ref()
                            .zip(bytes.as_ref())
                            .and_then(|(store, bytes)| {
                                edit_drafts::decode(store.scope(), super::unix_seconds(), bytes)
                                    .ok()
                            });
                    self.message = Some(
                        if self.stored {
                            "草稿已暂存"
                        } else {
                            "暂存草稿已清除"
                        }
                        .into(),
                    );
                }
                Err(error) => {
                    self.message = Some(error);
                    self.store = None;
                }
            }
        }
        if !self.open {
            return None;
        }
        let mut intent = None;
        if let Some(restore) = &mut self.restore {
            match restore.tick(session, ui.ctx()) {
                Ok(Some(preview)) => {
                    self.restore = None;
                    if preview.commands.is_empty() {
                        self.message = Some("当前记录已达到草稿目标，无需保存".into());
                    } else {
                        intent = Some(Intent::Apply(self.parent, preview));
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    if restore.permission_lost() {
                        self.clear();
                        return Some(Intent::ExpireSession);
                    }
                    self.restore = None;
                    self.message = Some(error);
                }
            }
        }
        let mut open = true;
        egui::Window::new("草稿")
            .id(ui.id().with("remote-drafts"))
            .open(&mut open)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.parent.map_or_else(
                        || "商品".to_owned(),
                        |parent| format!("SKU · {}", parent.to_uuid()),
                    ));
                    if ui
                        .add_enabled(
                            self.store.is_some() && !self.busy(),
                            egui::Button::new("暂存当前草稿"),
                        )
                        .on_hover_text("保存到当前设备，保留七天；不提交服务器，不保存登录凭据")
                        .clicked()
                    {
                        intent = Some(Intent::Save(self.parent));
                    }
                    if ui
                        .add_enabled(
                            can_restore && self.snapshot.is_some() && !self.busy(),
                            egui::Button::new("读取最新并恢复"),
                        )
                        .on_hover_text("读取原记录最新版本后打开核对窗口，确认后才保存")
                        .clicked()
                        && let Some(snapshot) = self.snapshot.clone()
                    {
                        match Restore::new(snapshot, columns) {
                            Ok(restore) => self.restore = Some(restore),
                            Err(error) => self.message = Some(error),
                        }
                    }
                    if ui
                        .add_enabled(
                            self.stored && self.store.is_some() && !self.busy(),
                            egui::Button::new("删除暂存草稿"),
                        )
                        .clicked()
                    {
                        self.commit(None, ui.ctx());
                    }
                });
                if let Some(snapshot) = &self.snapshot {
                    ui.label(format!("{} 个单元格", snapshot.rows.len()));
                }
                if let Some(restore) = &self.restore {
                    let (done, total) = restore.progress();
                    ui.label(format!("读取 {done}/{total}"));
                    if ui.button("停止读取").clicked() {
                        self.restore = None;
                    }
                }
                if self.busy() {
                    ui.spinner();
                }
                if let Some(message) = &self.message {
                    ui.label(message);
                }
            });
        if !open {
            self.open = false;
            self.restore = None;
        }
        if self.busy() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(150));
        }
        intent
    }
    pub(super) fn parent(&self) -> Option<RowId> {
        self.parent
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::{http::HttpClient, remote_actions::ActionScope};
    use erp_grid_core::ColumnId;
    use std::time::{Duration, Instant};

    fn wait<T>(mut task: StorageTask<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = task.poll() {
                return result.unwrap();
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(3));
        }
    }
    fn settle(view: &mut Drafts, session: &Session, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while view.busy() {
            assert!(Instant::now() < deadline, "{:?}", view.message);
            let mut output = ctx.run_ui(egui::RawInput::default(), |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    assert!(view.ui(ui, session, 1, false, &[]).is_none());
                });
            });
            output.textures_delta.clear();
            std::thread::sleep(Duration::from_millis(3));
        }
    }
    #[test]
    fn draft_host_explicit_save_reopens_without_restoring_or_dispatching() {
        let root = tempfile::tempdir().unwrap();
        let client = HttpClient::new("http://127.0.0.1:1/erp").unwrap();
        let scope = ActionScope::new(&client, RowId(1), RowId(2)).unwrap();
        let session = Session {
            client,
            tenant: RowId(1),
            token: "never-persist".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let loaded = wait(ActionStore::open_at(root.path().into(), scope.clone(), &ctx).unwrap());
        let mut view = Drafts::default();
        view.open(&loaded.store, None, 1, &ctx);
        settle(&mut view, &session, &ctx);
        assert!(view.snapshot.is_none());
        view.save(
            Ok(Some(vec![DraftCell {
                row: RowId(10),
                column: ColumnId(1),
                before: "Before".into(),
                after: "Unsent".into(),
                expected_version: "2026-09-10T00:00:00Z".into(),
                parent: None,
            }])),
            &ctx,
        );
        // Closing the UI must still observe the in-flight storage acknowledgement.
        view.open = false;
        settle(&mut view, &session, &ctx);
        assert!(view.stored);
        assert!(view.restore.is_none());
        view.clear();
        crate::http::runtime()
            .unwrap()
            .block_on(loaded.store.close());
        let reopened = wait(ActionStore::open_at(root.path().into(), scope, &ctx).unwrap());
        view.open(&reopened.store, None, 1, &ctx);
        settle(&mut view, &session, &ctx);
        let snapshot = view.snapshot.as_ref().unwrap();
        assert_eq!(snapshot.rows[0].after, "Unsent");
        assert_eq!(snapshot.scope.actor, RowId(2));
        assert!(view.restore.is_none());
        assert!(!view.busy());
        view.commit(None, &ctx);
        settle(&mut view, &session, &ctx);
        assert!(!view.stored);
        assert!(view.snapshot.is_none());
        view.clear();
        crate::http::runtime()
            .unwrap()
            .block_on(reopened.store.close());
    }
}
