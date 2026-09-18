use crate::{DataGrid, GridState};
use egui::Id;
use erp_grid_core::{CellEdit, Column, ColumnId, RowId};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_CELLS: usize = 10_000;
const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_HISTORY: usize = 50;
const MAX_RESULT_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChangeItemId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeResult {
    Applied,
    Rejected(String),
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeRowResult {
    pub row: RowId,
    pub result: ChangeResult,
}

#[derive(Clone, Debug)]
pub enum ChangePreviewEvent {
    Confirm(Vec<CellEdit>),
    Cancel,
}

pub struct ChangePreview {
    id: Id,
    edits: Vec<CellEdit>,
    labels: BTreeMap<ColumnId, String>,
    removed: BTreeSet<ChangeItemId>,
    undo: VecDeque<BTreeSet<ChangeItemId>>,
    redo: Vec<BTreeSet<ChangeItemId>>,
    submitted: bool,
    finished: bool,
    results: BTreeMap<RowId, ChangeResult>,
    before_overrides: BTreeMap<(RowId, ColumnId), String>,
    state: GridState,
    columns: Vec<Column>,
    message: Option<String>,
}

impl ChangePreview {
    pub fn new(
        id: Id,
        edits: Vec<CellEdit>,
        column_labels: impl IntoIterator<Item = (ColumnId, String)>,
    ) -> Result<Self, String> {
        if edits.is_empty() || edits.len() > MAX_CELLS {
            return Err("Choose 1–10,000 changes".into());
        }
        let needed: BTreeSet<_> = edits.iter().map(|edit| edit.column).collect();
        let mut labels = BTreeMap::new();
        let mut label_bytes = 0usize;
        for (index, (column, label)) in column_labels.into_iter().enumerate() {
            if index >= MAX_CELLS || label.len() > MAX_RESULT_BYTES {
                return Err("Column labels exceed preview limits".into());
            }
            if needed.contains(&column) {
                label_bytes += label.len();
                if label_bytes > MAX_TEXT_BYTES {
                    return Err("Column labels exceed 1 MiB".into());
                }
                labels.insert(column, label);
            }
        }
        let mut seen = BTreeSet::new();
        let mut bytes = 0usize;
        for edit in &edits {
            if edit.row.0 == 0 || !seen.insert((edit.row, edit.column)) {
                return Err("Changes need unique stable cells".into());
            }
            if !labels.contains_key(&edit.column) {
                return Err("Change column label is missing".into());
            }
            bytes = bytes
                .checked_add(edit.before.len())
                .and_then(|value| value.checked_add(edit.after.len()))
                .ok_or("Change preview exceeds 1 MiB")?;
            if bytes > MAX_TEXT_BYTES {
                return Err("Change preview exceeds 1 MiB".into());
            }
        }
        let mut state = GridState {
            read_only: true,
            sorting_enabled: false,
            filtering_enabled: false,
            ..Default::default()
        };
        state.bind_view(1);
        Ok(Self {
            id,
            edits,
            labels,
            removed: BTreeSet::new(),
            undo: VecDeque::new(),
            redo: Vec::new(),
            submitted: false,
            finished: false,
            results: BTreeMap::new(),
            before_overrides: BTreeMap::new(),
            state,
            columns: ["Row", "Field", "Before", "After", "Result"]
                .into_iter()
                .enumerate()
                .map(|(index, title)| {
                    Column::new(
                        ColumnId(index as u64),
                        title,
                        if index == 0 { 220.0 } else { 150.0 },
                    )
                })
                .collect(),
            message: None,
        })
    }

    pub fn submitted(&self) -> bool {
        self.submitted
    }

    pub fn mark_submitted(&mut self) {
        self.submitted = true;
        self.finished = false;
        self.undo.clear();
        self.redo.clear();
        self.state.row_selection = Default::default();
    }

    pub fn set_finished(&mut self, finished: bool) {
        self.finished = self.submitted
            && finished
            && self
                .edits
                .iter()
                .enumerate()
                .filter(|(index, _)| !self.removed.contains(&item_id(*index)))
                .all(|(_, edit)| {
                    matches!(
                        self.results.get(&edit.row),
                        Some(ChangeResult::Applied | ChangeResult::Rejected(_))
                    )
                });
    }

    pub fn set_before_overrides(
        &mut self,
        values: BTreeMap<(RowId, ColumnId), String>,
    ) -> Result<(), String> {
        let cells: BTreeSet<_> = self
            .edits
            .iter()
            .map(|edit| (edit.row, edit.column))
            .collect();
        if values.len() > MAX_CELLS
            || values.keys().any(|key| !cells.contains(key))
            || values.values().any(|value| value.len() > MAX_RESULT_BYTES)
            || values.values().map(String::len).sum::<usize>() > MAX_TEXT_BYTES
        {
            return Err("Invalid display value override".into());
        }
        self.before_overrides = values;
        Ok(())
    }

    pub fn item_ids(&self) -> Vec<ChangeItemId> {
        (0..self.edits.len())
            .map(item_id)
            .filter(|id| !self.removed.contains(id))
            .collect()
    }

    pub fn active_edits(&self) -> Vec<CellEdit> {
        self.edits
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.removed.contains(&item_id(*index)))
            .map(|(_, edit)| edit.clone())
            .collect()
    }

    pub fn remove(&mut self, item: ChangeItemId) -> bool {
        self.remove_items([item])
    }

    pub fn can_undo(&self) -> bool {
        !self.submitted && !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.submitted && !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        if self.submitted {
            return false;
        }
        let Some(batch) = self.undo.pop_back() else {
            return false;
        };
        for item in &batch {
            self.removed.remove(item);
        }
        self.redo.push(batch);
        self.state.clear_selection();
        true
    }

    pub fn redo(&mut self) -> bool {
        if self.submitted {
            return false;
        }
        let Some(batch) = self.redo.pop() else {
            return false;
        };
        self.removed.extend(batch.iter().copied());
        self.undo.push_back(batch);
        self.state.clear_selection();
        true
    }

    pub fn set_row_results(&mut self, values: Vec<ChangeRowResult>) -> Result<(), String> {
        if !self.submitted {
            return Err("Confirm changes before attaching results".into());
        }
        let rows: BTreeSet<_> = self
            .edits
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.removed.contains(&item_id(*index)))
            .map(|(_, edit)| edit.row)
            .collect();
        let mut results = BTreeMap::new();
        let mut bytes = 0usize;
        for value in values {
            if !rows.contains(&value.row) || results.contains_key(&value.row) {
                return Err("Result row does not match the submitted changes".into());
            }
            let text = result_detail(&value.result);
            if text.len() > MAX_RESULT_BYTES {
                return Err("Change result exceeds 4096 bytes".into());
            }
            bytes = bytes
                .checked_add(text.len())
                .ok_or("Change results exceed 1 MiB")?;
            if bytes > MAX_TEXT_BYTES {
                return Err("Change results exceed 1 MiB".into());
            }
            results.insert(value.row, value.result);
        }
        self.results = results;
        self.set_finished(self.finished);
        Ok(())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<ChangePreviewEvent> {
        let mut event = None;
        ui.horizontal(|ui| {
            if !self.submitted {
                let selected: Vec<_> = self
                    .state
                    .row_selection
                    .targets()
                    .into_iter()
                    .filter_map(|row| u64::try_from(row.0).ok().map(ChangeItemId))
                    .collect();
                if ui
                    .add_enabled(!selected.is_empty(), egui::Button::new("Remove selected"))
                    .clicked()
                    && self.remove_items(selected)
                {
                    self.state.row_selection = Default::default();
                    self.message = None;
                }
                if ui
                    .add_enabled(self.can_undo(), egui::Button::new("Undo"))
                    .clicked()
                {
                    self.undo();
                    self.state.row_selection = Default::default();
                }
                if ui
                    .add_enabled(self.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                {
                    self.redo();
                    self.state.row_selection = Default::default();
                }
                if ui
                    .add_enabled(
                        self.removed.len() < self.edits.len(),
                        egui::Button::new("Confirm"),
                    )
                    .clicked()
                {
                    let edits = self.active_edits();
                    event = Some(ChangePreviewEvent::Confirm(edits));
                }
                if ui.button("Cancel").clicked() {
                    event = Some(ChangePreviewEvent::Cancel);
                }
            } else if ui
                .add_enabled(self.finished, egui::Button::new("Close"))
                .clicked()
            {
                event = Some(ChangePreviewEvent::Cancel);
            }
            if let Some(message) = &self.message {
                ui.label(message);
            }
        });

        let indices: Vec<_> = (0..self.edits.len())
            .filter(|index| !self.removed.contains(&item_id(*index)))
            .collect();
        ui.allocate_ui(egui::vec2(ui.available_width(), 240.0), |ui| {
            DataGrid {
                id: self.id,
                columns: &mut self.columns,
                rows: indices.len(),
                state: &mut self.state,
                row_id: Some(&|row| RowId(u128::from(item_id(indices[row]).0))),
            }
            .show(ui, |row, column| {
                let edit = &self.edits[indices[row]];
                match column.0 {
                    0 => edit.row.to_uuid(),
                    1 => self.labels.get(&edit.column).cloned().unwrap_or_default(),
                    2 => self
                        .before_overrides
                        .get(&(edit.row, edit.column))
                        .cloned()
                        .unwrap_or_else(|| edit.before.clone()),
                    3 => edit.after.clone(),
                    4 if self.submitted => self
                        .results
                        .get(&edit.row)
                        .map(result_label)
                        .unwrap_or("Submitted")
                        .to_owned(),
                    _ => String::new(),
                }
            });
        });
        event
    }

    fn remove_items(&mut self, items: impl IntoIterator<Item = ChangeItemId>) -> bool {
        if self.submitted {
            return false;
        }
        let batch: BTreeSet<_> = items
            .into_iter()
            .filter(|item| {
                item.0 >= 1
                    && usize::try_from(item.0).is_ok_and(|value| value <= self.edits.len())
                    && !self.removed.contains(item)
            })
            .collect();
        if batch.is_empty() {
            return false;
        }
        self.removed.extend(batch.iter().copied());
        self.undo.push_back(batch);
        while self.undo.len() > MAX_HISTORY {
            self.undo.pop_front();
        }
        self.redo.clear();
        self.state.clear_selection();
        true
    }
}

fn item_id(index: usize) -> ChangeItemId {
    ChangeItemId(index as u64 + 1)
}

fn result_detail(result: &ChangeResult) -> &str {
    match result {
        ChangeResult::Applied => "",
        ChangeResult::Rejected(value) | ChangeResult::Unknown(value) => value,
    }
}

fn result_label(result: &ChangeResult) -> &str {
    match result {
        ChangeResult::Applied => "Applied",
        ChangeResult::Rejected(value) | ChangeResult::Unknown(value) => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(row: u128, column: u64) -> CellEdit {
        CellEdit {
            row: RowId(row),
            column: ColumnId(column),
            before: "before".into(),
            after: "after".into(),
        }
    }

    #[test]
    fn removal_history_preserves_original_item_identity() {
        let mut preview = ChangePreview::new(
            Id::new("preview"),
            vec![edit(10, 1), edit(20, 1), edit(30, 1)],
            [(ColumnId(1), "Value".into())],
        )
        .unwrap();
        assert!(preview.remove(ChangeItemId(2)));
        assert_eq!(preview.item_ids(), [ChangeItemId(1), ChangeItemId(3)]);
        assert!(preview.remove(ChangeItemId(3)));
        assert!(preview.undo());
        assert_eq!(preview.item_ids(), [ChangeItemId(1), ChangeItemId(3)]);
        assert!(preview.undo());
        assert_eq!(
            preview.item_ids(),
            [ChangeItemId(1), ChangeItemId(2), ChangeItemId(3)]
        );
        assert!(preview.redo());
        assert_eq!(preview.active_edits()[1].row, RowId(30));
    }

    #[test]
    fn validation_and_submitted_results_are_bounded() {
        assert!(
            ChangePreview::new(
                Id::new("duplicate"),
                vec![edit(10, 1), edit(10, 1)],
                [(ColumnId(1), "Value".into())],
            )
            .is_err()
        );
        let mut preview = ChangePreview::new(
            Id::new("results"),
            vec![edit(10, 1)],
            [(ColumnId(1), "Value".into())],
        )
        .unwrap();
        assert!(preview.set_row_results(Vec::new()).is_err());
        preview.submitted = true;
        assert!(
            preview
                .set_row_results(vec![ChangeRowResult {
                    row: RowId(10),
                    result: ChangeResult::Applied,
                }])
                .is_ok()
        );
        assert!(!preview.remove(ChangeItemId(1)) && !preview.undo() && !preview.redo());
        preview.set_finished(true);
        assert!(preview.finished);
        preview
            .set_row_results(vec![ChangeRowResult {
                row: RowId(10),
                result: ChangeResult::Unknown("pending".into()),
            }])
            .unwrap();
        preview.set_finished(true);
        assert!(!preview.finished);
        assert!(
            preview
                .set_row_results(vec![ChangeRowResult {
                    row: RowId(99),
                    result: ChangeResult::Unknown("unknown".into()),
                }])
                .is_err()
        );
    }
}
