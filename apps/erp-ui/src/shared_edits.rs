use erp_grid_core::{CellEdit, Column, ColumnId, EditBuffer, RowId};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// One committed data source for bottom and inline detail views; history belongs
/// to the last edited parent, so undo never silently operates on another parent.
#[derive(Clone, Default)]
pub struct SharedEdits {
    inner: Rc<RefCell<(EditBuffer, Option<RowId>)>>,
    revision: Rc<Cell<u64>>,
    pub scope: Option<RowId>,
}

impl SharedEdits {
    pub fn revision(&self) -> u64 {
        self.revision.get()
    }
    pub fn value(&self, row: RowId, col: ColumnId, base: impl FnOnce() -> String) -> String {
        self.inner.borrow().0.value(row, col, base)
    }
    pub fn clear_history(&self) {
        self.inner.borrow_mut().0.clear_history();
    }
    pub fn can_undo(&self) -> bool {
        let data = self.inner.borrow();
        data.1 == self.scope && data.0.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        let data = self.inner.borrow();
        data.1 == self.scope && data.0.can_redo()
    }
    pub fn undo(&self) -> bool {
        self.history(false)
    }
    pub fn redo(&self) -> bool {
        self.history(true)
    }
    fn history(&self, redo: bool) -> bool {
        let mut data = self.inner.borrow_mut();
        if data.1 != self.scope {
            return false;
        }
        let changed = if redo { data.0.redo() } else { data.0.undo() };
        if changed {
            self.revision.set(self.revision.get().wrapping_add(1));
        }
        changed
    }
    pub fn apply(
        &self,
        edits: Vec<CellEdit>,
        columns: &[Column],
        base: impl Fn(RowId, ColumnId) -> String,
    ) -> Result<(), String> {
        let mut data = self.inner.borrow_mut();
        let reset_history = data.1 != self.scope;
        data.0
            .apply_with_history_reset(edits, columns, base, reset_history)?;
        data.1 = self.scope;
        self.revision.set(self.revision.get().wrapping_add(1));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_scope_change_preserves_values_history_owner_and_revision() {
        let first = SharedEdits {
            scope: Some(RowId(1)),
            ..Default::default()
        };
        let second = SharedEdits {
            scope: Some(RowId(2)),
            ..first.clone()
        };
        let mut column = Column::new(ColumnId(1), "Quantity", 100.0);
        column.edit_kind = erp_grid_core::EditKind::Integer;
        let columns = [column];
        let edit = |row, before: &str, after: &str| CellEdit {
            row: RowId(row),
            column: ColumnId(1),
            before: before.into(),
            after: after.into(),
        };
        first
            .apply(vec![edit(1, "0", "1")], &columns, |_, _| "0".into())
            .unwrap();
        first
            .apply(vec![edit(1, "1", "2")], &columns, |_, _| "0".into())
            .unwrap();
        assert!(first.undo());
        let revision = first.revision();
        for changes in [
            vec![edit(2, "0", "3"), edit(3, "0", "bad")],
            vec![edit(2, "stale", "3")],
        ] {
            assert!(second.apply(changes, &columns, |_, _| "0".into()).is_err());
            assert_eq!(first.revision(), revision);
            assert!(first.can_undo());
            assert!(first.can_redo());
            assert!(!second.can_undo());
            assert_eq!(second.value(RowId(2), ColumnId(1), || "0".into()), "0");
        }
        assert!(first.redo());
        assert_eq!(first.value(RowId(1), ColumnId(1), || "0".into()), "2");
        second
            .apply(vec![edit(2, "0", "3")], &columns, |_, _| "0".into())
            .unwrap();
        assert!(!first.can_undo());
        assert!(second.undo());
        assert_eq!(first.value(RowId(1), ColumnId(1), || "0".into()), "2");
    }
}
