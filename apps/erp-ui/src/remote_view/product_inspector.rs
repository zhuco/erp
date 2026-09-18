use super::*;
use eframe::egui;
use erp_grid_core::{ColumnId, RowId};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct ProductInspector {
    pub(super) open: bool,
    row: Option<RowId>,
    base: Option<RemoteRow>,
    values: BTreeMap<ColumnId, String>,
    queued: Vec<(ColumnId, String, String)>,
}
impl ProductInspector {
    pub(super) fn dirty(&self) -> bool {
        self.base.as_ref().is_some_and(|base| {
            self.values
                .iter()
                .any(|(c, v)| base.cells.get(c) != Some(v))
        })
    }
    pub(super) fn selected(&self) -> Option<RowId> {
        self.row
    }
    pub(super) fn select(&mut self, row: Option<&RemoteRow>) {
        let Some(row) = row else { return };
        if self.row == Some(row.id) || self.dirty() {
            return;
        }
        self.row = Some(row.id);
        self.base = Some(row.clone());
        self.values = row.cells.clone();
    }
    pub(super) fn sync(&mut self, row: Option<&RemoteRow>) {
        let Some(row) = row else { return };
        let editable = [
            super::products::NAME,
            super::products::CODE,
            super::products::BRAND,
            super::products::CATEGORY,
        ];
        if self.row == Some(row.id)
            && self.dirty()
            && editable.iter().all(|c| {
                self.values
                    .get(c)
                    .is_none_or(|v| row.cells.get(c) == Some(v))
            })
        {
            self.commit_applied(row);
        }
    }
    pub(super) fn close(&mut self) {
        if !self.dirty() {
            self.open = false;
        }
    }
    pub(super) fn discard(&mut self) {
        if let Some(base) = &self.base {
            self.values = base.cells.clone();
        }
        self.queued.clear();
        self.open = false;
    }
    pub(super) fn changes(&self) -> Vec<(ColumnId, String, String)> {
        let Some(base) = &self.base else {
            return Vec::new();
        };
        [
            super::products::NAME,
            super::products::CODE,
            super::products::BRAND,
            super::products::CATEGORY,
        ]
        .into_iter()
        .filter_map(|column| {
            let before = base.cells.get(&column).cloned().unwrap_or_default();
            let after = self.values.get(&column).cloned().unwrap_or_default();
            (before != after).then_some((column, before, after))
        })
        .collect()
    }
    pub(super) fn begin_queue(
        &mut self,
        changes: Vec<(ColumnId, String, String)>,
    ) -> Option<(ColumnId, String, String)> {
        let mut changes = changes.into_iter();
        let first = changes.next();
        self.queued = changes.collect();
        first
    }
    pub(super) fn next_queued(&mut self) -> Option<(ColumnId, String, String)> {
        (!self.queued.is_empty()).then(|| self.queued.remove(0))
    }
    pub(super) fn clear_queue(&mut self) {
        self.queued.clear();
    }
    pub(super) fn version(&self) -> Option<String> {
        self.base.as_ref().and_then(|row| row.version.clone())
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        enabled: bool,
    ) -> Option<Vec<(ColumnId, String, String)>> {
        let Some(base) = self.base.clone() else {
            ui.label("请选择商品");
            return None;
        };
        ui.horizontal(|ui| {
            ui.heading("商品编辑");
            if ui.button("关闭").clicked() {
                self.close();
            }
        });
        ui.label(format!("商品 ID: {}", base.id.to_uuid()));
        let fields = [
            (super::products::NAME, "名称"),
            (super::products::CODE, "编码"),
            (super::products::BRAND, "品牌"),
            (super::products::CATEGORY, "分类"),
        ];
        for (column, label) in fields.iter().copied() {
            let value = self.values.entry(column).or_default();
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(value).desired_width(180.0),
                );
            });
        }
        let changes = fields
            .into_iter()
            .filter_map(|(column, _)| {
                let before = base.cells.get(&column).cloned().unwrap_or_default();
                let after = self.values.get(&column).cloned().unwrap_or_default();
                (before != after).then_some((column, before, after))
            })
            .collect::<Vec<_>>();
        if !changes.is_empty() {
            if ui.add_enabled(enabled, egui::Button::new("保存")).clicked() {
                return Some(changes);
            }
            if ui.button("放弃修改").clicked() {
                self.values = base.cells.clone();
            }
        }
        None
    }
    pub(super) fn commit_applied(&mut self, row: &RemoteRow) {
        self.row = Some(row.id);
        self.base = Some(row.clone());
        self.values = row.cells.clone();
    }
    pub(super) fn mark_saved(
        &mut self,
        row: RowId,
        column: ColumnId,
        value: String,
        version: String,
    ) {
        if self.row != Some(row) {
            return;
        }
        if let Some(base) = &mut self.base {
            base.cells.insert(column, value.clone());
            base.version = Some(version);
        }
        self.values.insert(column, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: u128, version: &str, name: &str) -> RemoteRow {
        RemoteRow {
            id: RowId(id),
            version: Some(version.into()),
            cells: BTreeMap::from([
                (super::products::CODE, "P-1".into()),
                (super::products::NAME, name.into()),
                (super::products::BRAND, "Brand".into()),
                (super::products::CATEGORY, "Cat".into()),
            ]),
        }
    }
    #[test]
    fn queue_advances_only_after_ack_and_applies_new_version() {
        let mut inspector = ProductInspector::default();
        inspector.select(Some(&row(1, "v1", "Old")));
        let first = inspector
            .begin_queue(vec![
                (super::products::NAME, "Old".into(), "New".into()),
                (super::products::BRAND, "Brand".into(), "NewBrand".into()),
            ])
            .unwrap();
        assert_eq!(first.0, super::products::NAME);
        assert!(inspector.next_queued().is_some());
        inspector.mark_saved(RowId(1), super::products::NAME, "New".into(), "v2".into());
        assert_eq!(inspector.version().as_deref(), Some("v2"));
    }
    #[test]
    fn failed_queue_keeps_remaining_dirty_values() {
        let mut inspector = ProductInspector::default();
        inspector.select(Some(&row(1, "v1", "Old")));
        inspector.values.insert(super::products::NAME, "New".into());
        inspector
            .values
            .insert(super::products::BRAND, "NewBrand".into());
        let _ = inspector.begin_queue(vec![
            (super::products::NAME, "Old".into(), "New".into()),
            (super::products::BRAND, "Brand".into(), "NewBrand".into()),
        ]);
        inspector.clear_queue();
        assert!(inspector.dirty());
        assert_eq!(inspector.changes().len(), 2);
    }
    #[test]
    fn dirty_selection_change_keeps_original_row() {
        let mut inspector = ProductInspector::default();
        let first = row(1, "v1", "One");
        inspector.select(Some(&first));
        inspector
            .values
            .insert(super::products::NAME, "Draft".into());
        inspector.select(Some(&row(2, "v1", "Two")));
        assert_eq!(inspector.selected(), Some(RowId(1)));
        assert_eq!(
            inspector
                .values
                .get(&super::products::NAME)
                .map(String::as_str),
            Some("Draft")
        );
    }
}
