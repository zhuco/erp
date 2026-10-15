//! Transport-free record-form save identity and recovery state.
//! Hosts verify authoritative receipts and durably checkpoint before external writes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

const MAX_FIELDS: usize = 256;
const MAX_VALUE: usize = 4096;
const MAX_DRAFT_VALUE: usize = 16 * 1024;
const MAX_CHECKPOINT: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailField {
    pub id: String,
    pub label: String,
    pub value: String,
    pub editable: bool,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetailSaveRequest {
    pub operation_id: Uuid,
    pub scope: String,
    pub record_id: String,
    pub observed_version: String,
    pub before: BTreeMap<String, String>,
    pub changes: BTreeMap<String, String>,
}

impl DetailSaveRequest {
    fn validate(&self) -> Result<(), String> {
        if self.operation_id.is_nil()
            || !bounded_id(&self.scope, 1024)
            || !bounded_id(&self.record_id, 512)
            || !bounded_id(&self.observed_version, 512)
            || self.changes.is_empty()
            || self.changes.len() > MAX_FIELDS
            || self.before.keys().ne(self.changes.keys())
        {
            return Err("保存身份或字段范围无效".into());
        }
        for (id, value) in &self.changes {
            let before = self.before.get(id).ok_or("缺少原字段值")?;
            if !bounded_id(id, 128)
                || value.len() > MAX_VALUE
                || before.len() > MAX_VALUE
                || value == before
            {
                return Err("保存字段或原意图无效".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailSaveOutcome {
    Committed {
        version: String,
        values: BTreeMap<String, String>,
    },
    Rejected {
        code: String,
        message: String,
    },
    Unknown(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailSavePhase {
    Checkpointing,
    Awaiting,
    Unknown,
    Reading,
}

#[derive(Clone)]
struct Pending {
    request: DetailSaveRequest,
    phase: DetailSavePhase,
}

#[derive(Clone)]
pub(super) struct FieldMeta {
    pub label: String,
    pub editable: bool,
    pub required: bool,
    pub available: bool,
}

#[derive(Clone)]
struct Latest {
    version: String,
    fields: Vec<DetailField>,
}

#[derive(Clone, Default)]
pub(super) struct FormState {
    pub scope: String,
    pub record_id: Option<String>,
    pub version: String,
    pub base: BTreeMap<String, String>,
    pub draft: BTreeMap<String, String>,
    pub fields: BTreeMap<String, FieldMeta>,
    pub order: Vec<String>,
    pub error: Option<String>,
    pub composing: bool,
    pub storage_ready: bool,
    pending: Option<Pending>,
    resolved: Option<DetailSaveRequest>,
    latest: Option<Latest>,
    conflict: bool,
    binding_valid: bool,
    needs_rebind: bool,
}

impl FormState {
    pub fn new(scope: String) -> Self {
        Self {
            scope,
            storage_ready: true,
            ..Default::default()
        }
    }
    pub fn unresolved(&self) -> bool {
        self.pending.is_some()
    }
    pub fn phase(&self) -> Option<DetailSavePhase> {
        self.pending.as_ref().map(|p| p.phase)
    }
    pub fn pending_request(&self) -> Option<&DetailSaveRequest> {
        self.pending.as_ref().map(|p| &p.request)
    }
    pub fn dirty(&self) -> bool {
        self.unresolved() || !self.changes().is_empty()
    }
    pub fn changes(&self) -> BTreeMap<String, String> {
        self.draft
            .iter()
            .filter(|(id, value)| self.base.get(*id) != Some(*value))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect()
    }
    pub fn adopt_backup(&mut self, source: &Self) -> Result<(), String> {
        if self.scope != source.scope
            || self.record_id.is_none()
            || self.record_id != source.record_id
        {
            return Err("先选择副本对应的当前记录，作用域和记录须一致".into());
        }
        if self.dirty() {
            return Err("当前草稿或保存未结束，不能采用副本".into());
        }
        if source.unresolved() {
            return Err("副本原结果尚未确认，不能采用".into());
        }
        if let Some(error) = self.validation() {
            return Err(error);
        }
        let changes = source.changes();
        if changes.is_empty() {
            return Err("副本没有可采用的修改".into());
        }
        let mut candidate = self.clone();
        for (id, value) in changes {
            if !self
                .fields
                .get(&id)
                .is_some_and(|meta| meta.available && meta.editable)
            {
                return Err("副本包含当前不可写或已移除的字段，未采用任何值".into());
            }
            candidate.draft.insert(id, value);
        }
        if let Some(error) = candidate.validation() {
            return Err(error);
        }
        if candidate.changes().is_empty() {
            return Err("副本值与当前记录相同".into());
        }
        self.draft = candidate.draft;
        self.resolved = None;
        self.error = None;
        Ok(())
    }
    pub fn reset(&mut self) -> Result<(), String> {
        if self.unresolved() {
            return Err("原保存结果尚未确认，不能丢弃".into());
        }
        *self = Self::new(self.scope.clone());
        Ok(())
    }
    pub fn bind(
        &mut self,
        record: &str,
        version: &str,
        fields: &[DetailField],
    ) -> Result<(), String> {
        if self.record_id.as_deref() != Some(record) && self.dirty() {
            return Err("请先保存或取消原记录修改".into());
        }
        if !bounded_id(record, 512) || !bounded_id(version, 512) {
            self.binding_valid = false;
            return Err("记录或版本无效".into());
        }
        if let Err(error) = validate_fields(fields) {
            self.binding_valid = false;
            return Err(error);
        }
        let changes = self.changes();
        let retained = changes
            .keys()
            .filter(|id| !fields.iter().any(|f| &f.id == *id))
            .count();
        if fields.len() + retained > MAX_FIELDS {
            self.binding_valid = false;
            return Err("字段范围超过256项，原草稿保留".into());
        }
        if !self.binding_valid && !self.unresolved() {
            self.error = None;
        }
        self.binding_valid = true;
        self.needs_rebind = false;
        if self.record_id.as_deref() != Some(record) {
            self.record_id = Some(record.to_owned());
            self.version = version.to_owned();
            self.base.clear();
            self.draft.clear();
            self.fields.clear();
            self.order.clear();
        }
        if version != self.version && (self.dirty() || self.unresolved()) {
            self.latest = Some(Latest {
                version: version.into(),
                fields: fields.to_vec(),
            });
            self.conflict = true;
            if !self.unresolved() {
                self.error = Some("记录版本已变化，请采用最新版本后保存".into());
            }
        }
        // Refresh authority metadata even when old values/intent must remain frozen.
        let keep: Vec<_> = changes
            .keys()
            .filter(|id| !fields.iter().any(|f| &f.id == *id))
            .cloned()
            .collect();
        self.fields
            .retain(|id, _| fields.iter().any(|f| &f.id == id) || keep.contains(id));
        self.order = fields.iter().map(|f| f.id.clone()).collect();
        self.order.extend(keep.iter().cloned());
        for id in keep {
            if let Some(meta) = self.fields.get_mut(&id) {
                meta.available = false;
                meta.editable = false;
            }
        }
        for field in fields {
            self.fields.insert(
                field.id.clone(),
                FieldMeta {
                    label: field.label.clone(),
                    editable: field.editable,
                    required: field.required,
                    available: true,
                },
            );
            if !self.unresolved() && !self.conflict {
                let changed = changes.contains_key(&field.id);
                if !changed {
                    self.base.insert(field.id.clone(), field.value.clone());
                    self.draft.insert(field.id.clone(), field.value.clone());
                }
            } else if !self.base.contains_key(&field.id) {
                self.base.insert(field.id.clone(), field.value.clone());
                self.draft.insert(field.id.clone(), field.value.clone());
            }
        }
        self.base.retain(|id, _| self.fields.contains_key(id));
        self.draft.retain(|id, _| self.fields.contains_key(id));
        if !self.dirty() {
            self.version = version.into();
            self.conflict = false;
            self.latest = None;
        }
        Ok(())
    }
    pub fn validation(&self) -> Option<String> {
        if !self.storage_ready {
            return Some("恢复存储尚未加载或确认".into());
        }
        if !self.binding_valid || self.needs_rebind {
            return Some("先重新读取原记录与字段权限".into());
        }
        if self.composing {
            return Some("请先完成输入法组合".into());
        }
        if self.unresolved() {
            return Some("原保存结果尚未确认".into());
        }
        if self.conflict {
            return Some("先采用最新版本，再用新身份保存".into());
        }
        if !bounded_id(&self.scope, 1024) {
            return Some("保存作用域无效".into());
        }
        for (id, meta) in &self.fields {
            let value = self.draft.get(id).map(String::as_str).unwrap_or("");
            if meta.required && meta.available && value.trim().is_empty() {
                return Some(format!("{}不能为空", meta.label));
            }
            if value.len() > MAX_VALUE {
                return Some(format!("{}超过4096字节", meta.label));
            }
        }
        for id in self.changes().keys() {
            if !self
                .fields
                .get(id)
                .is_some_and(|f| f.available && f.editable)
            {
                return Some("已修改字段不可写或已移除，原输入保留".into());
            }
        }
        None
    }
    pub fn begin_save(&mut self) -> Result<DetailSaveRequest, String> {
        if let Some(error) = self.validation() {
            return Err(error);
        }
        let changes = self.changes();
        let request = DetailSaveRequest {
            operation_id: Uuid::new_v4(),
            scope: self.scope.clone(),
            record_id: self.record_id.clone().ok_or("未绑定原记录")?,
            observed_version: self.version.clone(),
            before: changes
                .keys()
                .filter_map(|id| self.base.get(id).map(|v| (id.clone(), v.clone())))
                .collect(),
            changes,
        };
        request.validate()?;
        self.pending = Some(Pending {
            request: request.clone(),
            phase: DetailSavePhase::Checkpointing,
        });
        self.error = None;
        Ok(request)
    }
    pub fn persistence_write(&self) -> Result<super::detail_storage::DetailPersistWrite, String> {
        Ok(super::detail_storage::DetailPersistWrite {
            scope: self.scope.clone(),
            bytes: self.checkpoint()?,
            resolved: self.resolved.clone(),
        })
    }
    pub fn authorize_send(
        &mut self,
        request: &DetailSaveRequest,
        receipt: &super::detail_storage::DetailPersistReceipt,
    ) -> Result<(), String> {
        let bytes = self.checkpoint()?;
        if !receipt.matches(&self.scope, bytes.as_deref()) {
            return Err("持久检查点与原保存不匹配，禁止发送".into());
        }
        let pending = self.pending.as_mut().ok_or("没有原待发送保存")?;
        if pending.request != *request || pending.phase != DetailSavePhase::Checkpointing {
            return Err("原保存身份或发送阶段已变化".into());
        }
        pending.phase = DetailSavePhase::Awaiting;
        Ok(())
    }
    pub fn begin_read(&mut self) -> Result<DetailSaveRequest, String> {
        let pending = self.pending.as_mut().ok_or("没有原未决保存")?;
        if pending.phase != DetailSavePhase::Unknown {
            return Err("原结果回查尚未就绪".into());
        }
        pending.phase = DetailSavePhase::Reading;
        Ok(pending.request.clone())
    }
    pub fn resolve(
        &mut self,
        request: &DetailSaveRequest,
        outcome: DetailSaveOutcome,
    ) -> Result<(), String> {
        let pending = self.pending.as_ref().ok_or("没有对应的未决保存")?;
        if pending.request.operation_id != request.operation_id {
            return Err("忽略迟到或其他操作回执".into());
        }
        if pending.request != *request {
            self.mark_unknown("原保存身份或载荷不匹配");
            return Err("原保存身份或载荷不匹配".into());
        }
        match outcome {
            DetailSaveOutcome::Unknown(message) => self.mark_unknown(&message),
            DetailSaveOutcome::Rejected { code, message } => {
                self.resolved = Some(request.clone());
                self.pending = None;
                self.error = Some(bounded_message(&message));
                if code == "VERSION_CONFLICT" {
                    self.conflict = true;
                }
            }
            DetailSaveOutcome::Committed { version, values } => {
                if !bounded_id(&version, 512)
                    || values.len() > MAX_FIELDS
                    || values
                        .iter()
                        .any(|(id, v)| !bounded_id(id, 128) || v.len() > MAX_VALUE)
                    || request.changes.keys().any(|id| !values.contains_key(id))
                    || values.iter().any(|(id, value)| {
                        self.fields
                            .get(id)
                            .is_some_and(|meta| meta.required && value.trim().is_empty())
                    })
                    || (version == request.observed_version
                        && request
                            .changes
                            .keys()
                            .any(|id| values.get(id) != request.before.get(id)))
                {
                    self.mark_unknown("原保存结果不完整或版本无效");
                    return Err("原保存结果不完整或版本无效".into());
                }
                for (id, value) in values {
                    if self.fields.contains_key(&id) {
                        self.base.insert(id.clone(), value.clone());
                        self.draft.insert(id, value);
                    }
                }
                self.version = version;
                self.resolved = Some(request.clone());
                self.pending = None;
                self.conflict = false;
                self.latest = None;
                self.error = None;
            }
        }
        Ok(())
    }
    fn mark_unknown(&mut self, message: &str) {
        if let Some(pending) = self.pending.as_mut() {
            pending.phase = DetailSavePhase::Unknown;
        }
        self.error = Some(bounded_message(message));
    }
    pub fn require_latest(&mut self) {
        self.conflict = true;
        self.error = Some("原保存已核实，当前草稿保留；先读取并采用最新版本".into());
    }
    pub fn can_adopt(&self) -> bool {
        !self.unresolved()
            && self
                .latest
                .as_ref()
                .is_some_and(|latest| latest.version != self.version)
    }
    pub fn adopt_latest(&mut self) -> Result<(), String> {
        if !self.can_adopt() {
            return Err("没有可采用的最新版本，或原结果仍未决".into());
        }
        let latest = self.latest.take().ok_or("缺少最新版本")?;
        let changes = self.changes();
        let before = self.base.clone();
        self.base.clear();
        self.draft.clear();
        self.fields.clear();
        self.order.clear();
        self.version = latest.version.clone();
        self.conflict = false;
        self.error = None;
        self.bind(
            &self.record_id.clone().ok_or("未绑定原记录")?,
            &latest.version,
            &latest.fields,
        )?;
        for (id, value) in changes {
            if self.fields.contains_key(&id) {
                self.draft.insert(id, value);
            } else {
                self.fields.insert(
                    id.clone(),
                    FieldMeta {
                        label: id.clone(),
                        editable: false,
                        required: false,
                        available: false,
                    },
                );
                self.base
                    .insert(id.clone(), before.get(&id).cloned().ok_or("缺少原字段值")?);
                self.draft.insert(id.clone(), value);
                self.order.push(id);
            }
        }
        Ok(())
    }
    pub fn checkpoint(&self) -> Result<Option<Vec<u8>>, String> {
        if !self.dirty() {
            return Ok(None);
        }
        let changes = self.changes();
        if changes.len() > MAX_FIELDS
            || changes.iter().any(|(id, value)| {
                !bounded_id(id, 128)
                    || value.len() > MAX_DRAFT_VALUE
                    || self
                        .base
                        .get(id)
                        .is_none_or(|before| before.len() > MAX_VALUE)
            })
        {
            return Err("恢复草稿超出有界容量".into());
        }
        let before = changes
            .keys()
            .filter_map(|id| self.base.get(id).map(|v| (id.clone(), v.clone())))
            .collect();
        let checkpoint = Checkpoint {
            schema: 1,
            scope: self.scope.clone(),
            record: self.record_id.clone().ok_or("未绑定记录")?,
            version: self.version.clone(),
            before,
            changes,
            pending: self.pending_request().cloned(),
            conflict: self.conflict,
        };
        let bytes = serde_json::to_vec(&checkpoint).map_err(|_| "无法编码恢复检查点")?;
        if bytes.len() > MAX_CHECKPOINT {
            return Err("恢复检查点超过8MiB".into());
        }
        Ok(Some(bytes))
    }
    pub fn restore(scope: &str, bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_CHECKPOINT {
            return Err("恢复检查点超过8MiB".into());
        }
        let saved: Checkpoint = serde_json::from_slice(bytes).map_err(|_| "恢复检查点格式无效")?;
        if saved.schema != 1
            || saved.scope != scope
            || !bounded_id(scope, 1024)
            || !bounded_id(&saved.record, 512)
            || !bounded_id(&saved.version, 512)
            || saved.changes.len() > MAX_FIELDS
            || saved.before.keys().ne(saved.changes.keys())
        {
            return Err("恢复身份、版本或字段范围无效".into());
        }
        for (id, value) in &saved.changes {
            if !bounded_id(id, 128)
                || value.len() > MAX_DRAFT_VALUE
                || saved
                    .before
                    .get(id)
                    .is_none_or(|v| v.len() > MAX_VALUE || v == value)
            {
                return Err("恢复字段或原输入无效".into());
            }
        }
        if let Some(request) = &saved.pending {
            request.validate()?;
            if request.scope != scope
                || request.record_id != saved.record
                || request.observed_version != saved.version
                || request.before != saved.before
                || request.changes != saved.changes
            {
                return Err("恢复原保存与草稿不一致".into());
            }
        }
        let fields = saved
            .changes
            .keys()
            .map(|id| {
                (
                    id.clone(),
                    FieldMeta {
                        label: id.clone(),
                        editable: false,
                        required: false,
                        available: false,
                    },
                )
            })
            .collect();
        let order = saved.changes.keys().cloned().collect();
        Ok(Self {
            scope: scope.into(),
            record_id: Some(saved.record),
            version: saved.version,
            base: saved.before,
            draft: saved.changes,
            fields,
            order,
            pending: saved.pending.map(|request| Pending {
                request,
                phase: DetailSavePhase::Unknown,
            }),
            conflict: saved.conflict,
            needs_rebind: true,
            storage_ready: true,
            ..Default::default()
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    schema: u16,
    scope: String,
    record: String,
    version: String,
    before: BTreeMap<String, String>,
    changes: BTreeMap<String, String>,
    pending: Option<DetailSaveRequest>,
    conflict: bool,
}

fn bounded_id(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max
}
fn bounded_message(value: &str) -> String {
    value.chars().take(1024).collect()
}
fn validate_fields(fields: &[DetailField]) -> Result<(), String> {
    if fields.len() > MAX_FIELDS {
        return Err("字段超过256项".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for field in fields {
        if !bounded_id(&field.id, 128)
            || field.label.len() > 512
            || field.value.len() > MAX_VALUE
            || !ids.insert(&field.id)
        {
            return Err("字段描述、值或重复身份无效".into());
        }
    }
    Ok(())
}
