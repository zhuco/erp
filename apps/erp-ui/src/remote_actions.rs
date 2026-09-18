//! Serial business-action HTTP with a mandatory write-ahead storage handshake.
//! The host owns durable storage and authenticated identity. No token enters a checkpoint.
use crate::http::{HttpClient, HttpError, HttpRequest, HttpTask, MAX_BODY};
use eframe::egui;
use erp_grid_core::{RowId, actions::*};
use std::sync::Arc;

pub(crate) const MAX_CHECKPOINT: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionScope {
    server: String,
    tenant: RowId,
    actor: RowId,
}
impl ActionScope {
    pub(crate) fn server(&self) -> &str {
        &self.server
    }
    pub(crate) fn tenant(&self) -> RowId {
        self.tenant
    }
    pub(crate) fn actor(&self) -> RowId {
        self.actor
    }
    pub(crate) fn storage_key(&self) -> Result<String, String> {
        serde_json::to_string(self)
            .map(|value| format!("scope-v1:{value}"))
            .map_err(|_| "Cannot encode action scope".into())
    }
    /// Actor and tenant must come from the authenticated server response.
    pub fn new(client: &HttpClient, tenant: RowId, actor: RowId) -> Result<Self, String> {
        if tenant.0 == 0 || actor.0 == 0 || client.base_url().len() > 4096 {
            return Err("Invalid authenticated action scope".into());
        }
        Ok(Self {
            server: client.base_url().into(),
            tenant,
            actor,
        })
    }
}

#[derive(Clone)]
pub struct CheckpointTicket {
    owner: Arc<()>,
    revision: u64,
}
pub struct CheckpointWrite {
    pub ticket: CheckpointTicket,
    pub bytes: Vec<u8>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u16,
    scope: ActionScope,
    batch: ActionCheckpoint,
}
enum AfterSave {
    Continue,
    Pause,
    Send(ActionCommand),
}
enum Phase {
    Ready,
    Paused,
    Saving(AfterSave),
    Send(ActionCommand),
    Finished,
}
enum RequestKind {
    Execute,
    Lookup,
}
struct Pending {
    command: ActionCommand,
    kind: RequestKind,
    task: HttpTask,
}

pub struct ActionRunner {
    scope: ActionScope,
    batch: ActionBatch,
    phase: Phase,
    owner: Arc<()>,
    revision: u64,
    checkpoint: Option<CheckpointWrite>,
    pending: Option<Pending>,
    receipt: Option<ActionReceipt>,
    notice: Option<String>,
}
impl ActionRunner {
    /// A storage adapter must never replace unresolved commands with a new batch.
    pub(crate) fn validate_storage_change(
        scope: &ActionScope,
        before: Option<&[u8]>,
        after: &[u8],
    ) -> Result<(), String> {
        let next = Self::restore(scope.clone(), after)?;
        let Some(before) = before else {
            return Ok(());
        };
        let previous = Self::restore(scope.clone(), before)?;
        let same_commands = previous.items().len() == next.items().len()
            && previous
                .items()
                .iter()
                .zip(next.items())
                .all(|(a, b)| a.command() == b.command());
        if !same_commands {
            if !previous.items().iter().all(|item| {
                matches!(
                    item.state(),
                    ActionState::Complete(_) | ActionState::Cancelled
                )
            }) {
                return Err(
                    "Unresolved actions must be recovered before starting a new batch".into(),
                );
            }
            let old: std::collections::BTreeSet<_> = previous
                .items()
                .iter()
                .map(|item| item.command().operation_id)
                .collect();
            if next
                .items()
                .iter()
                .any(|item| old.contains(&item.command().operation_id))
            {
                return Err("New action batch reuses an old operation identity".into());
            }
            return Ok(());
        }
        for (old, new) in previous.items().iter().zip(next.items()) {
            let allowed = match old.state() {
                ActionState::Ready => matches!(
                    new.state(),
                    ActionState::Ready | ActionState::Unknown | ActionState::Cancelled
                ),
                ActionState::Unknown | ActionState::Sending | ActionState::RetryReady => {
                    !matches!(new.state(), ActionState::Ready | ActionState::Cancelled)
                }
                terminal => terminal == new.state(),
            };
            if !allowed {
                return Err("Action recovery state cannot move backwards".into());
            }
        }
        Ok(())
    }
    pub fn new(scope: ActionScope, batch: ActionBatch) -> Self {
        Self {
            scope,
            batch,
            phase: Phase::Ready,
            owner: Arc::new(()),
            revision: 0,
            checkpoint: None,
            pending: None,
            receipt: None,
            notice: None,
        }
    }
    pub fn restore(scope: ActionScope, bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_CHECKPOINT {
            return Err("Action checkpoint exceeds 16 MiB".into());
        }
        let journal: Journal =
            serde_json::from_slice(bytes).map_err(|_| "Invalid action checkpoint")?;
        if journal.version != 1 || journal.scope != scope {
            return Err("Action checkpoint scope or version mismatch".into());
        }
        let mut runner = Self::new(scope, ActionBatch::restore(journal.batch)?);
        // Restoring a file never starts writes without an explicit resume/readback.
        runner.phase = Phase::Paused;
        Ok(runner)
    }
    pub fn items(&self) -> &[ActionItem] {
        self.batch.items()
    }
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }
    pub fn finished(&self) -> bool {
        matches!(self.phase, Phase::Finished)
    }
    pub fn paused(&self) -> bool {
        matches!(self.phase, Phase::Paused) && self.pending.is_none() && self.checkpoint.is_none()
    }
    pub fn checkpoint(&self) -> Option<&CheckpointWrite> {
        self.checkpoint.as_ref()
    }
    pub fn take_receipt(&mut self) -> Option<ActionReceipt> {
        self.receipt.take()
    }

    fn save(&mut self, after: AfterSave) {
        let encoded = serde_json::to_vec(&Journal {
            version: 1,
            scope: self.scope.clone(),
            batch: self.batch.checkpoint(),
        });
        match encoded {
            Ok(bytes) if bytes.len() <= MAX_CHECKPOINT => {
                self.revision += 1;
                self.checkpoint = Some(CheckpointWrite {
                    ticket: CheckpointTicket {
                        owner: self.owner.clone(),
                        revision: self.revision,
                    },
                    bytes,
                });
                self.phase = Phase::Saving(after);
            }
            _ => {
                self.phase = Phase::Paused;
                self.notice =
                    Some("Cannot encode action recovery data; no write dispatched".into());
            }
        }
    }
    /// Acknowledge only after an atomic durable commit. On failure keep the same
    /// checkpoint pending; neither retrying storage nor late acknowledgements send HTTP.
    pub fn acknowledge(&mut self, ticket: &CheckpointTicket, result: Result<(), String>) -> bool {
        let Some(write) = &self.checkpoint else {
            return false;
        };
        if !Arc::ptr_eq(&write.ticket.owner, &ticket.owner)
            || write.ticket.revision != ticket.revision
        {
            return false;
        }
        if let Err(error) = result {
            self.notice = Some(format!(
                "Recovery storage failed: {}",
                error.chars().take(128).collect::<String>()
            ));
            return true;
        }
        self.checkpoint = None;
        self.notice = None;
        let Phase::Saving(after) = std::mem::replace(&mut self.phase, Phase::Paused) else {
            return false;
        };
        self.phase = match after {
            AfterSave::Continue => Phase::Ready,
            AfterSave::Pause => Phase::Paused,
            AfterSave::Send(command) => Phase::Send(command),
        };
        if matches!(self.phase, Phase::Paused) {
            if self
                .batch
                .items()
                .iter()
                .any(|item| item.state() == &ActionState::IdentityConflict)
            {
                self.notice = Some("Operation identity conflict; manual review required".into());
            } else if self
                .batch
                .items()
                .iter()
                .any(|item| item.state() == &ActionState::Unknown)
            {
                self.notice = Some("Result unknown; look up original receipt".into());
            }
        }
        true
    }
    /// Host cancellation cannot undo an HTTP write. Its identity remains recoverable.
    pub fn pause(&mut self) -> Result<(), &'static str> {
        if self.checkpoint.is_some() {
            return Err("Wait for recovery storage before pausing");
        }
        if let Some(pending) = self.pending.take() {
            self.batch.unknown(pending.command.operation_id);
            self.save(AfterSave::Pause);
        } else if let Phase::Send(command) = &self.phase {
            self.batch.unknown(command.operation_id);
            self.save(AfterSave::Pause);
        } else {
            self.phase = Phase::Paused;
        }
        Ok(())
    }
    pub fn cancel_unstarted(&mut self) -> Result<(), &'static str> {
        if self.pending.is_some()
            || self.checkpoint.is_some()
            || !matches!(self.phase, Phase::Ready | Phase::Paused)
        {
            return Err("Pause the executor before cancelling unstarted items");
        }
        self.batch.cancel_pending();
        self.save(AfterSave::Pause);
        Ok(())
    }
    pub fn resume(&mut self) -> Result<(), &'static str> {
        if self.pending.is_some()
            || self.checkpoint.is_some()
            || !matches!(self.phase, Phase::Paused)
        {
            return Err("Executor is not paused");
        }
        if self.batch.items().iter().any(|item| {
            matches!(
                item.state(),
                ActionState::Unknown
                    | ActionState::Sending
                    | ActionState::RetryReady
                    | ActionState::IdentityConflict
            )
        }) {
            return Err("Look up the original operation receipt first");
        }
        self.phase = Phase::Ready;
        self.notice = None;
        Ok(())
    }
    fn matches_session(&self, client: &HttpClient, scope: &ActionScope) -> bool {
        &self.scope == scope && scope.server == client.base_url()
    }
    pub fn lookup(
        &mut self,
        client: &HttpClient,
        scope: &ActionScope,
        token: &str,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        if !self.matches_session(client, scope) || token.is_empty() {
            return Err("Action session scope mismatch".into());
        }
        if self.pending.is_some()
            || self.checkpoint.is_some()
            || !matches!(self.phase, Phase::Paused)
        {
            return Err("Executor is not ready for receipt lookup".into());
        }
        let command = self
            .batch
            .items()
            .iter()
            .find(|item| item.state() == &ActionState::Unknown)
            .ok_or("No unresolved operation")?
            .command()
            .clone();
        let task = client
            .start(
                HttpRequest {
                    path: format!(
                        "/v1/tenants/{}/{}/actions/{}",
                        scope.tenant.to_uuid(),
                        action_resource(&command),
                        command.operation_id.to_uuid()
                    ),
                    query: vec![],
                    json: None,
                    bearer: Some(token.into()),
                },
                ctx,
            )
            .map_err(|error| error.to_string())?;
        self.pending = Some(Pending {
            command,
            kind: RequestKind::Lookup,
            task,
        });
        self.notice = None;
        Ok(())
    }
    /// Poll on the UI thread. Network work runs in the existing bounded HTTP host.
    /// Storage is deliberately a separate asynchronous host port, never a frame-time write.
    pub fn tick(
        &mut self,
        client: &HttpClient,
        scope: &ActionScope,
        token: &str,
        ctx: &egui::Context,
    ) {
        if !self.matches_session(client, scope) || token.is_empty() {
            self.notice = Some("Action session scope mismatch".into());
            return;
        }
        if let Some(result) = self
            .pending
            .as_mut()
            .and_then(|pending| pending.task.poll())
            && let Some(pending) = self.pending.take()
        {
            if matches!(pending.kind, RequestKind::Execute) && result == Err(HttpError::Status(409))
            {
                // A receipt for that ID belongs to a different payload. Looking it
                // up and accepting it would falsely report our intended edit as applied.
                self.batch.identity_conflict(pending.command.operation_id);
                self.save(AfterSave::Pause);
                return;
            }
            if matches!(pending.kind, RequestKind::Lookup) && result == Err(HttpError::Status(404))
            {
                self.batch.retry_after_lookup(pending.command.operation_id);
                if let Some(command) = self.batch.start_next() {
                    self.save(AfterSave::Send(command));
                }
                return;
            }
            let receipt = result.map_err(|error| error.to_string()).and_then(|bytes| {
                if bytes.len() > MAX_BODY {
                    return Err("Action receipt exceeds 8 MiB".into());
                }
                serde_json::from_slice::<ActionReceipt>(&bytes)
                    .map_err(|_| "Invalid action receipt".into())
            });
            match receipt {
                Ok(receipt)
                    if receipt.operation_id == pending.command.operation_id
                        && receipt.source == pending.command.source
                        && receipt.command_fingerprint != pending.command.fingerprint() =>
                {
                    self.batch.identity_conflict(pending.command.operation_id);
                    self.save(AfterSave::Pause);
                }
                Ok(receipt)
                    if receipt.operation_id == pending.command.operation_id
                        && receipt.source == pending.command.source
                        && !(matches!(
                            pending.command.action.as_str(),
                            "products.update" | "skus.update"
                        ) && matches!(&receipt.outcome, ActionOutcome::Applied { row, .. } if *row != pending.command.source))
                        && self.batch.accept(receipt.clone()) =>
                {
                    self.receipt = Some(receipt);
                    self.save(AfterSave::Continue);
                }
                result => {
                    self.batch.unknown(pending.command.operation_id);
                    self.save(AfterSave::Pause);
                    self.notice = Some(format!(
                        "Result unknown; look up original receipt: {}",
                        result
                            .err()
                            .unwrap_or_else(|| "Receipt identity or outcome mismatch".into())
                    ));
                }
            }
            return;
        }
        if self.pending.is_some() || self.checkpoint.is_some() || self.receipt.is_some() {
            return;
        }
        match std::mem::replace(&mut self.phase, Phase::Paused) {
            Phase::Ready => {
                if let Some(command) = self.batch.start_next() {
                    self.save(AfterSave::Send(command));
                } else {
                    self.phase = if self.batch.items().iter().all(|item| {
                        matches!(
                            item.state(),
                            ActionState::Complete(_) | ActionState::Cancelled
                        )
                    }) {
                        Phase::Finished
                    } else {
                        Phase::Paused
                    };
                }
            }
            Phase::Send(command) => {
                let result = serde_json::to_vec(&command)
                    .map_err(|_| HttpError::TooLarge)
                    .and_then(|body| {
                        client.start(
                            HttpRequest {
                                path: format!(
                                    "/v1/tenants/{}/{}/actions",
                                    scope.tenant.to_uuid(),
                                    action_resource(&command)
                                ),
                                query: vec![],
                                json: Some(body),
                                bearer: Some(token.into()),
                            },
                            ctx,
                        )
                    });
                match result {
                    Ok(task) => {
                        self.pending = Some(Pending {
                            command,
                            kind: RequestKind::Execute,
                            task,
                        })
                    }
                    Err(error) => {
                        self.batch.unknown(command.operation_id);
                        self.save(AfterSave::Pause);
                        self.notice = Some(error.to_string());
                    }
                }
            }
            other => self.phase = other,
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) mod tests;

fn action_resource(command: &ActionCommand) -> &'static str {
    if command.action == "skus.update" {
        "skus"
    } else {
        "products"
    }
}
