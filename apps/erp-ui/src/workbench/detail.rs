//! Reusable compact right hand detail form.  It only owns draft state and
//! emits events; the host decides how a successful save is persisted.

use eframe::egui;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailField {
    pub id: String,
    pub label: String,
    pub value: String,
    pub editable: bool,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailEvent {
    Save(Vec<(String, String)>),
    Discard,
    Blocked { current: String, requested: String },
}

#[derive(Default)]
pub struct DetailPanel {
    record_id: Option<String>,
    base: BTreeMap<String, String>,
    draft: BTreeMap<String, String>,
    editable: BTreeMap<String, bool>,
    required: BTreeMap<String, bool>,
    labels: BTreeMap<String, String>,
    save_pending: bool,
    error: Option<String>,
}

impl DetailPanel {
    pub fn is_dirty(&self) -> bool {
        self.save_pending
            || self
                .base
                .iter()
                .any(|(id, value)| self.draft.get(id) != Some(value))
    }

    pub fn bound_record(&self) -> Option<&str> {
        self.record_id.as_deref()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// A host should call this before changing its selected record.  Dirty
    /// drafts stay attached to their original record and produce Blocked.
    pub fn bind(&mut self, record_id: &str, fields: &[DetailField]) -> Option<DetailEvent> {
        if self.record_id.as_deref() == Some(record_id) {
            self.refresh_fields(fields);
            return None;
        }
        if let Some(current) = self.record_id.clone()
            && self.is_dirty()
        {
            return Some(DetailEvent::Blocked {
                current,
                requested: record_id.to_owned(),
            });
        }
        self.record_id = Some(record_id.to_owned());
        self.base.clear();
        self.draft.clear();
        self.refresh_fields(fields);
        self.error = None;
        None
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        record_id: &str,
        fields: &[DetailField],
    ) -> Vec<DetailEvent> {
        let mut events = Vec::new();
        if let Some(event) = self.bind(record_id, fields) {
            events.push(event);
            return events;
        }
        for field in fields {
            let panel_enabled = !self.save_pending;
            let value = self.draft.entry(field.id.clone()).or_default();
            let enabled = field.editable && panel_enabled;
            ui.horizontal(|ui| {
                let label = if field.required {
                    format!("{} *", field.label)
                } else {
                    field.label.clone()
                };
                ui.add_sized(
                    [78.0, 26.0],
                    egui::Label::new(
                        egui::RichText::new(label).color(ui.visuals().weak_text_color()),
                    )
                    .truncate(),
                );
                let response = if field.editable {
                    ui.add_enabled(
                        enabled,
                        egui::TextEdit::singleline(value).desired_width(ui.available_width()),
                    )
                } else {
                    ui.add_sized(
                        [ui.available_width(), 26.0],
                        egui::Label::new(value.as_str()).selectable(true).truncate(),
                    )
                };
                if field.required && value.trim().is_empty() {
                    response.on_hover_text("必填");
                }
            });
        }
        ui.horizontal(|ui| {
            let changed = self.changes();
            let has_changes = !changed.is_empty();
            let valid = self.validation_error(&changed).is_none();
            if ui
                .add_enabled(
                    has_changes && valid && !self.save_pending,
                    egui::Button::new("保存"),
                )
                .clicked()
            {
                self.save_pending = true;
                self.error = None;
                events.push(DetailEvent::Save(changed));
            }
            if ui
                .add_enabled(has_changes && !self.save_pending, egui::Button::new("取消"))
                .clicked()
            {
                self.draft = self.base.clone();
                self.error = None;
                events.push(DetailEvent::Discard);
            }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }
        });
        events
    }

    /// The host calls this exactly once for the emitted Save event.
    /// Failures preserve the draft so the user can correct or retry it.
    pub fn resolve_save(&mut self, result: Result<(), String>) {
        self.save_pending = false;
        match result {
            Ok(()) => {
                self.base = self.draft.clone();
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn refresh_fields(&mut self, fields: &[DetailField]) {
        let known: std::collections::BTreeSet<_> =
            fields.iter().map(|field| field.id.clone()).collect();
        if !self.is_dirty() {
            self.base.retain(|id, _| known.contains(id));
            self.draft.retain(|id, _| known.contains(id));
        } else {
            let keep: std::collections::BTreeSet<_> = self
                .base
                .iter()
                .filter(|(id, value)| known.contains(*id) || self.draft.get(*id) != Some(*value))
                .map(|(id, _)| id.clone())
                .collect();
            self.base.retain(|id, _| keep.contains(id));
            self.draft.retain(|id, _| keep.contains(id));
        }
        for field in fields {
            self.labels.insert(field.id.clone(), field.label.clone());
            self.editable.insert(field.id.clone(), field.editable);
            self.required.insert(field.id.clone(), field.required);
            let field_dirty = self.base.get(&field.id) != self.draft.get(&field.id);
            self.base
                .entry(field.id.clone())
                .or_insert_with(|| field.value.clone());
            self.draft
                .entry(field.id.clone())
                .or_insert_with(|| field.value.clone());
            if !field_dirty && !self.save_pending {
                self.base.insert(field.id.clone(), field.value.clone());
                self.draft.insert(field.id.clone(), field.value.clone());
            }
        }
    }

    fn changes(&self) -> Vec<(String, String)> {
        self.base
            .iter()
            .filter_map(|(id, before)| {
                let after = self.draft.get(id)?;
                (before != after).then(|| (id.clone(), after.clone()))
            })
            .collect()
    }

    fn validation_error(&self, changes: &[(String, String)]) -> Option<String> {
        for (id, value) in changes {
            if self.required.get(id).copied().unwrap_or(false) && value.trim().is_empty() {
                return Some(format!(
                    "{}不能为空",
                    self.labels.get(id).map(String::as_str).unwrap_or(id)
                ));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(value: &str, required: bool) -> DetailField {
        DetailField {
            id: "name".into(),
            label: "名称".into(),
            value: value.into(),
            editable: true,
            required,
        }
    }

    #[test]
    fn dirty_draft_blocks_record_change_and_failed_save_keeps_it() {
        let mut panel = DetailPanel::default();
        assert_eq!(panel.bind("one", &[field("old", true)]), None);
        panel.draft.insert("name".into(), "draft".into());
        assert!(panel.is_dirty());
        assert!(matches!(
            panel.bind("two", &[field("other", true)]),
            Some(DetailEvent::Blocked { .. })
        ));
        panel.save_pending = true;
        panel.resolve_save(Err("拒绝".into()));
        assert!(panel.is_dirty());
        assert_eq!(panel.draft.get("name").map(String::as_str), Some("draft"));
        panel.resolve_save(Ok(()));
        assert!(!panel.is_dirty());
    }

    #[test]
    fn required_validation_rejects_blank_change() {
        let mut panel = DetailPanel::default();
        panel.bind("one", &[field("old", true)]);
        panel.draft.insert("name".into(), "  ".into());
        assert_eq!(
            panel.validation_error(&panel.changes()),
            Some("名称不能为空".into())
        );
    }

    #[test]
    fn same_record_refresh_updates_clean_fields_and_preserves_dirty_fields() {
        let mut panel = DetailPanel::default();
        let fields = vec![
            field("old", true),
            DetailField {
                id: "status".into(),
                label: "状态".into(),
                value: "待处理".into(),
                editable: false,
                required: false,
            },
        ];
        panel.bind("one", &fields);
        panel.draft.insert("name".into(), "本地草稿".into());
        panel.bind(
            "one",
            &[
                field("server-new", true),
                DetailField {
                    id: "status".into(),
                    label: "状态".into(),
                    value: "已处理".into(),
                    editable: false,
                    required: false,
                },
            ],
        );
        assert_eq!(
            panel.draft.get("name").map(String::as_str),
            Some("本地草稿")
        );
        assert_eq!(
            panel.draft.get("status").map(String::as_str),
            Some("已处理")
        );
    }
}
