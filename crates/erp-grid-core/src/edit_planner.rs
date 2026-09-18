//! Pure, bounded paste and fill planning over a host-provided row snapshot.
use crate::{Cell, CellEdit, CellRect, Column, ColumnId, RowId, ValueKind, parse_tsv};
use std::collections::{BTreeMap, BTreeSet};

const MAX_CELLS: usize = 10_000;
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug)]
pub struct IndexedRow {
    pub ordinal: usize,
    pub id: RowId,
    pub values: BTreeMap<ColumnId, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillAxis {
    Vertical,
    Horizontal,
}

enum Lane {
    Repeat(Vec<String>),
    Series { start: i128, step: i128, scale: u32 },
}

pub fn plan_paste(
    start: Cell,
    text: &str,
    columns: &[Column],
    rows: &[IndexedRow],
) -> Result<Vec<CellEdit>, String> {
    if text.len() > MAX_BYTES {
        return Err("Paste exceeds 1 MiB".into());
    }
    let pasted = parse_tsv(text).map_err(String::from)?;
    let width = pasted.first().map_or(0, Vec::len);
    bounded_cells(pasted.len(), width, "Paste")?;
    let right = start
        .column
        .checked_add(width)
        .ok_or("Paste column range overflow")?;
    if right > columns.len() {
        return Err("Paste exceeds the displayed columns".into());
    }
    unique_columns(&columns[start.column..right])?;
    if rows.len() > MAX_CELLS {
        return Err("Paste row snapshot exceeds 10,000 rows".into());
    }
    let mut row_index = BTreeMap::new();
    for row in rows {
        if row_index.insert(row.ordinal, row).is_some() {
            return Err("Paste rows have duplicate ordinals".into());
        }
    }
    let mut ids = BTreeSet::new();
    let mut edits = Vec::new();
    let mut bytes = 0usize;
    for (offset, values) in pasted.iter().enumerate() {
        let ordinal = start
            .row
            .checked_add(offset)
            .ok_or("Paste row range overflow")?;
        let row = row_index.get(&ordinal).ok_or("Paste rows are incomplete")?;
        if row.id.0 == 0 || !ids.insert(row.id) {
            return Err("Paste rows have duplicate identities or ordinals".into());
        }
        for (column, after) in columns[start.column..right].iter().zip(values) {
            column
                .edit_kind
                .validate(after)
                .map_err(|error| format!("Row {}, {}: {error}", offset + 1, column.title))?;
            let before = row
                .values
                .get(&column.id)
                .ok_or_else(|| format!("Row {}, {} is not loaded", offset + 1, column.title))?;
            add_bytes(&mut bytes, before, after)?;
            if before != after {
                edits.push(CellEdit {
                    row: row.id,
                    column: column.id,
                    before: before.clone(),
                    after: after.clone(),
                });
            }
        }
    }
    Ok(edits)
}

pub fn plan_fill(
    source: CellRect,
    source_rows: &[IndexedRow],
    target: CellRect,
    target_rows: &[IndexedRow],
    columns: &[Column],
) -> Result<Vec<CellEdit>, String> {
    let axis = fill_axis(source, target)?;
    let source_height = source
        .bottom
        .checked_sub(source.top)
        .filter(|v| *v > 0)
        .ok_or("Fill source is empty")?;
    let width = source
        .right
        .checked_sub(source.left)
        .filter(|v| *v > 0)
        .ok_or("Fill source is empty")?;
    let target_height = target
        .bottom
        .checked_sub(target.top)
        .filter(|v| *v > 0)
        .ok_or("Fill target is empty")?;
    bounded_cells(source_height, width, "Fill source")?;
    bounded_cells(target_height, target.right - target.left, "Fill target")?;
    if source.right > columns.len()
        || target.right > columns.len()
        || source_rows.len() != source_height
        || target_rows.len() != target_height
    {
        return Err("Fill rows or columns are incomplete".into());
    }
    unique_columns(&columns[source.left..source.right])?;
    unique_columns(&columns[target.left..target.right])?;
    validate_rows(source_rows, source.top)?;
    validate_rows(target_rows, target.top)?;
    if axis == FillAxis::Vertical {
        let mut ids = BTreeSet::new();
        if source_rows
            .iter()
            .chain(target_rows)
            .any(|row| !ids.insert(row.id))
        {
            return Err("Fill rows contain duplicate identities".into());
        }
    } else if source_rows
        .iter()
        .zip(target_rows)
        .any(|(source, target)| source.id != target.id || source.ordinal != target.ordinal)
    {
        return Err("Horizontal fill source and target row identities differ".into());
    }
    let mut bytes = 0usize;
    for row in source_rows {
        for column in &columns[source.left..source.right] {
            let value = row
                .values
                .get(&column.id)
                .ok_or("Fill source cell is not loaded")?;
            add_bytes(&mut bytes, value, "")?;
        }
    }
    let source_values = source_rows
        .iter()
        .map(|row| {
            columns[source.left..source.right]
                .iter()
                .map(|column| {
                    row.values
                        .get(&column.id)
                        .cloned()
                        .ok_or("Fill source cell is not loaded")
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let lanes = lanes(&source_values, source, target, axis, columns);
    let mut edits = Vec::new();
    for row in target_rows {
        for (column_index, column) in columns
            .iter()
            .enumerate()
            .take(target.right)
            .skip(target.left)
        {
            let after = lane_value(&lanes, axis, source, row.ordinal, column_index)?;
            column
                .edit_kind
                .validate(&after)
                .map_err(|error| format!("Row {}, {}: {error}", row.ordinal + 1, column.title))?;
            let before = row
                .values
                .get(&column.id)
                .ok_or("Fill target cell is not loaded")?;
            add_bytes(&mut bytes, before, &after)?;
            if before != &after {
                edits.push(CellEdit {
                    row: row.id,
                    column: column.id,
                    before: before.clone(),
                    after,
                });
            }
        }
    }
    Ok(edits)
}

pub fn fill_axis(source: CellRect, target: CellRect) -> Result<FillAxis, String> {
    if source.top >= source.bottom
        || source.left >= source.right
        || target.top >= target.bottom
        || target.left >= target.right
    {
        return Err("Fill rectangles are empty or reversed".into());
    }
    let vertical = source.left == target.left
        && source.right == target.right
        && (target.bottom <= source.top || target.top >= source.bottom);
    let horizontal = source.top == target.top
        && source.bottom == target.bottom
        && (target.right <= source.left || target.left >= source.right);
    match (vertical, horizontal) {
        (true, false) => Ok(FillAxis::Vertical),
        (false, true) => Ok(FillAxis::Horizontal),
        _ => Err("Fill target must extend from one source edge along one axis".into()),
    }
}

fn lanes(
    values: &[Vec<String>],
    source: CellRect,
    target: CellRect,
    axis: FillAxis,
    columns: &[Column],
) -> Vec<Lane> {
    let numeric =
        |column: &Column| matches!(column.value_kind, ValueKind::Integer | ValueKind::Money);
    match axis {
        FillAxis::Vertical => (0..source.right - source.left)
            .map(|offset| {
                lane(
                    values.iter().map(|row| row[offset].clone()).collect(),
                    numeric(&columns[source.left + offset]),
                )
            })
            .collect(),
        FillAxis::Horizontal => {
            let typed = columns[source.left..source.right]
                .iter()
                .chain(&columns[target.left..target.right])
                .all(numeric);
            values
                .iter()
                .cloned()
                .map(|values| lane(values, typed))
                .collect()
        }
    }
}

fn lane(values: Vec<String>, numeric: bool) -> Lane {
    if numeric && let Some((start, step, scale)) = series(&values) {
        Lane::Series { start, step, scale }
    } else {
        Lane::Repeat(values)
    }
}

fn lane_value(
    lanes: &[Lane],
    axis: FillAxis,
    source: CellRect,
    row: usize,
    column: usize,
) -> Result<String, String> {
    let (lane, position) = match axis {
        FillAxis::Vertical => (
            &lanes[column - source.left],
            row as i128 - source.top as i128,
        ),
        FillAxis::Horizontal => (
            &lanes[row - source.top],
            column as i128 - source.left as i128,
        ),
    };
    match lane {
        Lane::Repeat(values) => {
            let len = i128::try_from(values.len()).map_err(|_| "Fill series is too large")?;
            Ok(values[usize::try_from(position.rem_euclid(len))
                .map_err(|_| "Fill position overflow")?]
            .clone())
        }
        Lane::Series { start, step, scale } => format_decimal(
            step.checked_mul(position)
                .and_then(|v| start.checked_add(v))
                .ok_or("Fill numeric series overflow")?,
            *scale,
        ),
    }
}

fn decimal(value: &str) -> Option<(i128, u32)> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('+') {
        return None;
    }
    let (negative, value) = value
        .strip_prefix('-')
        .map_or((false, value), |v| (true, v));
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|v| v.is_ascii_digit())
        || !fraction.bytes().all(|v| v.is_ascii_digit())
        || fraction.len() > 18
    {
        return None;
    }
    let scale = fraction.len() as u32;
    let factor = 10_i128.checked_pow(scale)?;
    let value = whole
        .parse::<i128>()
        .ok()?
        .checked_mul(factor)?
        .checked_add(if fraction.is_empty() {
            0
        } else {
            fraction.parse().ok()?
        })?;
    Some((
        if negative {
            value.checked_neg()?
        } else {
            value
        },
        scale,
    ))
}

fn series(values: &[String]) -> Option<(i128, i128, u32)> {
    if values.len() < 2 {
        return None;
    }
    let parsed = values
        .iter()
        .map(|value| decimal(value))
        .collect::<Option<Vec<_>>>()?;
    let scale = parsed.iter().map(|(_, scale)| *scale).max()?;
    let scaled = parsed
        .iter()
        .map(|(value, own)| value.checked_mul(10_i128.checked_pow(scale - own)?))
        .collect::<Option<Vec<_>>>()?;
    let step = scaled[1].checked_sub(scaled[0])?;
    scaled
        .windows(2)
        .all(|pair| pair[1].checked_sub(pair[0]) == Some(step))
        .then_some((scaled[0], step, scale))
}

fn format_decimal(value: i128, scale: u32) -> Result<String, String> {
    if scale == 0 {
        return Ok(value.to_string());
    }
    let factor = 10_i128
        .checked_pow(scale)
        .ok_or("Fill numeric scale overflow")? as u128;
    let magnitude = value.unsigned_abs();
    Ok(format!(
        "{}{}.{:0width$}",
        if value < 0 { "-" } else { "" },
        magnitude / factor,
        magnitude % factor,
        width = scale as usize
    ))
}

fn bounded_cells(rows: usize, columns: usize, label: &str) -> Result<(), String> {
    if rows == 0
        || columns == 0
        || rows
            .checked_mul(columns)
            .is_none_or(|count| count > MAX_CELLS)
    {
        Err(format!("{label} must contain 1–10,000 cells"))
    } else {
        Ok(())
    }
}
fn unique_columns(columns: &[Column]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    if columns.iter().any(|column| !ids.insert(column.id)) {
        Err("Displayed columns contain duplicate identities".into())
    } else {
        Ok(())
    }
}
fn validate_rows(rows: &[IndexedRow], start: usize) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    if rows
        .iter()
        .enumerate()
        .any(|(offset, row)| row.ordinal != start + offset || row.id.0 == 0 || !ids.insert(row.id))
    {
        Err("Rows are not contiguous or have duplicate identities".into())
    } else {
        Ok(())
    }
}
fn add_bytes(bytes: &mut usize, before: &str, after: &str) -> Result<(), String> {
    *bytes = bytes
        .checked_add(before.len())
        .and_then(|v| v.checked_add(after.len()))
        .filter(|v| *v <= MAX_BYTES)
        .ok_or("Change set exceeds 1 MiB")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn column(id: u64, kind: ValueKind) -> Column {
        let mut c = Column::new(ColumnId(id), "Value", 100.0);
        c.edit_kind = if matches!(kind, ValueKind::Text) {
            crate::EditKind::Text
        } else {
            crate::EditKind::Integer
        };
        c.value_kind = kind;
        c
    }
    fn row(ordinal: usize, id: u128, values: &[(u64, &str)]) -> IndexedRow {
        IndexedRow {
            ordinal,
            id: RowId(id),
            values: values
                .iter()
                .map(|(c, v)| (ColumnId(*c), (*v).into()))
                .collect(),
        }
    }
    #[test]
    fn paste_is_atomic_and_bounded() {
        let columns = [column(1, ValueKind::Text)];
        let rows = [row(5, 1, &[(1, "old")])];
        assert_eq!(
            plan_paste(Cell { row: 5, column: 0 }, "new", &columns, &rows).unwrap()[0].after,
            "new"
        );
        assert!(plan_paste(Cell { row: 5, column: 0 }, "", &columns, &rows).is_err());
    }
    #[test]
    fn vertical_series_uses_global_ordinals_and_text_repeats() {
        let columns = [column(1, ValueKind::Integer)];
        let source = [row(10, 1, &[(1, "10")]), row(11, 2, &[(1, "12")])];
        let targets = [row(20, 3, &[(1, "0")]), row(21, 4, &[(1, "0")])];
        let edits = plan_fill(
            CellRect {
                top: 10,
                left: 0,
                bottom: 12,
                right: 1,
            },
            &source,
            CellRect {
                top: 20,
                left: 0,
                bottom: 22,
                right: 1,
            },
            &targets,
            &columns,
        )
        .unwrap();
        assert_eq!(
            edits.iter().map(|e| e.after.as_str()).collect::<Vec<_>>(),
            ["30", "32"]
        );
    }
    #[test]
    fn horizontal_series_uses_the_same_stable_row() {
        let columns = [
            column(1, ValueKind::Integer),
            column(2, ValueKind::Integer),
            column(3, ValueKind::Integer),
        ];
        let source = [row(0, 1, &[(1, "1"), (2, "3")])];
        let target = [row(0, 1, &[(3, "0")])];
        assert_eq!(
            plan_fill(
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 1,
                    right: 2
                },
                &source,
                CellRect {
                    top: 0,
                    left: 2,
                    bottom: 1,
                    right: 3
                },
                &target,
                &columns
            )
            .unwrap()[0]
                .after,
            "5"
        );
    }
}
