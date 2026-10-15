//! Compact record editor. It emits immutable intents; hosts own transport and receipt proof.

use super::detail_state::FormState;
pub use super::detail_state::{DetailField, DetailSaveOutcome, DetailSavePhase, DetailSaveRequest};
use eframe::egui;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailEvent {
    Save(DetailSaveRequest),
    Lookup(DetailSaveRequest),
    Discard,
    Blocked { current: String, requested: String },
}

#[derive(Default)]
pub struct DetailPanel {
    state: FormState,
}

impl DetailPanel {
    pub fn new(scope: String) -> Self {
        Self {
            state: FormState::new(scope),
        }
    }
    pub fn field_label(&self, id: &str) -> String {
        self.state
            .fields
            .get(id)
            .map(|field| field.label.clone())
            .unwrap_or_else(|| id.to_owned())
    }
    pub fn field_value(&self, id: &str) -> Option<&str> {
        self.state
            .fields
            .get(id)
            .filter(|meta| meta.available)
            .and_then(|_| self.state.draft.get(id))
            .map(String::as_str)
    }
    pub fn bound_version(&self) -> &str {
        &self.state.version
    }
    pub fn adopt_backup(&mut self, source: &Self) -> Result<(), String> {
        self.state.adopt_backup(&source.state)
    }
    pub fn require_latest(&mut self) {
        self.state.require_latest();
    }
    pub fn scope(&self) -> &str {
        &self.state.scope
    }
    pub fn storage_ready(&mut self, ready: bool) {
        self.state.storage_ready = ready;
    }
    pub fn is_dirty(&self) -> bool {
        self.state.dirty()
    }
    pub fn has_unresolved_save(&self) -> bool {
        self.state.unresolved()
    }
    pub fn save_phase(&self) -> Option<DetailSavePhase> {
        self.state.phase()
    }
    pub fn begin_read(&mut self) -> Result<DetailSaveRequest, String> {
        self.state.begin_read()
    }
    pub fn adopt_latest(&mut self) -> Result<(), String> {
        self.state.adopt_latest()
    }
    pub fn pending_request(&self) -> Option<&DetailSaveRequest> {
        self.state.pending_request()
    }
    pub fn bound_record(&self) -> Option<&str> {
        self.state.record_id.as_deref()
    }
    pub fn reset(&mut self) -> Result<(), String> {
        self.state.reset()
    }
    pub fn persistence_write(&self) -> Result<super::detail_storage::DetailPersistWrite, String> {
        self.state.persistence_write()
    }
    pub fn authorize_send(
        &mut self,
        request: &DetailSaveRequest,
        receipt: &super::detail_storage::DetailPersistReceipt,
    ) -> Result<(), String> {
        self.state.authorize_send(request, receipt)
    }
    pub fn begin_save(&mut self) -> Result<DetailSaveRequest, String> {
        self.state.begin_save()
    }
    pub fn resolve_save(
        &mut self,
        request: &DetailSaveRequest,
        outcome: DetailSaveOutcome,
    ) -> Result<(), String> {
        self.state.resolve(request, outcome)
    }
    pub fn checkpoint(&self) -> Result<Option<Vec<u8>>, String> {
        self.state.checkpoint()
    }
    pub fn restore(scope: &str, bytes: &[u8]) -> Result<Self, String> {
        Ok(Self {
            state: FormState::restore(scope, bytes)?,
        })
    }
    pub fn edit_value(&mut self, id: &str, value: String) -> Result<(), String> {
        if self.has_unresolved_save()
            || !self
                .state
                .fields
                .get(id)
                .is_some_and(|f| f.available && f.editable)
            || value.len() > 4096
        {
            return Err("字段不可修改或原保存仍未决".into());
        }
        self.state.draft.insert(id.into(), value);
        Ok(())
    }
    pub fn bind(
        &mut self,
        record_id: &str,
        version: &str,
        fields: &[DetailField],
    ) -> Option<DetailEvent> {
        if self.bound_record() != Some(record_id) && self.is_dirty() {
            return Some(DetailEvent::Blocked {
                current: self.bound_record().unwrap_or_default().into(),
                requested: record_id.into(),
            });
        }
        if let Err(error) = self.state.bind(record_id, version, fields) {
            self.state.error = Some(error);
        }
        None
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        record_id: &str,
        version: &str,
        fields: &[DetailField],
    ) -> Vec<DetailEvent> {
        let mut events = Vec::new();
        if let Some(event) = self.bind(record_id, version, fields) {
            events.push(event);
            return events;
        }
        let focused = ui.memory(|memory| memory.focused());
        let owns_ime = self
            .state
            .order
            .iter()
            .any(|id| Some(ui.id().with(("detail-input", record_id, id))) == focused);
        if !owns_ime {
            self.state.composing = false;
        }
        ui.input(|input| {
            if owns_ime {
                for event in &input.events {
                    if let egui::Event::Ime(event) = event {
                        match event {
                            egui::ImeEvent::Preedit { text, .. } => {
                                self.state.composing = !text.is_empty()
                            }
                            egui::ImeEvent::Commit(_) => self.state.composing = false,
                            _ => {}
                        }
                    }
                }
            }
        });
        let locked = self.has_unresolved_save();
        for id in self.state.order.clone() {
            let Some(meta) = self.state.fields.get(&id) else {
                continue;
            };
            let label = if meta.required {
                format!("{} *", meta.label)
            } else {
                meta.label.clone()
            };
            let editable = meta.available && meta.editable;
            let input_id = ui.id().with(("detail-input", record_id, &id));
            let value = self.state.draft.entry(id.clone()).or_default();
            ui.horizontal(|ui| {
                ui.add_sized(
                    [78.0, 26.0],
                    egui::Label::new(
                        egui::RichText::new(label).color(ui.visuals().weak_text_color()),
                    )
                    .truncate(),
                );
                if editable {
                    ui.add_enabled(
                        !locked && self.state.storage_ready,
                        egui::TextEdit::singleline(value)
                            .id(input_id)
                            .desired_width(ui.available_width())
                            .char_limit(4096),
                    );
                } else {
                    ui.add_sized(
                        [ui.available_width(), 26.0],
                        egui::Label::new(value.as_str()).selectable(true).truncate(),
                    );
                }
            });
        }
        // Controls use the same admission checks as the public host methods.
        let invalid = self.state.validation();
        let changed = !self.state.changes().is_empty();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(changed && invalid.is_none(), egui::Button::new("保存"))
                .clicked()
            {
                match self.state.begin_save() {
                    Ok(request) => events.push(DetailEvent::Save(request)),
                    Err(error) => self.state.error = Some(error),
                }
            }
            if ui
                .add_enabled(changed && !locked, egui::Button::new("取消"))
                .clicked()
            {
                match self.state.reset() {
                    Ok(()) => events.push(DetailEvent::Discard),
                    Err(error) => self.state.error = Some(error),
                }
            }
            if self.state.phase() == Some(DetailSavePhase::Unknown)
                && ui.button("回查原结果").clicked()
            {
                match self.state.begin_read() {
                    Ok(request) => events.push(DetailEvent::Lookup(request)),
                    Err(error) => self.state.error = Some(error),
                }
            }
            if self.state.can_adopt()
                && ui.button("采用最新版本").clicked()
                && let Err(error) = self.state.adopt_latest()
            {
                self.state.error = Some(error);
            }
        });
        match self.save_phase() {
            Some(DetailSavePhase::Checkpointing) => {
                ui.label("检查点保存中");
            }
            Some(DetailSavePhase::Awaiting) => {
                ui.label("保存中");
            }
            Some(DetailSavePhase::Unknown) => {
                ui.label("结果待核实");
            }
            Some(DetailSavePhase::Reading) => {
                ui.label("回查中");
            }
            None => {}
        }
        if let Some(error) = self
            .state
            .error
            .as_ref()
            .or(invalid.as_ref().filter(|_| changed && !locked))
        {
            ui.colored_label(egui::Color32::RED, error);
        }
        events
    }
}
