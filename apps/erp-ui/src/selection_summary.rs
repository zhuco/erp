use erp_grid_core::{CellRect, Column, ColumnId, Selection, ValueKind, money_minor};

#[derive(PartialEq)]
struct SummaryKey {
    generation: u64,
    revision: u64,
    regions: Vec<CellRect>,
    columns: Vec<(ColumnId, ValueKind)>,
}

#[derive(Default)]
pub struct SelectionSummary {
    key: Option<SummaryKey>,
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
        let key = SummaryKey {
            generation,
            revision,
            regions: selection.regions().to_vec(),
            columns: visible.iter().map(|c| (c.id, c.value_kind)).collect(),
        };
        if self.key.as_ref() == Some(&key) {
            return;
        }
        self.key = Some(key);
        self.label.clear();
        if selection.cell_count() > 100_000 {
            self.label = "Σ: select ≤100,000 cells".into();
            return;
        }
        let scale = visible
            .iter()
            .filter_map(|column| match column.value_kind {
                ValueKind::Decimal { scale } => Some(scale),
                _ => None,
            })
            .max()
            .unwrap_or(2)
            .max(2);
        if scale > 8 {
            self.label = "Σ: unsupported decimal scale".into();
            return;
        }
        let (mut count, mut sum) = (0, 0_i128);
        for region in selection.regions() {
            for row in region.top..region.bottom.min(rows) {
                for column in visible.iter().take(region.right).skip(region.left) {
                    let cell = text(row, column.id);
                    let valid = match column.value_kind {
                        ValueKind::Integer => cell.parse::<i64>().is_ok(),
                        ValueKind::Money => money_minor(&cell).is_ok_and(|v| v.is_some()),
                        ValueKind::Decimal { scale } => {
                            erp_grid_core::decimal_scaled(&cell, scale).is_ok_and(|v| v.is_some())
                        }
                        _ => false,
                    };
                    if valid {
                        let value = erp_grid_core::decimal_scaled(&cell, scale).ok().flatten();
                        let Some(value) = value else {
                            self.label = "Σ: numeric range exceeded".into();
                            return;
                        };
                        let Some(next) = sum.checked_add(value) else {
                            self.label = "Σ: numeric range exceeded".into();
                            return;
                        };
                        count += 1;
                        sum = next;
                    }
                }
            }
        }
        if count > 1 {
            let mut value = erp_grid_core::decimal_text(sum, scale);
            if scale == 2 {
                match value.split_once('.').map(|(_, fraction)| fraction.len()) {
                    None => value.push_str(".00"),
                    Some(1) => value.push('0'),
                    _ => {}
                }
            }
            self.label = format!("Σ {value} ({count})");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{Cell, SelectionOperation};

    #[test]
    fn decimal_selection_sum_is_exact_and_overflow_is_visible() {
        let mut columns = vec![Column::new(ColumnId(0), "Qty", 80.0)];
        columns[0].value_kind = ValueKind::Decimal { scale: 6 };
        let mut selection = Selection::default();
        selection
            .apply(
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 2,
                    right: 1,
                },
                SelectionOperation::Replace,
            )
            .unwrap();
        let mut summary = SelectionSummary::default();
        summary.update(1, 0, &selection, &columns, 2, |row, _| {
            if row == 0 { "0.100001" } else { "0.2" }.into()
        });
        assert_eq!(summary.label, "Σ 0.300001 (2)");
        columns[0].value_kind = ValueKind::Decimal { scale: 8 };
        summary.update(1, 0, &selection, &columns, 2, |row, _| {
            if row == 0 { "0.133" } else { "0.014" }.into()
        });
        assert_eq!(summary.label, "Σ 0.147 (2)");
        summary.update(2, 0, &selection, &columns, 2, |_, _| {
            erp_grid_core::decimal_text(i128::MAX, 8)
        });
        assert_eq!(summary.label, "Σ: numeric range exceeded");
    }
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
