use super::Session;
use crate::{
    action_store::{ActionStore, LoadedStore, StorageTask},
    http::{HttpRequest, HttpTask},
    remote_actions::{ActionRunner, ActionScope, CheckpointTicket},
};
use eframe::egui;
use erp_grid_core::{
    RowId, RowSelection,
    actions::{ActionCommand, ActionOutcome, ActionSpec, ActionState},
};
use erp_ui_egui::{BatchActionIntent, BatchActionPanel};

#[derive(Default)]
pub(super) struct Intent {
    pub copy: bool,
    pub locate: Option<String>,
}

#[derive(Default)]
pub(super) struct ProductActions {
    profile: Option<HttpTask>,
    scope: Option<ActionScope>,
    opening: Option<StorageTask<LoadedStore>>,
    pub(super) store: Option<ActionStore>,
    revision: u64,
    saving: Option<(CheckpointTicket, StorageTask<u64>)>,
    pub(super) runner: Option<ActionRunner>,
    can_write: bool,
    storage_failed: bool,
    notice: Option<String>,
    authorized: bool,
    refresh_pending: bool,
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) test_root: Option<std::path::PathBuf>,
}
impl ProductActions {
    pub(super) fn actor(&self) -> Option<RowId> {
        self.authorized
            .then(|| self.scope.as_ref().map(ActionScope::actor))
            .flatten()
    }
    pub(super) fn start(&mut self, session: &Session, ctx: &egui::Context) {
        self.authorized = false;
        match session.client.start(
            HttpRequest {
                path: "/me".into(),
                query: vec![],
                json: None,
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            ctx,
        ) {
            Ok(task) => self.profile = Some(task),
            Err(error) => self.notice = Some(error.to_string()),
        }
    }
    fn open(&mut self, ctx: &egui::Context) {
        let Some(scope) = self.scope.clone() else {
            return;
        };
        #[cfg(all(test, not(target_arch = "wasm32")))]
        let result = match &self.test_root {
            Some(root) => ActionStore::open_at(root.clone(), scope, ctx),
            None => return, // Tests must explicitly own their storage directory.
        };
        #[cfg(not(all(test, not(target_arch = "wasm32"))))]
        let result = ActionStore::open(scope, ctx);
        match result {
            Ok(task) => {
                self.opening = Some(task);
                self.notice = None;
            }
            Err(error) => {
                self.notice = Some(error);
                self.storage_failed = true;
            }
        }
    }
    pub(super) fn tick(&mut self, session: &Session, ctx: &egui::Context) -> bool {
        #[derive(serde::Deserialize)]
        struct User {
            user_id: RowId,
            tenant_id: RowId,
            status: String,
        }
        #[derive(serde::Deserialize)]
        struct Profile {
            user: User,
            permissions: Vec<String>,
        }
        if let Some(result) = self.profile.as_mut().and_then(HttpTask::poll) {
            self.profile = None;
            let result = result.map_err(|error| error.to_string()).and_then(|bytes| {
                serde_json::from_slice::<Profile>(&bytes)
                    .map_err(|_| "Invalid account response".into())
            });
            match result {
                Ok(profile)
                    if profile.user.tenant_id == session.tenant
                        && profile.user.status == "ACTIVE" =>
                {
                    self.can_write = ["BASIC.SKU.READ", "BASIC.SKU.WRITE"]
                        .iter()
                        .all(|required| {
                            profile
                                .permissions
                                .iter()
                                .any(|permission| permission == required)
                        });
                    match ActionScope::new(&session.client, session.tenant, profile.user.user_id) {
                        Ok(scope) => {
                            if self
                                .scope
                                .as_ref()
                                .is_some_and(|previous| previous != &scope)
                            {
                                self.notice = Some("Account identity changed; pending work belongs to the original account".into());
                                return false;
                            }
                            self.authorized = true;
                            self.scope = Some(scope);
                            self.open(ctx);
                        }
                        Err(error) => self.notice = Some(error),
                    }
                }
                Ok(_) => self.notice = Some("Account does not match this merchant".into()),
                Err(error) => self.notice = Some(error),
            }
        }
        if let Some(result) = self.opening.as_mut().and_then(StorageTask::poll) {
            self.opening = None;
            match result {
                Ok(loaded) => {
                    self.revision = loaded.revision;
                    self.store = Some(loaded.store);
                    self.runner = match (self.scope.clone(), loaded.bytes) {
                        (Some(scope), Some(bytes)) => match ActionRunner::restore(scope, &bytes) {
                            Ok(mut runner) => {
                                if runner.items().iter().all(|item| {
                                    matches!(
                                        item.state(),
                                        ActionState::Complete(_) | ActionState::Cancelled
                                    )
                                }) {
                                    let _ = runner.resume();
                                }
                                Some(runner)
                            }
                            Err(error) => {
                                self.notice = Some(error);
                                self.storage_failed = true;
                                return false;
                            }
                        },
                        _ => None,
                    };
                    self.storage_failed = false;
                    self.notice = None;
                }
                Err(error) => {
                    self.notice = Some(error);
                    self.storage_failed = true;
                }
            }
        }
        if let Some(result) = self.saving.as_mut().and_then(|(_, task)| task.poll())
            && let Some((ticket, _)) = self.saving.take()
        {
            match result {
                Ok(revision) => {
                    self.revision = revision;
                    if let Some(runner) = &mut self.runner {
                        runner.acknowledge(&ticket, Ok(()));
                    }
                }
                Err(error) => {
                    if let Some(runner) = &mut self.runner {
                        runner.acknowledge(&ticket, Err(error.clone()));
                    }
                    self.notice = Some(error);
                    self.storage_failed = true;
                }
            }
        }
        if self.authorized
            && !self.storage_failed
            && self.opening.is_none()
            && let (Some(scope), Some(store), Some(runner)) =
                (&self.scope, &self.store, &mut self.runner)
        {
            runner.tick(&session.client, scope, &session.token, ctx);
            if let Some(receipt) = runner.take_receipt() {
                self.refresh_pending |= matches!(receipt.outcome, ActionOutcome::Applied { .. })
                    && runner.items().iter().any(|item| {
                        item.command().operation_id == receipt.operation_id
                            && item.command().action.starts_with("products.")
                    });
            }
            if self.saving.is_none()
                && let Some(write) = runner.checkpoint()
            {
                match store.commit(self.revision, write, ctx) {
                    Ok(task) => self.saving = Some((write.ticket.clone(), task)),
                    Err(error) => {
                        self.notice = Some(error);
                        self.storage_failed = true;
                    }
                }
            }
        }
        let changed = self.refresh_pending && self.settled();
        if changed {
            self.refresh_pending = false;
        }
        changed
    }
    fn settled(&self) -> bool {
        self.authorized
            && !self.storage_failed
            && self.opening.is_none()
            && self.saving.is_none()
            && self
                .runner
                .as_ref()
                .is_some_and(|runner| runner.checkpoint().is_none())
    }
    pub(super) fn outcome(&self, operation: RowId) -> Option<ActionState> {
        self.settled()
            .then(|| {
                self.runner
                    .as_ref()?
                    .items()
                    .iter()
                    .find(|item| item.command().operation_id == operation)
                    .map(|item| item.state().clone())
            })
            .flatten()
    }
    pub(super) fn outcomes(&self) -> std::collections::BTreeMap<RowId, ActionState> {
        if !self.settled() {
            return Default::default();
        }
        self.runner
            .as_ref()
            .map(|runner| {
                runner
                    .items()
                    .iter()
                    .map(|item| (item.command().operation_id, item.state().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    pub(super) fn suspend(&mut self) {
        self.authorized = false;
        if let Some(runner) = &mut self.runner {
            let _ = runner.pause();
        }
    }
    pub(super) fn update(
        &mut self,
        edit: &erp_grid_core::CellEdit,
        version: String,
    ) -> Result<RowId, String> {
        if edit.column.0 > 3 || (edit.column.0 == 1 && edit.after.trim().is_empty()) {
            return Err(
                "Only code, name, brand and category can be edited; name is required".into(),
            );
        }
        self.save_cell("products.update", edit, version)
    }
    pub(super) fn update_sku(
        &mut self,
        edit: &erp_grid_core::CellEdit,
        version: String,
    ) -> Result<RowId, String> {
        if !matches!(edit.column.0, 0 | 1 | 2 | 4 | 5)
            || (edit.column.0 == 0 && edit.after.trim().is_empty())
        {
            return Err("SKU column is read-only or code is empty".into());
        }
        if edit.column.0 == 4
            && erp_grid_core::money_minor(&edit.after)
                .map_err(str::to_owned)?
                .is_some_and(|value| value < 0)
        {
            return Err("Cost cannot be negative".into());
        }
        if edit.column.0 == 5 && !matches!(edit.after.as_str(), "true" | "false") {
            return Err("Choose true or false".into());
        }
        self.save_cell("skus.update", edit, version)
    }
    fn save_cell(
        &mut self,
        action: &str,
        edit: &erp_grid_core::CellEdit,
        version: String,
    ) -> Result<RowId, String> {
        if !self.available() {
            return Err("Finish or resolve the current operation first".into());
        }
        let operation = RowId(uuid::Uuid::new_v4().as_u128());
        let command = ActionCommand {
            operation_id: operation,
            action: action.into(),
            source: edit.row,
            expected_version: Some(version),
            values: [(edit.column, edit.after.clone())].into(),
        };
        let batch = erp_grid_core::actions::ActionBatch::new(vec![command])?;
        self.runner = Some(ActionRunner::new(
            self.scope.clone().ok_or("Missing account scope")?,
            batch,
        ));
        self.notice = None;
        Ok(operation)
    }
    pub(super) fn needs_attention(&self) -> bool {
        self.storage_failed
            || self.notice.is_some()
            || self
                .runner
                .as_ref()
                .is_some_and(|runner| !runner.finished())
    }
    pub(super) fn available(&self) -> bool {
        self.authorized
            && self.can_write
            && self.store.is_some()
            && self.opening.is_none()
            && !self.storage_failed
            && self.saving.is_none()
            && self.runner.as_ref().is_none_or(ActionRunner::finished)
    }
    pub(super) fn submit_paste(&mut self, commands: Vec<ActionCommand>) -> Result<(), String> {
        if !self.available() {
            return Err("请先恢复或结束当前操作".into());
        }
        if commands
            .iter()
            .any(|command| !matches!(command.action.as_str(), "products.update" | "skus.update"))
        {
            return Err("粘贴操作类型无效".into());
        }
        let batch = erp_grid_core::actions::ActionBatch::new(commands)?;
        self.runner = Some(ActionRunner::new(
            self.scope.clone().ok_or("Missing account scope")?,
            batch,
        ));
        self.notice = None;
        Ok(())
    }
    pub(super) fn copy(&mut self, selection: &RowSelection) {
        if !self.available() {
            return;
        }
        let Some(scope) = self.scope.clone() else {
            return;
        };
        match ActionSpec::new("products.copy", "复制商品", 10_000).freeze(selection, |source| {
            ActionCommand {
                operation_id: RowId(uuid::Uuid::new_v4().as_u128()),
                action: "products.copy".into(),
                source,
                expected_version: None,
                values: Default::default(),
            }
        }) {
            Ok(batch) => {
                self.runner = Some(ActionRunner::new(scope, batch));
                self.notice = None;
            }
            Err(error) => self.notice = Some(error),
        }
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &Session,
        ready: bool,
        targets: usize,
    ) -> Intent {
        let mut intent = Intent::default();
        let mut reopen = false;
        let mut retry_profile = false;
        ui.horizontal_wrapped(|ui| {
            intent.copy = ui
                .add_enabled(
                    ready && targets > 0 && self.available(),
                    egui::Button::new("复制商品"),
                )
                .on_hover_text(
                    "复制当前行；有勾选时复制全部勾选行。副本停用，条码和库存等经营数据不继承。",
                )
                .on_disabled_hover_text(self.notice.as_deref().unwrap_or(if !self.can_write {
                    "正在确认账号或无商品写入权限"
                } else {
                    "请选择商品，或先处理未完成操作"
                }))
                .clicked();
            if self.profile.is_some() || self.opening.is_some() || self.saving.is_some() {
                ui.spinner();
            }
            if self.storage_failed {
                reopen = ui.button("重新读取操作记录").clicked();
            }
            if self.profile.is_none() && !self.authorized && self.notice.is_some() {
                retry_profile = ui.button("重试账号确认").clicked();
            }
            if let Some(runner) = &mut self.runner {
                let success = runner
                    .items()
                    .iter()
                    .filter(|item| {
                        matches!(
                            item.state(),
                            ActionState::Complete(ActionOutcome::Applied { .. })
                        )
                    })
                    .count();
                let failed = runner
                    .items()
                    .iter()
                    .filter(|item| {
                        matches!(
                            item.state(),
                            ActionState::Complete(ActionOutcome::Rejected { .. })
                        )
                    })
                    .count();
                let unknown = runner
                    .items()
                    .iter()
                    .any(|item| item.state() == &ActionState::Unknown);
                let conflict = runner
                    .items()
                    .iter()
                    .any(|item| item.state() == &ActionState::IdentityConflict);
                let result_row = runner
                    .items()
                    .iter()
                    .rev()
                    .filter(|item| item.command().action == "products.copy")
                    .find_map(|item| match item.state() {
                        ActionState::Complete(ActionOutcome::Applied { row, .. }) => Some(*row),
                        _ => None,
                    });
                let failures: Vec<String> = runner
                    .items()
                    .iter()
                    .filter(|item| {
                        matches!(
                            item.state(),
                            ActionState::Complete(ActionOutcome::Rejected { .. })
                        )
                    })
                    .filter_map(|item| match item.state() {
                        ActionState::Complete(ActionOutcome::Rejected { message, .. }) => {
                            Some(message.clone())
                        }
                        _ => None,
                    })
                    .collect();
                let result_location =
                    result_row.map(|row| format!("COPY-{}", row.to_uuid().replace('-', "")));
                let failure_refs: Vec<&str> = failures.iter().map(String::as_str).collect();
                let panel_intent = BatchActionPanel {
                    total: runner.items().len(),
                    success,
                    failed,
                    paused: runner.paused(),
                    storage_failed: self.storage_failed,
                    authorized: self.authorized && !conflict,
                    can_write: self.can_write,
                    has_unknown: unknown,
                    has_conflict: conflict,
                    ready: runner
                        .items()
                        .iter()
                        .any(|item| item.state() == &ActionState::Ready),
                    can_pause: !runner.finished() && self.saving.is_none(),
                    result_location: result_location.as_deref(),
                    locate_label: Some("定位副本"),
                    disabled_reason: Some("需要商品读取与写入权限，核实后可能重试原操作"),
                    notice: runner.notice(),
                    failures: &failure_refs,
                }
                .show(ui);
                match panel_intent {
                    Some(BatchActionIntent::VerifyUnknown) => {
                        let result = self
                            .scope
                            .as_ref()
                            .ok_or("Missing account scope".into())
                            .and_then(|scope| {
                                runner.lookup(&session.client, scope, &session.token, ui.ctx())
                            });
                        if let Err(error) = result {
                            self.notice = Some(error);
                        }
                        ui.ctx().request_repaint();
                    }
                    Some(BatchActionIntent::Continue) => {
                        if let Err(error) = runner.resume().map_err(str::to_owned) {
                            self.notice = Some(error);
                        }
                        ui.ctx().request_repaint();
                    }
                    Some(BatchActionIntent::CancelUnstarted) => {
                        if let Err(error) = runner.cancel_unstarted() {
                            self.notice = Some(error.into());
                        }
                    }
                    Some(BatchActionIntent::Pause) => {
                        if let Err(error) = runner.pause() {
                            self.notice = Some(error.into());
                        }
                    }
                    Some(BatchActionIntent::LocateResult) => {
                        // This product adapter owns the server's COPY-UUID code contract.
                        intent.locate = result_location;
                    }
                    None => {}
                }
            }
            if let Some(error) = &self.notice {
                ui.label(error);
            }
        });
        if reopen && self.saving.is_none() {
            self.open(ui.ctx());
        }
        if retry_profile {
            self.start(session, ui.ctx());
        }
        intent
    }
}
