use erp_grid_core::{Column, ColumnId, Selection};

/// Copy only selected cells in current visual column order. Never consult checked rows.
pub fn selection_tsv(
    selection: &Selection,
    columns: &[Column],
    rows: usize,
    text: impl Fn(usize, ColumnId) -> String,
) -> Result<String, &'static str> {
    if selection.cell_count() > 10_000 {
        return Err("Copy limit: 10,000 cells. Narrow the selection.");
    }
    let mut columns: Vec<_> = columns.iter().filter(|column| column.visible).collect();
    columns.sort_by_key(|column| column.pin_rank());
    let mut result = String::new();
    for (i, area) in selection.regions().iter().enumerate() {
        if area.bottom > rows || area.right > columns.len() {
            return Err("Selection is stale; select again.");
        }
        if i > 0 {
            result.push_str("\n\n");
        }
        for row in area.top..area.bottom {
            if row != area.top {
                result.push('\n');
            }
            for (offset, column) in columns[area.left..area.right].iter().enumerate() {
                if offset > 0 {
                    result.push('\t');
                }
                let value = text(row, column.id);
                if value.len() + result.len() > 1_048_576 {
                    return Err("Copy limit: 1 MiB.");
                }
                // Prefix user text formulas; TSV is intended for spreadsheet pasting.
                let value = erp_grid_core::spreadsheet_value(&value, column.value_kind);
                if value.contains(['\t', '\n', '\r', '"']) {
                    result.push('"');
                    result.push_str(&value.replace('"', "\"\""));
                    result.push('"');
                } else {
                    result.push_str(&value);
                }
                if result.len() > 1_048_576 {
                    return Err("Copy limit: 1 MiB.");
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{CellRect, SelectionOperation};
    #[test]
    fn audit_signed_numbers_roundtrip_through_copy_and_paste_without_weakening_text_safety() {
        use erp_grid_core::{
            Cell, EditKind, RowId, ValueKind,
            edit_planner::{IndexedRow, plan_paste},
        };
        let mut columns = vec![
            Column::new(ColumnId(1), "Qty", 100.0),
            Column::new(ColumnId(2), "Money", 100.0),
        ];
        columns[0].value_kind = ValueKind::Integer;
        columns[0].edit_kind = EditKind::Integer;
        columns[1].value_kind = ValueKind::Money;
        columns[1].edit_kind = EditKind::Money;
        let mut selected = Selection::default();
        selected
            .apply(
                CellRect {
                    top: 0,
                    bottom: 1,
                    left: 0,
                    right: 2,
                },
                SelectionOperation::Replace,
            )
            .unwrap();
        let copied = selection_tsv(&selected, &columns, 1, |_, col| {
            if col.0 == 1 {
                "-2".into()
            } else {
                "-1.25".into()
            }
        })
        .unwrap();
        assert_eq!(copied, "-2\t-1.25");
        let rows = [IndexedRow {
            ordinal: 0,
            id: RowId(1),
            values: [(ColumnId(1), "0".into()), (ColumnId(2), "0.00".into())].into(),
        }];
        let edits = plan_paste(Cell { row: 0, column: 0 }, &copied, &columns, &rows).unwrap();
        assert_eq!(edits[0].after, "-2");
        assert_eq!(edits[1].after, "-1.25");
        for text in ["=SUM(A1)", " @cmd", "-label", "+label"] {
            assert!(erp_grid_core::spreadsheet_value(text, ValueKind::Text).starts_with('\''));
            assert!(erp_grid_core::spreadsheet_value(text, ValueKind::Money).starts_with('\''));
        }
    }
    #[test]
    fn copying_respects_visual_order_and_escapes_text() {
        let columns = [
            Column::new(ColumnId(9), "B", 100.0),
            Column::new(ColumnId(1), "A", 100.0),
        ];
        let mut selected = Selection::default();
        selected
            .apply(
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 1,
                    right: 2,
                },
                SelectionOperation::Replace,
            )
            .unwrap();
        assert_eq!(
            selection_tsv(&selected, &columns, 1, |_, c| if c.0 == 9 {
                "x\ty".into()
            } else {
                "=SUM(A1)".into()
            })
            .unwrap(),
            "\"x\ty\"\t'=SUM(A1)"
        );
        assert!(selection_tsv(&selected, &columns, 0, |_, _| String::new()).is_err());
    }
}
