use super::*;

#[derive(PartialEq, Eq)]
pub(super) struct QueryScope {
    pub kind: Kind,
    pub parent: Option<RowId>,
    pub rows: usize,
    pub filters: Vec<(ColumnId, Filter)>,
}

impl TableView {
    pub(super) fn publish_result(&mut self, result: Vec<usize>) {
        let old_page = self.pagination.page();
        let old_ids: Vec<_> = self
            .visible_indices()
            .into_iter()
            .map(|row| self.data.row_id(row))
            .collect();
        let focus = self.state.focus.and_then(|cell| {
            let row = *old_ids.get(cell.row)?;
            let column = self
                .columns
                .iter()
                .filter(|c| c.visible)
                .nth(cell.column)?
                .id;
            Some((row, column))
        });
        let active = self.state.row_selection.active;
        // Scan the published query once, rather than searching all results for each checked ID.
        if !self.state.row_selection.checked().is_empty() {
            let valid: std::collections::BTreeSet<_> = result
                .iter()
                .map(|row| self.data.row_id(*row))
                .filter(|id| self.state.row_selection.checked().contains(id))
                .collect();
            let removed = self.state.row_selection.checked().len() - valid.len();
            self.state
                .row_selection
                .retain_checked(|id| valid.contains(id));
            if removed > 0 {
                self.notice = Some(format!("{removed} checked rows no longer match the query"));
            }
        }
        self.result = Some(result);
        self.pagination.set_page(old_page, self.total());
        let new_ids: Vec<_> = self
            .visible_indices()
            .into_iter()
            .map(|row| self.data.row_id(row))
            .collect();
        self.generation += 1;
        if old_ids == new_ids && !self.query_sort_changed && old_page == self.pagination.page() {
            self.state.adopt_generation(self.generation);
        } else {
            self.state
                .rebind_page(self.generation, old_page != self.pagination.page());
            self.state.row_selection.active = active.filter(|id| new_ids.contains(id));
            self.state.focus = focus.and_then(|(row, column)| {
                Some(Cell {
                    row: new_ids.iter().position(|id| *id == row)?,
                    column: self
                        .columns
                        .iter()
                        .filter(|c| c.visible)
                        .position(|c| c.id == column)?,
                })
            });
        }
        self.resume_editing();
    }
}
