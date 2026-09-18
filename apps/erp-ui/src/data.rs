use erp_grid_core::*;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Products,
    Goods,
    Logs,
}
pub struct DataSet {
    pub kind: Kind,
    pub parent: Option<RowId>,
    pub rows: usize,
    pub edits: crate::shared_edits::SharedEdits,
    pub logs: Vec<String>,
}
impl DataSet {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            parent: None,
            rows: if kind == Kind::Products { 10_000 } else { 0 },
            edits: Default::default(),
            logs: Vec::new(),
        }
    }
    pub fn row_id(&self, index: usize) -> RowId {
        RowId(if self.kind == Kind::Products {
            index as u128 + 1
        } else {
            self.parent.map_or(0, |id| id.0) * 1000 + index as u128 + 1
        })
    }
    pub fn index_for_id(&self, row: RowId) -> Option<usize> {
        let index = if self.kind == Kind::Products {
            usize::try_from(row.0.checked_sub(1)?).ok()?
        } else {
            if Some(row.0 / 1000) != self.parent.map(|id| id.0) {
                return None;
            }
            usize::try_from((row.0 % 1000).checked_sub(1)?).ok()?
        };
        (index < self.rows).then_some(index)
    }
    pub fn base(&self, row: RowId, col: ColumnId) -> String {
        let Some(index) = self.index_for_id(row) else {
            return String::new();
        };
        if self.kind == Kind::Logs {
            return match col.0 {
                0 => (index + 1).to_string(),
                1 => self.logs.get(index).cloned().unwrap_or_default(),
                _ => String::new(),
            };
        }
        fixture(
            index,
            col,
            if self.kind == Kind::Goods {
                Some(row.0 / 1000)
            } else {
                None
            },
        )
    }
    pub fn text(&self, index: usize, col: ColumnId) -> String {
        let row = self.row_id(index);
        self.edits.value(row, col, || self.base(row, col))
    }
    pub fn apply(&mut self, changes: Vec<CellEdit>, columns: &[Column]) -> Result<(), String> {
        if changes
            .iter()
            .any(|edit| self.index_for_id(edit.row).is_none())
        {
            return Err("Row does not belong to the current data source".into());
        }
        self.edits
            .apply(changes, columns, |row, col| self.base(row, col))
    }
}

pub fn fixture_date(index: usize) -> DateTimeParts {
    DateTimeParts {
        year: 2026,
        month: ((index / 28) % 12 + 1) as u32,
        day: (index % 28 + 1) as u32,
        hour: (index % 24) as u32,
        minute: (index % 60) as u32,
        second: 0,
    }
}
fn fixture(index: usize, col: ColumnId, parent: Option<u128>) -> String {
    match col.0 {
        0 => parent.map_or_else(
            || format!("SKU-{:08}", index + 1),
            |parent| format!("SKU-{parent}-{:03}", index + 1),
        ),
        1 => format!(
            "{} {}",
            if parent.is_some() { "Item" } else { "Product" },
            index + 1
        ),
        2 => ((index * 17) % 1000).to_string(),
        3 => format!("{}.{:02}", 10 + index % 200, index % 100),
        4 => ["Ready", "Hold", "Closed"][index % 3].into(),
        5 => fixture_date(index).label(),
        6 => ["North", "South", "East", "West"][index % 4].into(),
        7 => RowId(100_000 + (index % 20) as u128).to_uuid(),
        _ => format!("R{} / C{}", index + 1, col.0),
    }
}
pub fn columns(kind: Kind) -> Vec<Column> {
    if kind == Kind::Logs {
        return vec![
            Column::new(ColumnId(0), "Event", 100.0),
            Column::new(ColumnId(1), "Action", 540.0),
        ];
    }
    (0..if kind == Kind::Products { 100 } else { 7 })
        .map(|id| {
            let mut column = Column::new(
                ColumnId(id),
                match id {
                    0 => "SKU".into(),
                    1 => "Product".into(),
                    2 => "Stock".into(),
                    3 => "Price".into(),
                    4 => "Status".into(),
                    5 => "Created at (UTC)".into(),
                    6 => "Warehouse".into(),
                    7 => "Related SKU".into(),
                    8 => "Open".into(),
                    _ => format!("Field {id}"),
                },
                if id == 1 || id == 5 {
                    220.0
                } else if id == 3 {
                    180.0
                } else {
                    120.0
                },
            );
            column.filter = match id {
                3 => FilterKind::Money,
                4 => FilterKind::categorical(["Ready", "Hold", "Closed"].map(String::from), true),
                5 => FilterKind::DateTime,
                6 => FilterKind::categorical(
                    ["North", "South", "East", "West"].map(String::from),
                    true,
                ),
                _ => FilterKind::Text,
            };
            column.value_kind = match id {
                2 => ValueKind::Integer,
                3 => ValueKind::Money,
                5 => ValueKind::DateTime,
                _ => ValueKind::Text,
            };
            column.edit_kind = match id {
                1 => EditKind::Text,
                2 => EditKind::Integer,
                3 => EditKind::Money,
                4 => EditKind::Choice(["Ready", "Hold", "Closed"].map(String::from).into()),
                5 => EditKind::DateTime,
                6 => EditKind::Choice(["North", "South", "East", "West"].map(String::from).into()),
                7 => EditKind::Entity,
                _ => EditKind::ReadOnly,
            };
            column
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_foreign_identity_cannot_alias_a_local_fixture_row() {
        let mut data = DataSet::new(Kind::Products);
        let id = RowId((1u128 << 96) + 1);
        assert!(data.index_for_id(id).is_none());
        assert_eq!(data.base(id, ColumnId(1)), "");
        assert!(
            data.apply(
                vec![CellEdit {
                    row: id,
                    column: ColumnId(1),
                    before: "Product 1".into(),
                    after: "Wrong row".into()
                }],
                &columns(Kind::Products)
            )
            .is_err()
        );
        assert_eq!(data.text(0, ColumnId(1)), "Product 1");
    }
}
