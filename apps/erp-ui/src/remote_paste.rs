//! Pure planning for bounded remote-grid paste. Persistence and dispatch stay in the host.
use erp_grid_core::{
    Cell, CellEdit, CellRect, Column, ColumnId, EditKind, RowId,
    actions::{ActionCommand, ActionOutcome, ActionState},
    money_minor,
    remote::RemoteRow,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_PASTE_ROWS: usize = 10_000;

pub(crate) struct Preview {
    pub commands: Vec<ActionCommand>,
    pub edits: Vec<CellEdit>,
}

struct RowPlan {
    source: RowId,
    version: String,
    values: BTreeMap<ColumnId, String>,
    edits: Vec<CellEdit>,
}

pub(crate) use erp_grid_core::edit_planner::FillAxis;

pub(crate) fn fill_axis(source: CellRect, target: CellRect) -> Result<FillAxis, String> {
    erp_grid_core::edit_planner::fill_axis(source, target)
}

pub(crate) fn plan(
    action: &str,
    start: Cell,
    text: &str,
    columns: &[Column],
    rows: &[RemoteRow],
) -> Result<Preview, String> {
    supported(action, "paste")?;
    if rows.len() > MAX_PASTE_ROWS {
        return Err("Paste row snapshot exceeds limits".into());
    }
    let indexed = rows
        .iter()
        .enumerate()
        .map(|(ordinal, row)| indexed_row(ordinal, row))
        .collect::<Vec<_>>();
    let edits = erp_grid_core::edit_planner::plan_paste(start, text, columns, &indexed)?;
    business_preview(action, edits, rows, columns)
}

/// Plan a bounded remote fill from a loaded source snapshot into a loaded target rectangle.
/// Both rectangles use half-open view coordinates; `target` contains only the new fill area.
pub(crate) fn plan_fill(
    action: &str,
    source: CellRect,
    target: CellRect,
    columns: &[Column],
    rows: &[RemoteRow],
) -> Result<Preview, String> {
    supported(action, "fill")?;
    fill_axis(source, target)?;
    if source.bottom > rows.len() || target.bottom > rows.len() {
        return Err("Fill reaches rows that are not loaded".into());
    }
    let source_rows = rows[source.top..source.bottom]
        .iter()
        .enumerate()
        .map(|(offset, row)| indexed_row(source.top + offset, row))
        .collect::<Vec<_>>();
    let target_rows = rows[target.top..target.bottom]
        .iter()
        .enumerate()
        .map(|(offset, row)| indexed_row(target.top + offset, row))
        .collect::<Vec<_>>();
    let edits = erp_grid_core::edit_planner::plan_fill(
        source,
        &source_rows,
        target,
        &target_rows,
        columns,
    )?;
    business_preview(action, edits, rows, columns)
}

/// Plan a vertical fill whose source and targets use zero-based ordinals in one
/// server-frozen query. Only the supplied rows are materialized.
pub(crate) fn plan_indexed_fill(
    action: &str,
    source: CellRect,
    source_rows: &[RemoteRow],
    target: CellRect,
    target_rows: &[(usize, RemoteRow)],
    columns: &[Column],
) -> Result<Preview, String> {
    supported(action, "fill")?;
    if source_rows.len() > MAX_PASTE_ROWS
        || target_rows.len() > MAX_PASTE_ROWS
        || source.bottom.checked_sub(source.top) != Some(source_rows.len())
    {
        return Err("Indexed source rows are incomplete or exceed limits".into());
    }
    if erp_grid_core::edit_planner::fill_axis(source, target)?
        != erp_grid_core::edit_planner::FillAxis::Vertical
    {
        return Err("Indexed fill supports only vertical ranges".into());
    }
    let indexed_source = source_rows
        .iter()
        .enumerate()
        .map(|(offset, row)| indexed_row(source.top + offset, row))
        .collect::<Vec<_>>();
    let indexed_target = target_rows
        .iter()
        .map(|(ordinal, row)| indexed_row(*ordinal, row))
        .collect::<Vec<_>>();
    let edits = erp_grid_core::edit_planner::plan_fill(
        source,
        &indexed_source,
        target,
        &indexed_target,
        columns,
    )?;
    let rows = source_rows
        .iter()
        .cloned()
        .chain(target_rows.iter().map(|(_, row)| row.clone()))
        .collect::<Vec<_>>();
    business_preview(action, edits, &rows, columns)
}

fn supported(action: &str, operation: &str) -> Result<(), String> {
    match action {
        "products.update" | "skus.update" => Ok(()),
        _ => Err(format!(
            "Remote {operation} supports only product or SKU updates"
        )),
    }
}
fn indexed_row(ordinal: usize, row: &RemoteRow) -> erp_grid_core::edit_planner::IndexedRow {
    erp_grid_core::edit_planner::IndexedRow {
        ordinal,
        id: row.id,
        values: row.cells.clone(),
    }
}
fn business_preview(
    action: &str,
    planned: Vec<CellEdit>,
    rows: &[RemoteRow],
    columns: &[Column],
) -> Result<Preview, String> {
    let mut by_id = BTreeMap::new();
    for row in rows {
        if row.id.0 == 0 || by_id.insert(row.id, row).is_some() {
            return Err("Loaded rows contain zero or duplicate identities".into());
        }
    }
    let mut plans: Vec<RowPlan> = Vec::new();
    let mut plan_indices = BTreeMap::new();
    let column_index: BTreeMap<_, _> = columns.iter().map(|column| (column.id, column)).collect();
    for edit in planned {
        let row = by_id
            .get(&edit.row)
            .ok_or("Planned edit row is not loaded")?;
        let column = column_index
            .get(&edit.column)
            .ok_or("Planned edit column is not displayed")?;
        validate_cell(action, column, row, &edit.after)
            .map_err(|error| format!("{}, {}: {error}", row.id.to_uuid(), column.title))?;
        let index = if let Some(index) = plan_indices.get(&edit.row) {
            *index
        } else {
            let version = row
                .version
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or("Row has no save version")?;
            plans.push(RowPlan {
                source: edit.row,
                version,
                values: BTreeMap::new(),
                edits: Vec::new(),
            });
            let index = plans.len() - 1;
            plan_indices.insert(edit.row, index);
            index
        };
        plans[index].values.insert(edit.column, edit.after.clone());
        plans[index].edits.push(edit);
    }
    let mut commands = Vec::with_capacity(plans.len());
    let mut edits = Vec::new();
    for plan in plans {
        let command = ActionCommand {
            operation_id: RowId(uuid::Uuid::new_v4().as_u128()),
            action: action.into(),
            source: plan.source,
            expected_version: Some(plan.version),
            values: plan.values,
        };
        command.validate().map_err(String::from)?;
        commands.push(command);
        edits.extend(plan.edits);
    }
    Ok(Preview { commands, edits })
}

pub(crate) fn prepare_retry(
    preview: &Preview,
    outcomes: &BTreeMap<RowId, ActionState>,
    latest: &[RemoteRow],
    columns: &[Column],
) -> Result<Preview, String> {
    if preview.commands.len() > MAX_PASTE_ROWS
        || latest.len() > MAX_PASTE_ROWS
        || latest
            .iter()
            .map(|row| {
                row.cells.values().map(String::len).sum::<usize>()
                    + row.version.as_ref().map_or(0, String::len)
            })
            .try_fold(0usize, usize::checked_add)
            .is_none_or(|bytes| bytes > 8 * 1024 * 1024)
    {
        return Err("Retry data exceeds the bounded preview budget".into());
    }
    let mut operations = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut retry = Vec::new();
    for command in &preview.commands {
        if !operations.insert(command.operation_id) || !sources.insert(command.source) {
            return Err("Original paste contains duplicate operation or row identities".into());
        }
        match outcomes.get(&command.operation_id) {
            Some(ActionState::Complete(ActionOutcome::Applied { .. })) => {}
            Some(
                ActionState::Complete(ActionOutcome::Rejected { .. }) | ActionState::Cancelled,
            ) => {
                retry.push(command);
            }
            Some(_) => {
                return Err("Every original paste operation must have a terminal result".into());
            }
            None => return Err("An original paste operation result is missing".into()),
        }
    }

    let retry_sources: BTreeSet<_> = retry.iter().map(|command| command.source).collect();
    let mut latest_by_source = BTreeMap::new();
    for row in latest {
        if row.id.0 == 0 || latest_by_source.insert(row.id, row).is_some() {
            return Err("Latest rows contain zero or duplicate identities".into());
        }
        if !retry_sources.contains(&row.id) {
            return Err("Latest rows include a successful or unrelated row".into());
        }
        if row.version.as_deref().is_none_or(str::is_empty) {
            return Err("A latest row has no save version".into());
        }
    }
    if latest_by_source.len() != retry_sources.len() {
        return Err("A retryable row is missing from the latest rows".into());
    }

    let mut columns_by_id = BTreeMap::new();
    for column in columns {
        if columns_by_id.insert(column.id, column).is_some() {
            return Err("Displayed columns contain duplicate identities".into());
        }
    }

    let mut commands = Vec::with_capacity(retry.len());
    let mut edits = Vec::new();
    for original in retry {
        match original.action.as_str() {
            "products.update" | "skus.update" => {}
            _ => return Err("Original paste contains an unsupported action".into()),
        }
        let row = latest_by_source
            .get(&original.source)
            .ok_or("A retryable row is missing from the latest rows")?;
        let mut values = BTreeMap::new();
        let mut row_edits = Vec::with_capacity(original.values.len());
        for (column_id, after) in &original.values {
            let column = columns_by_id
                .get(column_id)
                .ok_or("An original paste column is no longer displayed")?;
            validate_cell(&original.action, column, row, after)
                .map_err(|error| format!("{}, {}: {error}", row.id.to_uuid(), column.title))?;
            let before = row
                .cells
                .get(column_id)
                .ok_or("An original paste cell is missing from the latest row")?;
            if before == after {
                continue;
            }
            values.insert(*column_id, after.clone());
            row_edits.push(CellEdit {
                row: row.id,
                column: *column_id,
                before: before.clone(),
                after: after.clone(),
            });
        }
        if values.is_empty() {
            continue;
        }
        let command = ActionCommand {
            operation_id: RowId(uuid::Uuid::new_v4().as_u128()),
            action: original.action.clone(),
            source: original.source,
            expected_version: row.version.clone(),
            values,
        };
        command.validate().map_err(String::from)?;
        commands.push(command);
        edits.extend(row_edits);
    }
    Ok(Preview { commands, edits })
}

fn validate_cell(
    action: &str,
    column: &Column,
    row: &RemoteRow,
    value: &str,
) -> Result<(), &'static str> {
    if matches!(column.edit_kind, EditKind::ReadOnly) {
        return Err("Column is read-only");
    }
    column.edit_kind.validate(value)?;
    match (action, column.id.0) {
        ("products.update", 1) if value.trim().is_empty() => Err("Product name is required"),
        ("products.update", 0..=3) => Ok(()),
        ("skus.update", 0) if !value.trim().is_empty() && value.trim().len() <= 255 => Ok(()),
        ("skus.update", 0) => Err("SKU code is required and must not exceed 255 bytes"),
        ("skus.update", 1) if value.trim().len() <= 255 => Ok(()),
        ("skus.update", 1) => Err("SKU barcode must not exceed 255 bytes"),
        ("skus.update", 2) if value.trim().len() <= 500 => Ok(()),
        ("skus.update", 2) => Err("SKU name must not exceed 500 bytes"),
        ("skus.update", 4) => {
            if row
                .cells
                .get(&ColumnId(3))
                .is_some_and(|kind| kind == "KIT")
            {
                return Err("KIT cost is derived and read-only");
            }
            let cents = money_minor(value)?;
            if cents.is_some_and(|cents| cents < 0) {
                return Err("SKU cost cannot be negative");
            }
            Ok(())
        }
        ("skus.update", 5) if value == "true" || value == "false" => Ok(()),
        ("skus.update", 5) => Err("SKU active value must be true or false"),
        ("products.update", _) => Err("Column is not editable by product updates"),
        ("skus.update", _) => Err("Column is not editable by SKU updates"),
        _ => Err("Unknown remote update action"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::ValueKind;

    fn column(id: u64, edit_kind: EditKind) -> Column {
        let mut column = Column::new(ColumnId(id), format!("C{id}"), 100.0);
        column.edit_kind = edit_kind;
        column
    }

    fn row(id: u128, version: Option<&str>, values: &[(u64, &str)]) -> RemoteRow {
        RemoteRow {
            id: RowId(id),
            version: version.map(str::to_owned),
            cells: values
                .iter()
                .map(|(column, value)| (ColumnId(*column), (*value).to_owned()))
                .collect(),
        }
    }

    #[test]
    fn quoted_product_paste_groups_columns_per_row_and_skips_unchanged() {
        let columns = [
            column(0, EditKind::OptionalText),
            column(1, EditKind::Text),
            column(2, EditKind::OptionalText),
        ];
        let rows = [
            row(1, Some("v1"), &[(0, "P1"), (1, "Old"), (2, "A")]),
            row(2, Some("v2"), &[(0, "P2"), (1, "Second"), (2, "B")]),
        ];
        let preview = plan(
            "products.update",
            Cell { row: 0, column: 1 },
            "\"New\nName\"\tA\r\nSecond\tC",
            &columns,
            &rows,
        )
        .unwrap();
        assert_eq!(preview.commands.len(), 2);
        assert_eq!(preview.edits.len(), 2);
        assert_eq!(preview.commands[0].source, RowId(1));
        assert_eq!(preview.commands[0].values.len(), 1);
        assert_eq!(preview.commands[0].values[&ColumnId(1)], "New\nName");
        assert_eq!(preview.commands[1].values[&ColumnId(2)], "C");
        assert_ne!(
            preview.commands[0].operation_id,
            preview.commands[1].operation_id
        );
        assert_eq!(preview.commands[0].expected_version.as_deref(), Some("v1"));
    }

    #[test]
    fn unchanged_rectangle_produces_no_commands_or_edits() {
        let columns = [column(1, EditKind::Text)];
        let rows = [row(1, Some("v1"), &[(1, "Same")])];
        let preview = plan(
            "products.update",
            Cell { row: 0, column: 0 },
            "Same",
            &columns,
            &rows,
        )
        .unwrap();
        assert!(preview.commands.is_empty());
        assert!(preview.edits.is_empty());
    }

    #[test]
    fn rejects_unloaded_read_only_missing_version_and_ranges_atomically() {
        let editable = [column(1, EditKind::Text)];
        let loaded = [row(1, Some("v1"), &[(1, "A")])];
        assert!(
            plan(
                "products.update",
                Cell { row: 1, column: 0 },
                "B",
                &editable,
                &loaded
            )
            .is_err()
        );
        assert!(
            plan(
                "products.update",
                Cell {
                    row: usize::MAX,
                    column: 0
                },
                "B",
                &editable,
                &loaded
            )
            .is_err()
        );
        assert!(
            plan(
                "products.update",
                Cell { row: 0, column: 0 },
                "A\nB",
                &editable,
                &loaded
            )
            .is_err()
        );
        let readonly = [column(4, EditKind::ReadOnly)];
        assert!(
            plan(
                "products.update",
                Cell { row: 0, column: 0 },
                "ACTIVE",
                &readonly,
                &[row(1, Some("v1"), &[(4, "ACTIVE")])]
            )
            .is_err()
        );
        assert!(
            plan(
                "products.update",
                Cell { row: 0, column: 0 },
                "B",
                &editable,
                &[row(1, None, &[(1, "A")])]
            )
            .is_err()
        );
    }

    #[test]
    fn sku_rules_reject_bad_money_kit_cost_and_non_backend_columns() {
        let cost = [column(4, EditKind::OptionalText)];
        let normal = [row(1, Some("v1"), &[(3, "NORMAL"), (4, "1.00")])];
        assert!(
            plan(
                "skus.update",
                Cell { row: 0, column: 0 },
                "1.001",
                &cost,
                &normal
            )
            .is_err()
        );
        assert!(
            plan(
                "skus.update",
                Cell { row: 0, column: 0 },
                "-1.00",
                &cost,
                &normal
            )
            .is_err()
        );
        let kit = [row(2, Some("v2"), &[(3, "KIT"), (4, "2.00")])];
        assert!(
            plan(
                "skus.update",
                Cell { row: 0, column: 0 },
                "3.00",
                &cost,
                &kit
            )
            .is_err()
        );
        let unsupported = [column(6, EditKind::OptionalText)];
        assert!(
            plan(
                "skus.update",
                Cell { row: 0, column: 0 },
                "ACTIVE",
                &unsupported,
                &[row(3, Some("v3"), &[(6, "INACTIVE")])]
            )
            .is_err()
        );
    }

    fn rejected() -> ActionState {
        ActionState::Complete(ActionOutcome::Rejected {
            code: "VERSION_CONFLICT".into(),
            message: "refresh".into(),
        })
    }

    #[test]
    fn retry_excludes_successful_rows_and_rebases_conflicts() {
        let columns = [column(1, EditKind::Text)];
        let original_rows = [
            row(1, Some("v1"), &[(1, "Old one")]),
            row(2, Some("v2"), &[(1, "Old two")]),
        ];
        let original = plan(
            "products.update",
            Cell { row: 0, column: 0 },
            "Wanted one\nWanted two",
            &columns,
            &original_rows,
        )
        .unwrap();
        let first_operation = original.commands[0].operation_id;
        let second_operation = original.commands[1].operation_id;
        let outcomes = BTreeMap::from([
            (
                first_operation,
                ActionState::Complete(ActionOutcome::Applied {
                    row: RowId(1),
                    version: "saved".into(),
                    values: BTreeMap::new(),
                }),
            ),
            (second_operation, rejected()),
        ]);
        let retried = prepare_retry(
            &original,
            &outcomes,
            &[row(2, Some("v3"), &[(1, "Concurrent value")])],
            &columns,
        )
        .unwrap();
        assert_eq!(retried.commands.len(), 1);
        assert_eq!(retried.commands[0].source, RowId(2));
        assert_eq!(retried.commands[0].expected_version.as_deref(), Some("v3"));
        assert_eq!(retried.commands[0].values[&ColumnId(1)], "Wanted two");
        assert_ne!(retried.commands[0].operation_id, second_operation);
        assert_eq!(retried.edits[0].before, "Concurrent value");
        assert_eq!(retried.edits[0].after, "Wanted two");
        assert_eq!(original.commands[1].operation_id, second_operation);
        assert_eq!(original.commands[1].expected_version.as_deref(), Some("v2"));
    }

    #[test]
    fn retry_blocks_unknown_or_missing_latest_rows() {
        let columns = [column(1, EditKind::Text)];
        let original = plan(
            "products.update",
            Cell { row: 0, column: 0 },
            "Wanted one\nWanted two",
            &columns,
            &[
                row(1, Some("v1"), &[(1, "Old one")]),
                row(2, Some("v2"), &[(1, "Old two")]),
            ],
        )
        .unwrap();
        let blocked = BTreeMap::from([
            (original.commands[0].operation_id, ActionState::Unknown),
            (original.commands[1].operation_id, rejected()),
        ]);
        assert!(prepare_retry(&original, &blocked, &[], &columns).is_err());

        let terminal = BTreeMap::from([
            (original.commands[0].operation_id, rejected()),
            (original.commands[1].operation_id, ActionState::Cancelled),
        ]);
        assert!(
            prepare_retry(
                &original,
                &terminal,
                &[row(1, Some("v3"), &[(1, "Latest")])],
                &columns,
            )
            .is_err()
        );
    }

    #[test]
    fn retry_skips_values_already_applied() {
        let columns = [column(1, EditKind::Text), column(2, EditKind::OptionalText)];
        let original = plan(
            "products.update",
            Cell { row: 0, column: 0 },
            "Wanted\tBrand",
            &columns,
            &[row(1, Some("v1"), &[(1, "Old"), (2, "Old brand")])],
        )
        .unwrap();
        let outcomes = BTreeMap::from([(original.commands[0].operation_id, rejected())]);
        let retried = prepare_retry(
            &original,
            &outcomes,
            &[row(1, Some("v2"), &[(1, "Wanted"), (2, "Newer brand")])],
            &columns,
        )
        .unwrap();
        assert_eq!(retried.commands[0].values.len(), 1);
        assert_eq!(retried.commands[0].values[&ColumnId(2)], "Brand");
        assert_eq!(retried.edits.len(), 1);
    }

    #[test]
    fn retry_rejects_cost_when_latest_sku_became_kit() {
        let columns = [column(4, EditKind::OptionalText)];
        let original = plan(
            "skus.update",
            Cell { row: 0, column: 0 },
            "2.00",
            &columns,
            &[row(1, Some("v1"), &[(3, "NORMAL"), (4, "1.00")])],
        )
        .unwrap();
        let outcomes = BTreeMap::from([(original.commands[0].operation_id, rejected())]);
        assert!(
            prepare_retry(
                &original,
                &outcomes,
                &[row(1, Some("v2"), &[(3, "KIT"), (4, "1.50")])],
                &columns,
            )
            .is_err()
        );
    }

    #[test]
    fn fill_repeats_snapshot_values_and_preserves_raw_text() {
        let columns = [column(1, EditKind::Text), column(2, EditKind::OptionalText)];
        let rows = [
            row(10, Some("v10"), &[(1, "A"), (2, "tab\tline\ntext")]),
            row(11, Some("v11"), &[(1, "B"), (2, "second")]),
            row(20, Some("v20"), &[(1, "old"), (2, "old")]),
            row(21, Some("v21"), &[(1, "old"), (2, "old")]),
            row(22, Some("v22"), &[(1, "old"), (2, "old")]),
        ];
        let preview = plan_fill(
            "products.update",
            CellRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            CellRect {
                top: 2,
                left: 0,
                bottom: 5,
                right: 2,
            },
            &columns,
            &rows,
        )
        .unwrap();

        assert_eq!(preview.commands.len(), 3);
        assert_eq!(preview.commands[0].source, RowId(20));
        assert_eq!(preview.commands[0].expected_version.as_deref(), Some("v20"));
        assert_eq!(preview.commands[0].values[&ColumnId(2)], "tab\tline\ntext");
        assert_eq!(preview.commands[1].values[&ColumnId(1)], "B");
        assert_eq!(preview.commands[2].values[&ColumnId(1)], "A");
        assert_ne!(
            preview.commands[0].operation_id,
            preview.commands[1].operation_id
        );
    }

    #[test]
    fn fill_above_uses_the_same_periodic_source_alignment() {
        let columns = [column(1, EditKind::Text)];
        let rows = [
            row(1, Some("v1"), &[(1, "old")]),
            row(2, Some("v2"), &[(1, "old")]),
            row(3, Some("v3"), &[(1, "A")]),
            row(4, Some("v4"), &[(1, "B")]),
        ];
        let preview = plan_fill(
            "products.update",
            CellRect {
                top: 2,
                left: 0,
                bottom: 4,
                right: 1,
            },
            CellRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 1,
            },
            &columns,
            &rows,
        )
        .unwrap();
        assert_eq!(preview.commands[0].values[&ColumnId(1)], "A");
        assert_eq!(preview.commands[1].values[&ColumnId(1)], "B");
    }

    #[test]
    fn fill_infers_exact_numeric_series_in_both_axes() {
        let mut vertical_columns = [column(1, EditKind::Integer)];
        vertical_columns[0].value_kind = ValueKind::Integer;
        let vertical_rows = [
            row(1, Some("v1"), &[(1, "10")]),
            row(2, Some("v2"), &[(1, "12")]),
            row(3, Some("v3"), &[(1, "0")]),
            row(4, Some("v4"), &[(1, "0")]),
        ];
        let vertical = plan_fill(
            "products.update",
            CellRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 1,
            },
            CellRect {
                top: 2,
                left: 0,
                bottom: 4,
                right: 1,
            },
            &vertical_columns,
            &vertical_rows,
        )
        .unwrap();
        assert_eq!(vertical.commands[0].values[&ColumnId(1)], "14");
        assert_eq!(vertical.commands[1].values[&ColumnId(1)], "16");

        let mut horizontal_columns = [
            column(1, EditKind::Money),
            column(2, EditKind::Money),
            column(3, EditKind::Money),
        ];
        for column in &mut horizontal_columns {
            column.value_kind = ValueKind::Money;
        }
        let horizontal_rows = [row(1, Some("v1"), &[(1, "1.20"), (2, "1.35"), (3, "0.00")])];
        let horizontal = plan_fill(
            "products.update",
            CellRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
            },
            CellRect {
                top: 0,
                left: 2,
                bottom: 1,
                right: 3,
            },
            &horizontal_columns,
            &horizontal_rows,
        )
        .unwrap();
        assert_eq!(horizontal.commands.len(), 1);
        assert_eq!(horizontal.commands[0].values[&ColumnId(3)], "1.50");
    }

    #[test]
    fn numeric_text_horizontal_fill_repeats_without_coercion() {
        let columns = [
            column(1, EditKind::Text),
            column(2, EditKind::Text),
            column(3, EditKind::Text),
        ];
        let rows = [row(1, Some("v1"), &[(1, "001"), (2, "002"), (3, "old")])];
        let preview = plan_fill(
            "products.update",
            CellRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
            },
            CellRect {
                top: 0,
                left: 2,
                bottom: 1,
                right: 3,
            },
            &columns,
            &rows,
        )
        .unwrap();
        assert_eq!(preview.commands[0].values[&ColumnId(3)], "001");
    }

    #[test]
    fn numeric_series_overflow_rejects_the_whole_preview() {
        let mut columns = [
            column(1, EditKind::Text),
            column(2, EditKind::Text),
            column(3, EditKind::Text),
        ];
        for column in &mut columns {
            column.value_kind = ValueKind::Integer;
        }
        let rows = [row(
            1,
            Some("v1"),
            &[
                (1, "170141183460469231731687303715884105726"),
                (2, "170141183460469231731687303715884105727"),
                (3, "old"),
            ],
        )];
        let error = plan_fill(
            "products.update",
            CellRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
            },
            CellRect {
                top: 0,
                left: 2,
                bottom: 1,
                right: 3,
            },
            &columns,
            &rows,
        )
        .err()
        .unwrap();
        assert_eq!(error, "Fill numeric series overflow");
    }

    #[test]
    fn fill_validates_the_whole_target_atomically() {
        let columns = [
            column(0, EditKind::OptionalText),
            column(4, EditKind::ReadOnly),
        ];
        let rows = [
            row(1, Some("v1"), &[(0, "source"), (4, "locked")]),
            row(2, Some("v2"), &[(0, "old"), (4, "locked")]),
        ];
        assert!(
            plan_fill(
                "products.update",
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 1,
                    right: 2,
                },
                CellRect {
                    top: 1,
                    left: 0,
                    bottom: 2,
                    right: 2,
                },
                &columns,
                &rows,
            )
            .is_err()
        );

        let cost = [column(4, EditKind::OptionalText)];
        assert!(
            plan_fill(
                "skus.update",
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 1,
                    right: 1,
                },
                CellRect {
                    top: 1,
                    left: 0,
                    bottom: 2,
                    right: 1,
                },
                &cost,
                &[
                    row(1, Some("v1"), &[(3, "NORMAL"), (4, "2.00")]),
                    row(2, Some("v2"), &[(3, "KIT"), (4, "1.00")]),
                ],
            )
            .is_err()
        );

        let editable = [column(1, EditKind::Text)];
        assert!(
            plan_fill(
                "products.update",
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 1,
                    right: 1,
                },
                CellRect {
                    top: 1,
                    left: 0,
                    bottom: 3,
                    right: 1,
                },
                &editable,
                &[
                    row(1, Some("v1"), &[(1, "source")]),
                    row(2, None, &[(1, "old")])
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn fill_rejects_overlap_column_mismatch_and_unloaded_bounds() {
        let columns = [column(1, EditKind::Text), column(2, EditKind::OptionalText)];
        let rows = [
            row(1, Some("v1"), &[(1, "A"), (2, "B")]),
            row(2, Some("v2"), &[(1, "C"), (2, "D")]),
        ];
        let source = CellRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 1,
        };
        assert!(
            plan_fill(
                "products.update",
                source,
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 2,
                    right: 1,
                },
                &columns,
                &rows,
            )
            .is_err()
        );
        assert!(
            plan_fill(
                "products.update",
                source,
                CellRect {
                    top: 1,
                    left: 1,
                    bottom: 2,
                    right: 2,
                },
                &columns,
                &rows,
            )
            .is_err()
        );
        assert!(
            plan_fill(
                "products.update",
                source,
                CellRect {
                    top: 1,
                    left: 0,
                    bottom: 3,
                    right: 1,
                },
                &columns,
                &rows,
            )
            .is_err()
        );
    }

    #[test]
    fn indexed_fill_uses_global_ordinals_and_versions() {
        let mut columns = [column(1, EditKind::Integer)];
        columns[0].value_kind = ValueKind::Integer;
        let source_rows = [
            row(1, Some("s1"), &[(1, "10")]),
            row(2, Some("s2"), &[(1, "12")]),
        ];
        let targets = [
            (12, row(3, Some("t1"), &[(1, "0")])),
            (13, row(4, Some("t2"), &[(1, "0")])),
        ];
        let preview = plan_indexed_fill(
            "products.update",
            CellRect {
                top: 10,
                left: 0,
                bottom: 12,
                right: 1,
            },
            &source_rows,
            CellRect {
                top: 12,
                left: 0,
                bottom: 14,
                right: 1,
            },
            &targets,
            &columns,
        )
        .unwrap();
        assert_eq!(preview.commands[0].expected_version.as_deref(), Some("t1"));
        assert_eq!(preview.commands[0].values[&ColumnId(1)], "14");
        assert_eq!(preview.commands[1].values[&ColumnId(1)], "16");
    }

    #[test]
    fn indexed_fill_rejects_duplicate_column_identity() {
        let columns = [column(1, EditKind::Integer), column(1, EditKind::Integer)];
        let source_rows = [row(1, Some("s1"), &[(1, "10")])];
        let targets = [(1, row(2, Some("t1"), &[(1, "0")]))];
        let result = plan_indexed_fill(
            "products.update",
            CellRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
            },
            &source_rows,
            CellRect {
                top: 1,
                left: 0,
                bottom: 2,
                right: 2,
            },
            &targets,
            &columns,
        );
        assert!(result.is_err());
    }

    #[test]
    fn indexed_fill_rejects_noncontiguous_targets() {
        let columns = [column(1, EditKind::Integer)];
        let source_rows = [row(1, Some("s1"), &[(1, "10")])];
        let targets = [(3, row(2, Some("t1"), &[(1, "0")]))];
        assert!(
            plan_indexed_fill(
                "products.update",
                CellRect {
                    top: 1,
                    left: 0,
                    bottom: 2,
                    right: 1
                },
                &source_rows,
                CellRect {
                    top: 2,
                    left: 0,
                    bottom: 3,
                    right: 1
                },
                &targets,
                &columns,
            )
            .is_err()
        );
    }
}
