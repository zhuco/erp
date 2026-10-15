//! A bounded write-ahead handshake. Storage errors stop writes until explicit reopen.
use super::detail::{DetailPanel, DetailSavePhase, DetailSaveRequest};
use super::detail_storage::{DetailPersistReceipt, DetailStore, LoadedDetailStore};
use crate::action_store::StorageTask;
use eframe::egui;

pub struct DetailPersistence {
    scope: String,
    backup: super::detail_backup::DetailBackup,
    store: Option<DetailStore>,
    opening: Option<StorageTask<LoadedDetailStore>>,
    writing: Option<(StorageTask<DetailPersistReceipt>, Option<DetailSaveRequest>)>,
    revision: u64,
    clear_needed: bool,
    conflict_confirming: bool,
    conflict: Option<super::detail_conflict::DetailConflict>,
    pub error: Option<String>,
}
impl DetailPersistence {
    pub fn new(scope: String) -> Self {
        Self {
            backup: super::detail_backup::DetailBackup::new(scope.clone()),
            scope,
            store: None,
            opening: None,
            writing: None,
            revision: 0,
            clear_needed: false,
            conflict_confirming: false,
            conflict: None,
            error: None,
        }
    }
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn from_loaded(loaded: LoadedDetailStore) -> Self {
        Self {
            backup: super::detail_backup::DetailBackup::new(loaded.store.scope().to_owned()),
            scope: loaded.store.scope().to_owned(),
            store: Some(loaded.store),
            opening: None,
            writing: None,
            revision: loaded.revision,
            clear_needed: false,
            conflict_confirming: false,
            conflict: None,
            error: None,
        }
    }
    pub fn show_backup(
        &mut self,
        ui: &mut egui::Ui,
        panel: &mut DetailPanel,
    ) -> Option<DetailSaveRequest> {
        self.backup.show(ui, panel)
    }
    pub fn resolve_backup(
        &mut self,
        request: &DetailSaveRequest,
        outcome: super::detail::DetailSaveOutcome,
    ) -> Result<(), String> {
        self.backup.resolve_original(request, outcome)
    }
    pub fn show_conflict(
        &mut self,
        ui: &mut egui::Ui,
        panel: &DetailPanel,
    ) -> Option<DetailSaveRequest> {
        match self
            .conflict
            .as_mut()
            .map(|conflict| conflict.show(ui, panel))
        {
            Some(Ok(Some(super::detail_conflict::ConflictAction::Lookup(request)))) => {
                Some(request)
            }
            Some(Ok(Some(super::detail_conflict::ConflictAction::ConfirmTerminal))) => {
                if let Err(error) = self.confirm_conflict(ui.ctx(), panel) {
                    self.error = Some(error);
                }
                None
            }
            Some(Ok(None)) => None,
            Some(Err(error)) => {
                self.error = Some(error);
                None
            }
            None => None,
        }
    }
    pub fn confirm_conflict(
        &mut self,
        ctx: &egui::Context,
        panel: &DetailPanel,
    ) -> Result<(), String> {
        if self.writing.is_some() || panel.has_unresolved_save() {
            return Err("当前保存仍未决，不能取舍两项原意图".into());
        }
        let conflict = self.conflict.as_mut().ok_or("没有保存冲突")?;
        if conflict.stored.has_unresolved_save() {
            return Err("存储原结果仍未知，不能清理".into());
        }
        let store = self.store.as_ref().ok_or("未加载恢复存储")?;
        self.writing = Some((
            store.commit(self.revision, conflict.stored.persistence_write()?, ctx)?,
            None,
        ));
        conflict.confirming = true;
        self.conflict_confirming = true;
        ctx.request_repaint();
        Ok(())
    }
    pub fn resolve_conflict(
        &mut self,
        request: &DetailSaveRequest,
        outcome: super::detail::DetailSaveOutcome,
    ) -> Result<(), String> {
        self.conflict
            .as_mut()
            .ok_or("没有存储冲突")?
            .resolve(request, outcome)
    }
    pub fn pending(&self) -> bool {
        self.backup.busy()
            || self.opening.is_some()
            || self.writing.is_some()
            || self.clear_needed
            || self.error.is_some()
    }
    pub fn terminal_changed(&mut self) {
        self.clear_needed = true;
    }
    /// Reopen does not replay a write; any stored original is restored asUnknown.
    pub fn reopen(&mut self) {
        if self.writing.is_none() {
            self.store = None;
            self.opening = None;
            self.error = None;
        }
    }
    /// Returns only a request whose exact write-ahead bytes were committed and read back.
    pub fn poll(
        &mut self,
        ctx: &egui::Context,
        panel: &mut DetailPanel,
    ) -> (Option<DetailSaveRequest>, bool) {
        self.backup.poll(ctx, panel);
        let mut restored = false;
        let mut send = None;
        if panel.scope() != self.scope {
            panel.storage_ready(false);
            self.error = Some("恢复协调器与当前面板作用域不匹配；原数据保留".into());
            return (None, false);
        }
        if self.store.is_none() && self.opening.is_none() && self.error.is_none() {
            match DetailStore::open(self.scope.clone(), ctx) {
                Ok(task) => self.opening = Some(task),
                Err(error) => self.error = Some(error),
            }
        }
        if let Some(result) = self.opening.as_mut().and_then(StorageTask::poll) {
            self.opening = None;
            match result {
                Ok(loaded) => {
                    self.revision = loaded.revision;
                    if loaded.store.scope() != self.scope {
                        self.error = Some("恢复存储作用域不匹配".into());
                    } else if let Some(bytes) = loaded.bytes {
                        match reconcile_checkpoint(&self.scope, panel, &bytes) {
                            Ok(changed) => restored = changed,
                            Err(error) => {
                                self.conflict = super::detail_conflict::DetailConflict::new(
                                    &self.scope,
                                    bytes,
                                    self.revision,
                                )
                                .ok();
                                self.error = Some(error);
                            }
                        }
                    }
                    self.store = Some(loaded.store);
                }
                Err(error) => self.error = Some(error),
            }
        }
        if let Some(result) = self.writing.as_mut().and_then(|(task, _)| task.poll()) {
            let request = self.writing.take().and_then(|(_, request)| request);
            match result {
                Ok(receipt) => {
                    self.revision = receipt.revision();
                    self.clear_needed = false;
                    if let Some(request) = request {
                        if self.error.is_some() {
                            panel.storage_ready(false);
                            return (None, restored);
                        }
                        match panel.authorize_send(&request, &receipt) {
                            Ok(()) => send = Some(request),
                            Err(error) => self.error = Some(error),
                        }
                    } else if self.conflict_confirming {
                        let same_record = self.conflict.as_ref().is_some_and(|conflict| {
                            conflict.stored.bound_record() == panel.bound_record()
                        });
                        self.conflict_confirming = false;
                        self.conflict = None;
                        self.error = None;
                        if same_record && panel.is_dirty() {
                            panel.require_latest();
                        }
                    } else {
                        self.clear_needed = false;
                    }
                }
                Err(error) => {
                    self.conflict_confirming = false;
                    if let Some(conflict) = self.conflict.as_mut() {
                        conflict.confirming = false;
                    }
                    self.error = Some(error);
                }
            }
        }
        if self.writing.is_none()
            && self.error.is_none()
            && let Some(store) = &self.store
        {
            let request = (panel.save_phase() == Some(DetailSavePhase::Checkpointing))
                .then(|| panel.pending_request().cloned())
                .flatten();
            if request.is_some() || self.clear_needed {
                match panel
                    .persistence_write()
                    .and_then(|write| store.commit(self.revision, write, ctx))
                {
                    Ok(task) => self.writing = Some((task, request)),
                    Err(error) => self.error = Some(error),
                }
            }
        }
        if self.opening.is_some() || self.writing.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
        panel.storage_ready(
            self.store.is_some()
                && self.opening.is_none()
                && self.writing.is_none()
                && !self.clear_needed
                && self.error.is_none(),
        );
        (send, restored)
    }
}

fn reconcile_checkpoint(
    scope: &str,
    panel: &mut DetailPanel,
    bytes: &[u8],
) -> Result<bool, String> {
    let restored = DetailPanel::restore(scope, bytes)?;
    if panel.scope() != scope {
        return Err("恢复面板作用域不匹配".into());
    }
    if !panel.is_dirty() {
        *panel = restored;
        return Ok(true);
    }
    if panel.checkpoint()?.as_deref() != Some(bytes) {
        return Err("恢复存储存在不同原意图；当前草稿和存储记录均保留，禁止发送".into());
    }
    // Retain current controls/schema and in-flight write. A prepared memory intent
    // found in storage is conservatively Unknown, never dispatched by reopening.
    if panel.save_phase() == Some(DetailSavePhase::Checkpointing) {
        let original = panel.pending_request().cloned().ok_or("缺少原保存")?;
        panel.resolve_save(
            &original,
            super::detail::DetailSaveOutcome::Unknown("已核对原检查点，请先回查原保存结果".into()),
        )?;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::detail::{DetailField, DetailSaveOutcome};
    fn panel(scope: &str, after: &str) -> DetailPanel {
        let mut panel = DetailPanel::new(scope.into());
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
        if !after.is_empty() {
            panel.edit_value("name", after.into()).expect("edit");
        }
        panel
    }
    #[test]
    fn detail_reconcile_wrong_panel_scope_never_opens_or_replaces() {
        let mut panel = panel("other", "draft");
        let before = panel.checkpoint().expect("bytes");
        let mut coordinator = DetailPersistence::new("owner".into());
        let (send, restored) = coordinator.poll(&egui::Context::default(), &mut panel);
        assert!(send.is_none());
        assert!(!restored);
        assert_eq!(panel.scope(), "other");
        assert_eq!(panel.checkpoint().expect("bytes"), before);
        assert!(coordinator.opening.is_none());
        assert!(coordinator.error.is_some());
    }
    #[test]
    fn detail_reconcile_different_stored_pending_retains_current_draft() {
        let mut stored = panel("owner", "stored edit");
        let original = stored.begin_save().expect("stored intent");
        let bytes = stored.checkpoint().expect("encode").expect("bytes");
        let mut current = panel("owner", "new local draft");
        let before = current.checkpoint().expect("draft bytes");
        assert!(reconcile_checkpoint("owner", &mut current, &bytes).is_err());
        assert_eq!(current.checkpoint().expect("bytes"), before);
        assert_eq!(
            DetailPanel::restore("owner", &bytes)
                .expect("stored intact")
                .pending_request(),
            Some(&original)
        );
    }
    #[test]
    fn detail_reconcile_different_pending_id_never_overwrites_current_original() {
        let mut stored = panel("owner", "draft");
        stored.begin_save().expect("intent");
        let bytes = stored.checkpoint().expect("bytes").expect("some");
        let mut current = panel("owner", "draft");
        let current_request = current.begin_save().expect("different identity");
        assert!(reconcile_checkpoint("owner", &mut current, &bytes).is_err());
        assert_eq!(current.pending_request(), Some(&current_request));
    }
    #[test]
    fn detail_reconcile_same_prepared_original_becomes_unknown_without_new_id() {
        let mut panel = panel("owner", "draft");
        let original = panel.begin_save().expect("intent");
        let bytes = panel.checkpoint().expect("bytes").expect("some");
        assert!(!reconcile_checkpoint("owner", &mut panel, &bytes).expect("match"));
        assert_eq!(panel.save_phase(), Some(DetailSavePhase::Unknown));
        assert_eq!(panel.pending_request(), Some(&original));
    }
    #[test]
    fn detail_reconcile_same_inflight_read_is_not_cancelled_by_reopen() {
        let mut panel = panel("owner", "draft");
        let original = panel.begin_save().expect("intent");
        panel
            .resolve_save(&original, DetailSaveOutcome::Unknown("unknown".into()))
            .expect("unknown");
        panel.begin_read().expect("in flight original read");
        let bytes = panel.checkpoint().expect("bytes").expect("some");
        assert!(!reconcile_checkpoint("owner", &mut panel, &bytes).expect("match"));
        assert_eq!(panel.save_phase(), Some(DetailSavePhase::Reading));
        assert_eq!(panel.pending_request(), Some(&original));
    }
    #[test]
    fn detail_reconcile_clean_panel_restores_original_as_unknown() {
        let mut stored = panel("owner", "draft");
        let original = stored.begin_save().expect("intent");
        let bytes = stored.checkpoint().expect("bytes").expect("some");
        let mut clean = panel("owner", "");
        assert!(reconcile_checkpoint("owner", &mut clean, &bytes).expect("install original"));
        assert_eq!(clean.save_phase(), Some(DetailSavePhase::Unknown));
        assert_eq!(clean.pending_request(), Some(&original));
    }
    #[test]
    fn detail_reconcile_corrupt_stored_bytes_leave_current_panel_intact() {
        let mut current = panel("owner", "draft");
        let before = current.checkpoint().expect("bytes");
        assert!(reconcile_checkpoint("owner", &mut current, b"invalid").is_err());
        assert_eq!(current.checkpoint().expect("bytes"), before);
    }
    #[test]
    fn detail_reconcile_foreign_stored_scope_is_rejected_before_mutation() {
        let mut stored = panel("foreign", "draft");
        stored.begin_save().expect("intent");
        let bytes = stored.checkpoint().expect("bytes").expect("some");
        let mut current = panel("owner", "");
        assert!(reconcile_checkpoint("owner", &mut current, &bytes).is_err());
        assert_eq!(current.scope(), "owner");
        assert!(!current.is_dirty());
    }
    #[test]
    fn detail_conflict_unknown_original_cannot_be_confirmed_or_discarded() {
        let mut original = panel("owner", "original");
        original.begin_save().expect("intent");
        let bytes = original.checkpoint().expect("bytes").expect("some");
        let current = panel("owner", "local");
        let before = current.checkpoint().expect("before");
        let mut coordinator = DetailPersistence::new("owner".into());
        coordinator.conflict = Some(
            super::super::detail_conflict::DetailConflict::new("owner", bytes, 1)
                .expect("conflict"),
        );
        assert!(
            coordinator
                .confirm_conflict(&egui::Context::default(), &current)
                .is_err()
        );
        assert_eq!(current.checkpoint().expect("after"), before);
        assert!(coordinator.writing.is_none());
    }
    #[test]
    fn detail_conflict_current_unresolved_is_not_abandoned_for_known_stored_result() {
        let mut original = panel("owner", "original");
        let request = original.begin_save().expect("intent");
        let bytes = original.checkpoint().expect("bytes").expect("some");
        let mut conflict = super::super::detail_conflict::DetailConflict::new("owner", bytes, 1)
            .expect("conflict");
        conflict
            .resolve(
                &request,
                DetailSaveOutcome::Rejected {
                    code: "VALIDATION".into(),
                    message: "known rejection".into(),
                },
            )
            .expect("terminal");
        let mut current = panel("owner", "local");
        let local = current.begin_save().expect("local unresolved");
        let mut coordinator = DetailPersistence::new("owner".into());
        coordinator.conflict = Some(conflict);
        assert!(
            coordinator
                .confirm_conflict(&egui::Context::default(), &current)
                .is_err()
        );
        assert_eq!(current.pending_request(), Some(&local));
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn detail_conflict_confirmed_terminal_cas_preserves_local_and_requires_latest() {
        use super::super::detail_storage::{DetailStore, wait_storage};
        let ctx = egui::Context::default();
        let mut original = panel("owner", "original");
        let request = original.begin_save().expect("intent");
        let bytes = original.checkpoint().expect("bytes").expect("some");
        let root = std::env::current_dir()
            .expect("cwd")
            .join("qa/acceptance/detail-conflict-native")
            .join(uuid::Uuid::new_v4().to_string());
        let loaded =
            wait_storage(DetailStore::open_at(root.clone(), "owner".into(), &ctx).expect("open"))
                .expect("loaded");
        wait_storage(
            loaded
                .store
                .commit(0, original.persistence_write().expect("write"), &ctx)
                .expect("commit"),
        )
        .expect("stored");
        let mut coordinator = DetailPersistence::from_loaded(LoadedDetailStore {
            store: loaded.store,
            revision: 1,
            bytes: Some(bytes.clone()),
        });
        let mut conflict = super::super::detail_conflict::DetailConflict::new("owner", bytes, 1)
            .expect("conflict");
        conflict
            .resolve(
                &request,
                DetailSaveOutcome::Committed {
                    version: "1".into(),
                    values: [("name".into(), "original".into())].into_iter().collect(),
                },
            )
            .expect("verified original");
        coordinator.conflict = Some(conflict);
        coordinator.error = Some("conflict".into());
        let mut current = panel("owner", "local");
        let before = current.checkpoint().expect("before");
        coordinator
            .confirm_conflict(&ctx, &current)
            .expect("explicit confirm");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while coordinator.writing.is_some() {
            let (send, _) = coordinator.poll(&ctx, &mut current);
            assert!(send.is_none());
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(coordinator.error.is_none());
        assert!(coordinator.conflict.is_none());
        assert!(current.is_dirty());
        assert!(current.begin_save().is_err());
        let local: serde_json::Value =
            serde_json::from_slice(&current.checkpoint().expect("after").expect("bytes"))
                .expect("json");
        let old: serde_json::Value =
            serde_json::from_slice(&before.expect("before bytes")).expect("json");
        assert_eq!(local["changes"], old["changes"]);
        assert_eq!(local["before"], old["before"]);
        let independent =
            wait_storage(DetailStore::open_at(root, "owner".into(), &ctx).expect("reopen"))
                .expect("read");
        assert_eq!(independent.revision, 2);
        assert!(independent.bytes.is_none());
    }
}
