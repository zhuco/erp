use erp_grid_core::{CellRect, Column, ColumnId, Selection, ValueKind, money_minor};

#[derive(Default)]
pub struct SelectionSummary {
    key: Option<(u64, u64, Vec<CellRect>, Vec<ColumnId>)>,
    pub label: String,
}
impl SelectionSummary {
    pub fn update(
        &mut self,
        generation: u64,
        revision: u64,
        selection: &Selection,
        columns: &[Column],
        rows: usize,
        text: impl Fn(usize, ColumnId) -> String,
    ) {
        let mut visible: Vec<_> = columns.iter().filter(|c| c.visible).collect();
        visible.sort_by_key(|c| c.pin_rank());
        let key = (
            generation,
            revision,
            selection.regions().to_vec(),
            visible.iter().map(|c| c.id).collect(),
        );
        if self.key.as_ref() == Some(&key) {
            return;
        }
        self.key = Some(key);
        self.label.clear();
        if selection.cell_count() > 100_000 {
            self.label = "Σ: select ≤100,000 cells".into();
            return;
        }
        let (mut count, mut sum) = (0, 0_i128);
        for region in selection.regions() {
            for row in region.top..region.bottom.min(rows) {
                for column in visible.iter().take(region.right).skip(region.left) {
                    let value = match column.value_kind {
                        ValueKind::Integer => text(row, column.id)
                            .parse::<i64>()
                            .ok()
                            .map(|v| i128::from(v) * 100),
                        ValueKind::Money => money_minor(&text(row, column.id))
                            .ok()
                            .flatten()
                            .map(i128::from),
                        _ => None,
                    };
                    if let Some(value) = value {
                        count += 1;
                        sum += value;
                    }
                }
            }
        }
        if count > 1 {
            self.label = format!(
                "Σ {}{}.{:02} ({count})",
                if sum < 0 { "-" } else { "" },
                sum.abs() / 100,
                sum.abs() % 100
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{Cell, SelectionOperation};
    #[test]
    fn visible_numeric_cells_sum_exactly_and_subtraction_recomputes() {
        let mut cols = vec![
            Column::new(ColumnId(0), "Text", 80.0),
            Column::new(ColumnId(1), "Amount", 80.0),
        ];
        cols[1].value_kind = ValueKind::Money;
        let mut selection = Selection::default();
        selection
            .apply(
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 2,
                    right: 2,
                },
                SelectionOperation::Replace,
            )
            .unwrap();
        let mut summary = SelectionSummary::default();
        summary.update(1, 0, &selection, &cols, 2, |row, col| {
            if col.0 == 0 {
                "999".into()
            } else if row == 0 {
                "0.10".into()
            } else {
                "-0.20".into()
            }
        });
        assert_eq!(summary.label, "Σ -0.10 (2)");
        selection
            .apply(
                CellRect::between(Cell { row: 0, column: 1 }, Cell { row: 0, column: 1 }),
                SelectionOperation::Subtract,
            )
            .unwrap();
        summary.update(1, 0, &selection, &cols, 2, |_, _| "3.00".into());
        assert!(summary.label.is_empty());
    }
}
