use super::actions::ProductActions;
use crate::edit_drafts::DraftCell;
use crate::remote_paste::Preview;
use eframe::egui;
use erp_grid_core::{
    Column, ColumnId, RowId,
    actions::{ActionOutcome, ActionState},
};
use erp_ui_egui::{ChangePreview, ChangePreviewEvent, ChangeResult, ChangeRowResult};
#[path = "paste_retry.rs"]
mod retry;

pub(super) struct PasteReview {
    preview: Preview,
    submitted: bool,
    panel: Option<ChangePreview>,
    error: Option<String>,
    source_columns: Vec<Column>,
    retry: Option<retry::Retry>,
}

impl PasteReview {
    pub(super) fn submitted(&self) -> bool {
        self.submitted
    }
}
impl PasteReview {
    pub(super) fn new(preview: Preview, source_columns: &[Column]) -> Self {
        let labels: Vec<_> = source_columns
            .iter()
            .map(|column| (column.id, column.title.clone()))
            .collect();
        let panel = ChangePreview::new(
            egui::Id::new("remote-paste-review-grid"),
            preview.edits.clone(),
            labels,
        );
        let error = panel.as_ref().err().cloned();
        Self {
            preview,
            submitted: false,
            panel: panel.ok(),
            error,
            source_columns: source_columns.to_vec(),
            retry: None,
        }
    }

    pub(super) fn unsent_preview(&self) -> Option<Preview> {
        if self.submitted {
            return None;
        }
        retained_preview(&self.preview, self.panel.as_ref()?.active_edits()).ok()
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        actions: &mut ProductActions,
        session: &super::Session,
        parent: Option<RowId>,
    ) -> bool {
        let mut close = false;
        egui::Window::new("粘贴预览")
            .id(ui.id().with("remote-paste-review"))
            .collapsible(false)
            .default_size(egui::vec2(980.0, 360.0))
            .show(ui.ctx(), |ui| {
                close = self.panel(ui, actions, session, parent);
            });
        close
    }
    fn panel(
        &mut self,
        ui: &mut egui::Ui,
        actions: &mut ProductActions,
        session: &super::Session,
        parent: Option<RowId>,
    ) -> bool {
        self.retry_tools(ui, actions, session, parent);
        let outcome_map = actions.outcomes();
        let finished = self.submitted
            && self.preview.commands.iter().all(|command| {
                matches!(
                    outcome_map.get(&command.operation_id),
                    Some(ActionState::Complete(_) | ActionState::Cancelled)
                )
            });
        let results = self
            .preview
            .commands
            .iter()
            .filter_map(|command| {
                let result = match outcome_map.get(&command.operation_id)? {
                    ActionState::Complete(ActionOutcome::Applied { .. }) => ChangeResult::Applied,
                    ActionState::Complete(ActionOutcome::Rejected { code, message }) => {
                        ChangeResult::Rejected(format!("{code}: {message}"))
                    }
                    ActionState::Cancelled => ChangeResult::Rejected("未发送，已取消".into()),
                    ActionState::Unknown | ActionState::IdentityConflict => {
                        ChangeResult::Unknown("需要查询原操作回执".into())
                    }
                    _ => return None,
                };
                Some(ChangeRowResult {
                    row: command.source,
                    result,
                })
            })
            .collect();
        let mut close = false;
        if let Some(panel) = &mut self.panel {
            let overrides = self
                .retry
                .as_ref()
                .and_then(|retry| retry.staged.as_ref())
                .map(|preview| {
                    preview
                        .edits
                        .iter()
                        .map(|edit| ((edit.row, edit.column), edit.before.clone()))
                        .collect()
                })
                .unwrap_or_default();
            if let Err(error) = panel.set_before_overrides(overrides) {
                self.error = Some(error);
            }
            if self.submitted {
                if let Err(error) = panel.set_row_results(results) {
                    self.error = Some(error);
                }
                panel.set_finished(finished);
            }
            if let Some(event) = panel.ui(ui) {
                match event {
                    ChangePreviewEvent::Confirm(edits) if actions.available() => {
                        match retained_preview(&self.preview, edits).and_then(|preview| {
                            actions.submit_paste(preview.commands.clone())?;
                            Ok(preview)
                        }) {
                            Ok(preview) => {
                                self.preview = preview;
                                self.submitted = true;
                                panel.mark_submitted();
                                self.error = None;
                            }
                            Err(error) => self.error = Some(error),
                        }
                    }
                    ChangePreviewEvent::Confirm(_) => {
                        self.error = Some("保存尚不可用".into());
                    }
                    ChangePreviewEvent::Cancel => close = !self.submitted || finished,
                }
            }
        } else if (!self.submitted || finished) && ui.button("Cancel").clicked() {
            close = true;
        }
        if let Some(error) = &self.error {
            ui.label(error);
        }
        if self.submitted && !finished {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        close
    }
}

fn retained_preview(
    preview: &Preview,
    edits: Vec<erp_grid_core::CellEdit>,
) -> Result<Preview, String> {
    let originals: std::collections::BTreeMap<_, _> = preview
        .commands
        .iter()
        .map(|command| (command.source, command))
        .collect();
    let mut values: std::collections::BTreeMap<
        RowId,
        std::collections::BTreeMap<ColumnId, String>,
    > = std::collections::BTreeMap::new();
    for edit in &edits {
        values
            .entry(edit.row)
            .or_default()
            .insert(edit.column, edit.after.clone());
    }
    let mut commands = Vec::with_capacity(values.len());
    for (source, values) in values {
        let original = originals
            .get(&source)
            .ok_or("Preview row identity changed")?;
        let mut command = (*original).clone();
        command.values = values;
        commands.push(command);
    }
    Ok(Preview { commands, edits })
}

pub(super) fn draft_cells(
    preview: &Preview,
    parent: Option<RowId>,
) -> Result<Vec<DraftCell>, String> {
    let mut versions = std::collections::BTreeMap::new();
    for command in &preview.commands {
        if command.operation_id.0 == 0 || command.source.0 == 0 {
            return Err("粘贴草稿缺少稳定身份".into());
        }
        let version = command
            .expected_version
            .clone()
            .filter(|version| !version.is_empty())
            .ok_or_else(|| "粘贴草稿缺少版本".to_string())?;
        if versions.insert(command.source, version).is_some() {
            return Err("粘贴草稿包含重复行".into());
        }
    }
    preview
        .edits
        .iter()
        .map(|edit| {
            let expected_version = versions
                .get(&edit.row)
                .cloned()
                .ok_or_else(|| "粘贴草稿缺少命令身份".to_string())?;
            Ok(DraftCell {
                row: edit.row,
                column: edit.column,
                before: edit.before.clone(),
                after: edit.after.clone(),
                expected_version,
                parent,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{CellEdit, actions::ActionCommand};
    use std::collections::BTreeMap;

    fn review() -> PasteReview {
        PasteReview::new(
            Preview {
                commands: vec![ActionCommand {
                    operation_id: RowId(91),
                    action: "products.update".into(),
                    source: RowId(10),
                    expected_version: Some("v10".into()),
                    values: BTreeMap::from([(ColumnId(1), "A".into()), (ColumnId(2), "B".into())]),
                }],
                edits: vec![
                    CellEdit {
                        row: RowId(10),
                        column: ColumnId(1),
                        before: "a".into(),
                        after: "A".into(),
                    },
                    CellEdit {
                        row: RowId(10),
                        column: ColumnId(2),
                        before: "b".into(),
                        after: "B".into(),
                    },
                ],
            },
            &[],
        )
    }

    #[test]
    fn remove_undo_redo_then_confirm_materializes_only_retained_field() {
        let mut review = review();
        review.panel = Some(
            ChangePreview::new(
                egui::Id::new("review-test"),
                review.preview.edits.clone(),
                [
                    (ColumnId(1), "First".into()),
                    (ColumnId(2), "Second".into()),
                ],
            )
            .unwrap(),
        );
        let panel = review.panel.as_mut().unwrap();
        assert!(panel.remove(erp_ui_egui::ChangeItemId(2)));
        assert_eq!(panel.active_edits().len(), 1);
        assert!(panel.undo());
        assert_eq!(panel.active_edits().len(), 2);
        assert!(panel.redo());
        let submitted = retained_preview(&review.preview, panel.active_edits()).unwrap();
        assert_eq!(submitted.commands.len(), 1);
        assert_eq!(
            submitted.commands[0].expected_version.as_deref(),
            Some("v10")
        );
        assert_eq!(
            submitted.commands[0].values,
            BTreeMap::from([(ColumnId(1), "A".into())])
        );
    }
}
