use crate::{Column, ColumnId, RowId, money_minor};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub enum EditKind {
    #[default]
    ReadOnly,
    Text,
    OptionalText,
    Integer,
    Money,
    Date,
    DateTime,
    /// The stored value is a complete UUID, never the object's display label.
    Entity,
    Choice(Vec<String>),
}
impl EditKind {
    pub fn validate(&self, text: &str) -> Result<(), &'static str> {
        if text.len() > 4096 {
            return Err("Cell exceeds 4096 bytes");
        }
        match self {
            Self::ReadOnly => Err("Read-only column"),
            Self::Text if !text.trim().is_empty() => Ok(()),
            Self::Text => Err("Value is required"),
            Self::OptionalText => Ok(()),
            Self::Integer => text
                .parse::<i64>()
                .map(|_| ())
                .map_err(|_| "Invalid integer"),
            Self::Money => money_minor(text)?.map(|_| ()).ok_or("Amount is required"),
            Self::Date | Self::DateTime => {
                let expected = if matches!(self, Self::Date) { 10 } else { 19 };
                if text.len() != expected {
                    return Err("Invalid date format");
                }
                crate::DateTimeParts::parse(text).map(|_| ())
            }
            Self::Entity => RowId::from_uuid(text)
                .filter(|id| id.0 != 0)
                .map(|_| ())
                .ok_or("Choose a valid object"),
            Self::Choice(values) if values.iter().any(|value| value == text) => Ok(()),
            Self::Choice(_) => Err("Choose an allowed value"),
        }
    }
}
#[derive(Clone, Debug)]
pub struct CellEdit {
    pub row: RowId,
    pub column: ColumnId,
    pub before: String,
    pub after: String,
}
#[derive(Default)]
pub struct EditBuffer {
    values: BTreeMap<(RowId, ColumnId), String>,
    undo: Vec<Vec<CellEdit>>,
    redo: Vec<Vec<CellEdit>>,
}
impl EditBuffer {
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
    pub fn value(&self, row: RowId, column: ColumnId, base: impl FnOnce() -> String) -> String {
        self.values
            .get(&(row, column))
            .cloned()
            .unwrap_or_else(base)
    }
    pub fn apply(
        &mut self,
        edits: Vec<CellEdit>,
        columns: &[Column],
        base: impl Fn(RowId, ColumnId) -> String,
    ) -> Result<(), String> {
        self.apply_with_history_reset(edits, columns, base, false)
    }

    /// A new history scope takes effect only after the entire batch validates.
    pub fn apply_with_history_reset(
        &mut self,
        edits: Vec<CellEdit>,
        columns: &[Column],
        base: impl Fn(RowId, ColumnId) -> String,
        reset_history: bool,
    ) -> Result<(), String> {
        if edits.is_empty() || edits.len() > 10_000 {
            return Err("Choose 1–10,000 cells".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        let bytes: usize = edits
            .iter()
            .map(|edit| edit.before.len() + edit.after.len())
            .sum();
        if bytes > 1_048_576 {
            return Err("Change set exceeds 1 MiB".into());
        }
        for edit in &edits {
            if !seen.insert((edit.row, edit.column)) {
                return Err("Duplicate cell in change set".into());
            }
            let column = columns
                .iter()
                .find(|column| column.id == edit.column)
                .ok_or("Unknown column")?;
            column
                .edit_kind
                .validate(&edit.after)
                .map_err(|error| format!("{}: {error}", column.title))?;
            if self.value(edit.row, edit.column, || base(edit.row, edit.column)) != edit.before {
                return Err("Data changed; refresh the edit preview".into());
            }
        }
        if reset_history {
            self.clear_history();
        }
        for edit in &edits {
            self.values
                .insert((edit.row, edit.column), edit.after.clone());
        }
        self.undo.push(edits);
        while self.undo.len() > 50
            || self
                .undo
                .iter()
                .flatten()
                .map(|edit| edit.before.len() + edit.after.len())
                .sum::<usize>()
                > 4_194_304
        {
            self.undo.remove(0);
        }
        self.redo.clear();
        Ok(())
    }
    pub fn undo(&mut self) -> bool {
        let Some(edits) = self.undo.pop() else {
            return false;
        };
        for edit in &edits {
            self.values
                .insert((edit.row, edit.column), edit.before.clone());
        }
        self.redo.push(edits);
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(edits) = self.redo.pop() else {
            return false;
        };
        for edit in &edits {
            self.values
                .insert((edit.row, edit.column), edit.after.clone());
        }
        self.undo.push(edits);
        true
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

/// Spreadsheet TSV including quoted tabs/newlines and doubled quotes; bounded before parsing.
pub fn parse_tsv(text: &str) -> Result<Vec<Vec<String>>, &'static str> {
    if text.len() > 1_048_576 {
        return Err("Paste exceeds 1 MiB");
    }
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    let mut closed = false;
    let mut cells = 0;
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                cell.push(ch);
            }
        } else {
            match ch {
                '"' if cell.is_empty() && !closed => quoted = true,
                '\t' | '\n' | '\r' => {
                    row.push(std::mem::take(&mut cell));
                    cells += 1;
                    closed = false;
                    if cells > 10_000 {
                        return Err("Paste exceeds 10,000 cells");
                    }
                    if ch != '\t' {
                        if ch == '\r' && chars.peek() == Some(&'\n') {
                            chars.next();
                        }
                        rows.push(std::mem::take(&mut row));
                    }
                }
                _ if closed => return Err("Invalid quoted TSV"),
                _ => cell.push(ch),
            }
        }
    }
    if quoted {
        return Err("Unclosed TSV quote");
    }
    if !cell.is_empty() || !row.is_empty() || closed {
        row.push(cell);
        rows.push(row);
    }
    if rows.is_empty() {
        return Err("Clipboard is empty");
    }
    let width = rows[0].len();
    if rows.iter().any(|row| row.len() != width) || rows.len().saturating_mul(width) > 10_000 {
        return Err("Paste must be a rectangle of at most 10,000 cells");
    }
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audit_typed_dates_and_entity_identity_validate_before_any_write() {
        assert!(EditKind::Date.validate("2024-02-29").is_ok());
        assert!(EditKind::Date.validate("2026-02-29").is_err());
        assert!(EditKind::Date.validate("2026-01-01 00:00:00").is_err());
        assert!(EditKind::DateTime.validate("2026-01-01 23:59:59").is_ok());
        assert!(EditKind::DateTime.validate("2026-01-01 24:00:00").is_err());
        assert!(EditKind::DateTime.validate("2026-01-01").is_err());
        assert!(EditKind::Entity.validate(&RowId(99).to_uuid()).is_ok());
        assert!(EditKind::Entity.validate(&RowId(0).to_uuid()).is_err());
        assert!(EditKind::Entity.validate("SKU display name").is_err());
    }
    #[test]
    fn optional_text_can_be_cleared_without_weakening_required_text_or_size_limits() {
        assert!(EditKind::OptionalText.validate("").is_ok());
        assert!(EditKind::Text.validate("").is_err());
        assert!(EditKind::OptionalText.validate(&"x".repeat(4097)).is_err());
    }
    #[test]
    fn paste_parser_handles_spreadsheet_quotes_and_limits() {
        assert_eq!(
            parse_tsv("a\t\"b\tc\"\r\nx\t\"line\n2\"\r\n").unwrap(),
            vec![vec!["a", "b\tc"], vec!["x", "line\n2"]]
        );
        assert!(parse_tsv("a\tb\nc").is_err());
        assert!(parse_tsv("\"x").is_err());
    }
    #[test]
    fn edits_are_atomic_and_undo_redo_and_stale_versions_are_checked() {
        let mut c = Column::new(ColumnId(1), "Qty", 100.0);
        c.edit_kind = EditKind::Integer;
        let columns = vec![c];
        let mut buffer = EditBuffer::default();
        let edit = |row, after: &str| CellEdit {
            row: RowId(row),
            column: ColumnId(1),
            before: "1".into(),
            after: after.into(),
        };
        assert!(
            buffer
                .apply(vec![edit(1, "2"), edit(2, "bad")], &columns, |_, _| "1"
                    .into())
                .is_err()
        );
        assert_eq!(buffer.value(RowId(1), ColumnId(1), || "1".into()), "1");
        buffer
            .apply(vec![edit(1, "2")], &columns, |_, _| "1".into())
            .unwrap();
        assert!(
            buffer
                .apply(vec![edit(1, "3")], &columns, |_, _| "1".into())
                .is_err()
        );
        assert!(buffer.undo());
        assert_eq!(buffer.value(RowId(1), ColumnId(1), || "1".into()), "1");
        assert!(buffer.redo());
        assert_eq!(buffer.value(RowId(1), ColumnId(1), || "1".into()), "2");
    }
}
