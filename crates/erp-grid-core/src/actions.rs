//! Owned action targets and retry identities. Business policy and I/O stay in adapters.
use crate::{ColumnId, RowId, RowSelection};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct ActionSpec {
    pub id: String,
    pub label: String,
    pub min_targets: usize,
    pub max_targets: usize,
    pub disabled_reason: Option<String>,
}
impl ActionSpec {
    pub fn new(id: impl Into<String>, label: impl Into<String>, max_targets: usize) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            min_targets: 1,
            max_targets,
            disabled_reason: None,
        }
    }
    pub fn unavailable(&self, count: usize) -> Option<String> {
        self.disabled_reason.clone().or_else(|| {
            if count < self.min_targets || count > self.max_targets {
                Some(format!(
                    "Choose {}–{} rows",
                    self.min_targets, self.max_targets
                ))
            } else {
                None
            }
        })
    }
    pub fn freeze(
        &self,
        selection: &RowSelection,
        mut command: impl FnMut(RowId) -> ActionCommand,
    ) -> Result<ActionBatch, String> {
        if let Some(reason) = self.unavailable(selection.target_count()) {
            return Err(reason);
        }
        let mut commands = Vec::new();
        let mut bytes = 0;
        for source in selection.targets() {
            let value = command(source);
            value.validate().map_err(str::to_owned)?;
            if value.action != self.id || value.source != source {
                return Err("Action identity mismatch".into());
            }
            bytes += value.estimated_bytes();
            if bytes > 8 * 1024 * 1024 {
                return Err("Action batch exceeds 8 MiB".into());
            }
            commands.push(value);
        }
        ActionBatch::new(commands)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCommand {
    pub operation_id: RowId,
    pub action: String,
    pub source: RowId,
    pub expected_version: Option<String>,
    #[serde(default)]
    pub values: BTreeMap<ColumnId, String>,
}
impl ActionCommand {
    /// Versioned typed encoding, independent of JSON field order and whitespace.
    /// Lengths and numeric IDs are unsigned big-endian; strings are UTF-8.
    pub fn fingerprint(&self) -> String {
        fn text(hash: &mut Sha256, value: &str) {
            hash.update((value.len() as u64).to_be_bytes());
            hash.update(value.as_bytes());
        }
        let mut hash = Sha256::new();
        hash.update(b"erp-grid-action-v1\0");
        hash.update(self.operation_id.0.to_be_bytes());
        hash.update(self.source.0.to_be_bytes());
        text(&mut hash, &self.action);
        hash.update([u8::from(self.expected_version.is_some())]);
        if let Some(version) = &self.expected_version {
            text(&mut hash, version);
        }
        hash.update((self.values.len() as u64).to_be_bytes());
        for (column, value) in &self.values {
            hash.update(column.0.to_be_bytes());
            text(&mut hash, value);
        }
        format!("sha256-v1:{:x}", hash.finalize())
    }
    fn estimated_bytes(&self) -> usize {
        256 + self.action.len()
            + self.expected_version.as_ref().map_or(0, String::len)
            + self.values.len() * 64
            + self.values.values().map(String::len).sum::<usize>()
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.operation_id.0 == 0
            || self.action.is_empty()
            || self.action.len() > 64
            || !self
                .action
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err("Invalid action identity");
        }
        if self
            .expected_version
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.len() > 128)
            || self.values.len() > 64
            || self
                .values
                .values()
                .any(|v| v.len() > 4096 || v.contains('\0'))
            || self.values.values().map(String::len).sum::<usize>() > 65_536
        {
            return Err("Action payload exceeds limits");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActionReceipt {
    pub operation_id: RowId,
    pub source: RowId,
    pub command_fingerprint: String,
    pub outcome: ActionOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionOutcome {
    Applied {
        row: RowId,
        version: String,
        values: BTreeMap<ColumnId, String>,
    },
    Rejected {
        code: String,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionState {
    Ready,
    /// A prior attempt may still commit. This must never be cancelled as unsent.
    RetryReady,
    Sending,
    Unknown,
    /// The server already owns this identity for a different command. Never replay.
    IdentityConflict,
    Complete(ActionOutcome),
    Cancelled,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionItem {
    command: ActionCommand,
    state: ActionState,
}
impl ActionItem {
    pub fn command(&self) -> &ActionCommand {
        &self.command
    }
    pub fn state(&self) -> &ActionState {
        &self.state
    }
}
#[derive(Clone, Debug)]
pub struct ActionBatch {
    items: Vec<ActionItem>,
}

/// Non-secret recovery data. Hosts must durably save before dispatching a write.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCheckpoint {
    version: u16,
    #[serde(deserialize_with = "checkpoint_items")]
    items: Vec<ActionItem>,
}

fn checkpoint_items<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ActionItem>, D::Error> {
    struct Items;
    impl<'de> serde::de::Visitor<'de> for Items {
        type Value = Vec<ActionItem>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("at most 10,000 action items")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let mut items = Vec::new();
            while let Some(item) = sequence.next_element()? {
                if items.len() == RowSelection::MAX_CHECKED {
                    return Err(serde::de::Error::custom("Too many action checkpoint items"));
                }
                items.push(item);
            }
            Ok(items)
        }
    }
    deserializer.deserialize_seq(Items)
}
impl ActionBatch {
    pub fn checkpoint(&self) -> ActionCheckpoint {
        ActionCheckpoint {
            version: 1,
            items: self.items.clone(),
        }
    }
    pub fn restore(checkpoint: ActionCheckpoint) -> Result<Self, String> {
        if checkpoint.version != 1 {
            return Err("Unsupported action checkpoint version".into());
        }
        let mut batch = Self::new(
            checkpoint
                .items
                .iter()
                .map(|item| item.command.clone())
                .collect(),
        )?;
        let mut unresolved = 0;
        for (item, saved) in batch.items.iter_mut().zip(checkpoint.items) {
            item.state = match saved.state {
                ActionState::Sending | ActionState::RetryReady | ActionState::Unknown => {
                    unresolved += 1;
                    ActionState::Unknown
                }
                ActionState::IdentityConflict => {
                    unresolved += 1;
                    ActionState::IdentityConflict
                }
                ActionState::Complete(outcome) => {
                    let outcome = summary(outcome).ok_or("Invalid saved action receipt")?;
                    ActionState::Complete(outcome)
                }
                other => other,
            };
        }
        if unresolved > 1 {
            return Err("Multiple unresolved actions in serial batch".into());
        }
        Ok(batch)
    }
    pub fn new(commands: Vec<ActionCommand>) -> Result<Self, String> {
        if commands.is_empty() || commands.len() > RowSelection::MAX_CHECKED {
            return Err("Choose 1–10,000 action targets".into());
        }
        let mut identities = BTreeSet::new();
        let mut sources = BTreeSet::new();
        let mut bytes = 0usize;
        for command in &commands {
            command.validate().map_err(str::to_owned)?;
            if !identities.insert(command.operation_id) || !sources.insert(command.source) {
                return Err("Duplicate operation or source".into());
            }
            bytes += command.estimated_bytes();
        }
        if bytes > 8 * 1024 * 1024 {
            return Err("Action batch exceeds 8 MiB".into());
        }
        Ok(Self {
            items: commands
                .into_iter()
                .map(|command| ActionItem {
                    command,
                    state: ActionState::Ready,
                })
                .collect(),
        })
    }
    pub fn items(&self) -> &[ActionItem] {
        &self.items
    }
    pub fn start_next(&mut self) -> Option<ActionCommand> {
        if self.items.iter().any(|item| {
            matches!(
                item.state,
                ActionState::Sending | ActionState::Unknown | ActionState::IdentityConflict
            )
        }) {
            return None;
        }
        let index = self
            .items
            .iter()
            .position(|item| item.state == ActionState::RetryReady)
            .or_else(|| {
                self.items
                    .iter()
                    .position(|item| item.state == ActionState::Ready)
            })?;
        let item = &mut self.items[index];
        item.state = ActionState::Sending;
        Some(item.command.clone())
    }
    pub fn unknown(&mut self, operation: RowId) {
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.command.operation_id == operation)
            && item.state == ActionState::Sending
        {
            item.state = ActionState::Unknown;
        }
    }
    pub fn identity_conflict(&mut self, operation: RowId) {
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.command.operation_id == operation)
            && matches!(item.state, ActionState::Sending | ActionState::Unknown)
        {
            item.state = ActionState::IdentityConflict;
        }
    }
    /// A missing receipt allows an idempotent resend only, never a fresh operation ID.
    pub fn retry_after_lookup(&mut self, operation: RowId) -> bool {
        let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.command.operation_id == operation)
        else {
            return false;
        };
        if item.state != ActionState::Unknown {
            return false;
        }
        item.state = ActionState::RetryReady;
        true
    }
    pub fn accept(&mut self, receipt: ActionReceipt) -> bool {
        let Some(item) = self.items.iter_mut().find(|item| {
            item.command.operation_id == receipt.operation_id
                && item.command.source == receipt.source
        }) else {
            return false;
        };
        if !matches!(item.state, ActionState::Sending | ActionState::Unknown) {
            return false;
        }
        if receipt.command_fingerprint != item.command.fingerprint() {
            return false;
        }
        // Keep bounded summaries for up to 10,000 targets. The host consumes
        // returned cell values before accepting; the batch is not a row cache.
        let Some(outcome) = summary(receipt.outcome) else {
            return false;
        };
        item.state = ActionState::Complete(outcome);
        true
    }
    pub fn cancel_pending(&mut self) {
        for item in &mut self.items {
            if item.state == ActionState::Ready {
                item.state = ActionState::Cancelled;
            }
        }
    }
}

fn summary(outcome: ActionOutcome) -> Option<ActionOutcome> {
    Some(match outcome {
        ActionOutcome::Applied { row, version, .. }
            if !version.is_empty() && version.len() <= 128 =>
        {
            ActionOutcome::Applied {
                row,
                version,
                values: BTreeMap::new(),
            }
        }
        ActionOutcome::Applied { .. } => return None,
        ActionOutcome::Rejected { code, message } if !code.is_empty() && code.len() <= 64 => {
            ActionOutcome::Rejected {
                code,
                message: message.chars().take(128).collect(),
            }
        }
        ActionOutcome::Rejected { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(source: RowId) -> ActionCommand {
        ActionCommand {
            operation_id: RowId(source.0 + 100),
            action: "products.copy".into(),
            source,
            expected_version: None,
            values: BTreeMap::new(),
        }
    }
    #[test]
    fn actions_freeze_checked_ids_and_unknown_requires_same_identity_lookup() {
        let mut selection = RowSelection::default();
        selection.active = Some(RowId(9));
        selection.toggle(RowId(1));
        selection.toggle(RowId(2));
        let spec = ActionSpec::new("products.copy", "Copy product", 10000);
        let mut batch = spec.freeze(&selection, command).unwrap();
        selection.clear();
        let first = batch.start_next().unwrap();
        assert_eq!(first.source, RowId(1));
        assert!(batch.start_next().is_none());
        batch.unknown(first.operation_id);
        assert!(batch.start_next().is_none());
        assert!(batch.retry_after_lookup(first.operation_id));
        assert_eq!(batch.start_next().unwrap(), first);
        assert!(!batch.accept(ActionReceipt {
            operation_id: first.operation_id,
            source: RowId(9),
            command_fingerprint: first.fingerprint(),
            outcome: ActionOutcome::Rejected {
                code: "wrong".into(),
                message: "wrong".into()
            }
        }));
        assert!(batch.accept(ActionReceipt {
            operation_id: first.operation_id,
            source: first.source,
            command_fingerprint: first.fingerprint(),
            outcome: ActionOutcome::Applied {
                row: RowId(500),
                version: "v1".into(),
                values: BTreeMap::new()
            }
        }));
        batch.cancel_pending();
        assert!(batch.start_next().is_none());
        assert_eq!(batch.items()[1].state(), &ActionState::Cancelled);
    }
    #[test]
    fn actions_charge_empty_field_maps_and_stop_freezing_at_budget() {
        let mut selection = RowSelection::default();
        for id in 1..=10_000 {
            selection.toggle(RowId(id));
        }
        let values: BTreeMap<_, _> = (0..64).map(|id| (ColumnId(id), String::new())).collect();
        let mut allocated = 0;
        let result = ActionSpec::new("products.copy", "Copy", 10000).freeze(&selection, |source| {
            allocated += 1;
            ActionCommand {
                values: values.clone(),
                ..command(source)
            }
        });
        assert!(result.is_err());
        assert!(
            allocated < 10_000,
            "Reject before allocating the entire oversized batch"
        );
    }
    #[test]
    fn actions_enforce_limits_capabilities_and_unique_identities() {
        let mut selection = RowSelection::default();
        selection.toggle(RowId(1));
        selection.toggle(RowId(2));
        assert!(
            ActionSpec::new("products.copy", "Copy", 1)
                .freeze(&selection, command)
                .is_err()
        );
        assert!(ActionBatch::new(vec![command(RowId(1)), command(RowId(1))]).is_err());
        let mut spec = ActionSpec::new("products.copy", "Copy", 10);
        spec.disabled_reason = Some("No permission".into());
        assert_eq!(
            spec.freeze(&selection, command).unwrap_err(),
            "No permission"
        );
    }
    #[test]
    fn recovery_preserves_retry_identity_and_never_cancels_a_possible_write() {
        let mut batch = ActionBatch::new(vec![command(RowId(1)), command(RowId(2))]).unwrap();
        let first = batch.start_next().unwrap();
        let mut recovered = ActionBatch::restore(batch.checkpoint()).unwrap();
        assert_eq!(recovered.items()[0].state(), &ActionState::Unknown);
        assert!(recovered.start_next().is_none());
        assert!(recovered.retry_after_lookup(first.operation_id));
        recovered.cancel_pending();
        assert_eq!(recovered.items()[0].state(), &ActionState::RetryReady);
        assert_eq!(recovered.items()[1].state(), &ActionState::Cancelled);
        let mut restarted = ActionBatch::restore(recovered.checkpoint()).unwrap();
        assert_eq!(restarted.items()[0].state(), &ActionState::Unknown);
        assert!(restarted.retry_after_lookup(first.operation_id));
        assert_eq!(restarted.start_next(), Some(first));
    }
    #[test]
    fn recovery_rejects_corrupt_versions_duplicate_ids_and_unresolved_concurrency() {
        let batch = ActionBatch::new(vec![command(RowId(1)), command(RowId(2))]).unwrap();
        let mut checkpoint = batch.checkpoint();
        checkpoint.version = 2;
        assert!(ActionBatch::restore(checkpoint).is_err());
        let mut checkpoint = batch.checkpoint();
        checkpoint.items[1].command.operation_id = checkpoint.items[0].command.operation_id;
        assert!(ActionBatch::restore(checkpoint).is_err());
        let mut checkpoint = batch.checkpoint();
        for item in &mut checkpoint.items {
            item.state = ActionState::Sending;
        }
        assert!(ActionBatch::restore(checkpoint).is_err());
        let mut checkpoint = batch.checkpoint();
        checkpoint.items[0].state = ActionState::Complete(ActionOutcome::Applied {
            row: RowId(3),
            version: String::new(),
            values: BTreeMap::new(),
        });
        assert!(ActionBatch::restore(checkpoint).is_err());
    }
    #[test]
    fn action_factory_cannot_substitute_an_unselected_source() {
        let mut selection = RowSelection::default();
        selection.active = Some(RowId(1));
        assert!(
            ActionSpec::new("products.copy", "Copy", 1)
                .freeze(&selection, |_| command(RowId(99)))
                .is_err()
        );
    }

    #[test]
    fn fingerprints_bind_exact_typed_command_and_receipts_cannot_cross_payloads() {
        let original = command(RowId(1));
        // Independently calculated using Python hashlib and big-endian struct.pack.
        assert_eq!(
            original.fingerprint(),
            "sha256-v1:d1fb3af7ea3a956b068f6b2394a596091c34a5bac68588c38c58f1b721c2a923"
        );
        let mut changed = original.clone();
        changed.expected_version = Some("version-2".into());
        assert_ne!(original.fingerprint(), changed.fingerprint());
        let mut batch = ActionBatch::new(vec![changed]).unwrap();
        batch.start_next().unwrap();
        assert!(!batch.accept(ActionReceipt {
            operation_id: original.operation_id,
            source: original.source,
            command_fingerprint: original.fingerprint(),
            outcome: ActionOutcome::Rejected {
                code: "VERSION_CONFLICT".into(),
                message: "Old command".into()
            },
        }));
        assert_eq!(batch.items()[0].state(), &ActionState::Sending);
        let mut fields = original.clone();
        fields.values.insert(ColumnId(1), "ab".into());
        let mut other_fields = original;
        other_fields.values.insert(ColumnId(2), "ab".into());
        assert_ne!(fields.fingerprint(), other_fields.fingerprint());
    }
}
