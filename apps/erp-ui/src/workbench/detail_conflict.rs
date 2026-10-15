//! Read-only comparison of preserved current and stored intents.
use super::detail::{DetailPanel, DetailSaveOutcome, DetailSavePhase, DetailSaveRequest};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId};
use erp_ui_egui::{DataGrid, GridState};
use std::collections::BTreeMap;

pub(super) enum ConflictAction {
    Lookup(DetailSaveRequest),
    ConfirmTerminal,
}

pub(super) struct DetailConflict {
    pub stored: DetailPanel,
    pub stored_bytes: Vec<u8>,
    pub revision: u64,
    pub confirming: bool,
    open: bool,
    comparison: Option<Comparison>,
}
struct Comparison {
    current: View,
    stored: View,
    rows: Vec<(String, String, String)>,
    columns: Vec<Column>,
    grid: GridState,
}
#[derive(serde::Deserialize)]
struct View {
    record: String,
    version: String,
    changes: BTreeMap<String, String>,
    pending: Option<DetailSaveRequest>,
}
impl DetailConflict {
    pub fn new(scope: &str, bytes: Vec<u8>, revision: u64) -> Result<Self, String> {
        let stored = DetailPanel::restore(scope, &bytes)?;
        Ok(Self {
            stored,
            stored_bytes: bytes,
            revision,
            confirming: false,
            open: false,
            comparison: None,
        })
    }
    fn compare(&mut self, panel: &DetailPanel) -> Result<(), String> {
        let bytes = panel.checkpoint()?.ok_or("当前没有待保留草稿")?;
        let current: View = serde_json::from_slice(&bytes).map_err(|_| "无法读取当前草稿摘要")?;
        let stored: View =
            serde_json::from_slice(&self.stored_bytes).map_err(|_| "无法读取原检查点摘要")?;
        let mut keys: Vec<_> = current
            .changes
            .keys()
            .chain(stored.changes.keys())
            .cloned()
            .collect();
        keys.sort();
        keys.dedup();
        let rows = keys
            .into_iter()
            .map(|id| {
                let local = current.changes.get(&id).cloned().unwrap_or_default();
                let original = stored.changes.get(&id).cloned().unwrap_or_default();
                (panel.field_label(&id), local, original)
            })
            .collect();
        let mut grid = GridState::default();
        grid.read_only = true;
        grid.quick_search_open = false;
        grid.filtering_enabled = false;
        grid.sorting_enabled = false;
        self.comparison = Some(Comparison {
            current,
            stored,
            rows,
            columns: vec![
                Column::new(ColumnId(1), "字段", 90.0),
                Column::new(ColumnId(2), "当前草稿", 180.0),
                Column::new(ColumnId(3), "存储原意图", 180.0),
            ],
            grid,
        });
        self.open = true;
        Ok(())
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        panel: &DetailPanel,
    ) -> Result<Option<ConflictAction>, String> {
        if ui.button("核对保存冲突").clicked() {
            self.compare(panel)?;
        }
        let mut read = None;
        if self.open {
            egui::Window::new("保存冲突")
                .id(ui.id().with("detail-save-conflict"))
                .open(&mut self.open)
                .resizable(true)
                .default_size([650.0, 360.0])
                .show(ui.ctx(), |ui| {
                    if let Some(view) = &mut self.comparison {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "当前记录 {} · 版本 {}",
                                view.current.record, view.current.version
                            ));
                            ui.label(format!(
                                "存储记录 {} · 版本 {} · revision {}",
                                view.stored.record, view.stored.version, self.revision
                            ));
                        });
                        ui.horizontal(|ui| {
                            if let Some(request) = &view.current.pending {
                                ui.label(format!("当前操作 {}", request.operation_id));
                            }
                            if let Some(request) = &view.stored.pending {
                                ui.label(format!("存储原操作 {}", request.operation_id));
                            }
                        });
                        let height = (ui.available_height() - 50.0).max(120.0);
                        ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
                            DataGrid {
                                id: ui.id().with("conflict-data-grid"),
                                columns: &mut view.columns,
                                rows: view.rows.len(),
                                state: &mut view.grid,
                                row_id: Some(&|i| RowId(i as u128 + 1)),
                            }
                            .show(ui, |i, column| match column.0 {
                                1 => view.rows[i].0.clone(),
                                2 => view.rows[i].1.clone(),
                                _ => view.rows[i].2.clone(),
                            });
                        });
                        if self.stored.save_phase() == Some(DetailSavePhase::Unknown)
                            && ui.button("回查存储原结果").clicked()
                        {
                            match self.stored.begin_read() {
                                Ok(request) => read = Some(ConflictAction::Lookup(request)),
                                Err(error) => {
                                    ui.colored_label(egui::Color32::RED, error);
                                }
                            }
                        }
                        if !self.stored.has_unresolved_save()
                            && view.stored.pending.is_some()
                            && ui
                                .add_enabled(
                                    !panel.has_unresolved_save() && !self.confirming,
                                    egui::Button::new("确认原终态，保留当前草稿"),
                                )
                                .clicked()
                        {
                            read = Some(ConflictAction::ConfirmTerminal);
                        }
                        if self.stored.save_phase() == Some(DetailSavePhase::Reading) {
                            ui.label("原结果回查中");
                        }
                    }
                });
        }
        Ok(read)
    }
    pub fn resolve(
        &mut self,
        request: &DetailSaveRequest,
        outcome: DetailSaveOutcome,
    ) -> Result<(), String> {
        self.stored.resolve_save(request, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn panel(value: &str) -> DetailPanel {
        let mut p = DetailPanel::new("owner".into());
        p.bind(
            "one",
            "0",
            &[super::super::detail::DetailField {
                id: "name".into(),
                label: "Name".into(),
                value: "old".into(),
                editable: true,
                required: true,
            }],
        );
        p.edit_value("name", value.into()).expect("edit");
        p
    }
    #[test]
    fn detail_conflict_compare_keeps_both_identities_and_local_bytes() {
        let mut original = panel("original");
        let request = original.begin_save().expect("request");
        let bytes = original.checkpoint().expect("bytes").expect("some");
        let current = panel("local");
        let before = current.checkpoint().expect("local");
        let mut conflict = DetailConflict::new("owner", bytes.clone(), 4).expect("stored");
        conflict.compare(&current).expect("compare");
        assert_eq!(current.checkpoint().expect("after"), before);
        assert_eq!(conflict.stored.pending_request(), Some(&request));
        assert_eq!(conflict.stored_bytes, bytes);
        let view = conflict.comparison.as_ref().expect("view");
        assert_eq!(
            view.rows,
            [("Name".into(), "local".into(), "original".into())]
        );
    }
    #[test]
    fn detail_conflict_original_read_unknown_never_creates_write_or_new_id() {
        let mut original = panel("original");
        let request = original.begin_save().expect("request");
        let mut conflict = DetailConflict::new(
            "owner",
            original.checkpoint().expect("bytes").expect("some"),
            1,
        )
        .expect("conflict");
        let read = conflict.stored.begin_read().expect("read");
        assert_eq!(read, request);
        conflict
            .resolve(&read, DetailSaveOutcome::Unknown("not proved".into()))
            .expect("still unknown");
        assert_eq!(conflict.stored.pending_request(), Some(&request));
        assert_eq!(conflict.stored.save_phase(), Some(DetailSavePhase::Unknown));
    }
    #[test]
    fn detail_conflict_foreign_result_cannot_clear_stored_original() {
        let mut original = panel("original");
        let request = original.begin_save().expect("request");
        let mut conflict = DetailConflict::new(
            "owner",
            original.checkpoint().expect("bytes").expect("some"),
            1,
        )
        .expect("conflict");
        let mut wrong = request.clone();
        wrong.operation_id = uuid::Uuid::new_v4();
        assert!(
            conflict
                .resolve(
                    &wrong,
                    DetailSaveOutcome::Rejected {
                        code: "rejected".into(),
                        message: "wrong".into()
                    }
                )
                .is_err()
        );
        assert_eq!(conflict.stored.pending_request(), Some(&request));
    }
}
