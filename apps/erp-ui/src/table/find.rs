//! Local TableView lookup adapter. Remote tables must provide their own
//! server-side lookup; this module deliberately scans only the published local
//! query result (`result`).

use super::*;
use erp_grid_core::{Cell, FindCursor, FindDirection, FindMatch, FindRequest, FindStep};

impl TableView {
    /// Compact find popup. The host owns `find_open/query/case`; this view
    /// only mutates those fields and lets `tick` perform bounded scanning.
    pub(super) fn find_ui(&mut self, ui: &mut egui::Ui) {
        if !self.find_open {
            return;
        }
        let mut open = self.find_open;
        egui::Window::new("Find")
            .id(egui::Id::new((&self.scope, "table-find")))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                let changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.find_query)
                            .char_limit(4096)
                            .hint_text("Find in current results"),
                    )
                    .changed();
                let case_changed = ui.checkbox(&mut self.find_case, "Case sensitive").changed();
                if changed || case_changed {
                    self.find_cursor = None;
                }
                ui.add_enabled_ui(!self.busy() && !self.dirty(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Previous").clicked() {
                            self.begin_find(FindDirection::Backward);
                        }
                        if ui.button("Next").clicked() {
                            self.begin_find(FindDirection::Forward);
                        }
                    });
                });
            });
        self.find_open = open;
        if !open {
            self.find_cursor = None;
        }
        if self.find_cursor.is_some() {
            ui.ctx().request_repaint();
        }
    }

    fn begin_find(&mut self, direction: FindDirection) {
        if self.busy() || self.dirty() || self.find_query.is_empty() {
            return;
        }
        let columns: Vec<_> = self
            .columns
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.id)
            .collect();
        self.find_columns = columns.clone();
        let start = self.state.focus.and_then(|cell| {
            columns.get(cell.column).copied().map(|column| FindMatch {
                row: self.pagination.range(self.total()).start + cell.row,
                column,
            })
        });
        self.find_cursor = Some(FindCursor::new_from(
            FindRequest {
                query: self.find_query.clone(),
                case_sensitive: self.find_case,
                direction,
            },
            self.total(),
            start,
        ));
    }

    /// Scan at most 1,000 cells per tick and focus/page to the first match.
    /// This is intentionally local-only: a remote view must expose a server
    /// search contract before it can claim matches outside loaded rows.
    pub(super) fn advance_find(&mut self, ctx: &egui::Context) {
        if !self.find_open || self.find_query.is_empty() || self.busy() || self.dirty() {
            return;
        }
        let columns: Vec<_> = self
            .columns
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.id)
            .collect();
        if columns != self.find_columns {
            self.find_columns = columns.clone();
            self.find_cursor = None;
        }
        if columns.is_empty() {
            return;
        }
        let Some(mut cursor) = self.find_cursor.take() else {
            return;
        };
        let result = {
            let data = &self.data;
            let rows = self.result.as_deref();
            cursor.step(1_000, &columns, |position, column| {
                let source = rows.map_or(position, |result| result[position]);
                data.text(source, column)
            })
        };
        match result {
            FindStep::Pending => {
                self.find_cursor = Some(cursor);
                ctx.request_repaint();
            }
            FindStep::Exhausted => {
                self.notice = Some("No match in current results".into());
                self.find_cursor = None;
            }
            FindStep::Found(found) => {
                self.notice = None;
                let page_size = self.pagination.page_size();
                let page = found.row / page_size;
                if page != self.pagination.page() {
                    self.pagination.set_page(page, self.total());
                    self.generation += 1;
                    self.state.rebind_page(self.generation, true);
                }
                let local_row = found.row % page_size;
                let Some(column) = columns.iter().position(|id| *id == found.column) else {
                    self.find_cursor = None;
                    return;
                };
                let row_id = self.data.row_id(self.source(found.row));
                self.state.row_selection.active = Some(row_id);
                self.state.request_focus(Cell {
                    row: local_row,
                    column,
                });
                self.find_cursor = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_find_pages_without_checking_or_restarting_and_query_cancels_it() {
        let mut view = TableView::new("find:test", Kind::Products);
        view.set_rows(250);
        let ctx = egui::Context::default();
        for _ in 0..10 {
            view.tick(&ctx);
        }
        assert!(!view.busy());
        view.state.row_selection.set_checked(RowId(5), true);
        view.find_open = true;
        view.find_query = "SKU-00000201".into();
        view.begin_find(FindDirection::Forward);
        for _ in 0..26 {
            view.advance_find(&ctx);
        }
        assert!(view.find_cursor.is_none());
        assert_eq!(view.pagination.page(), 2);
        assert_eq!(view.state.row_selection.active, Some(RowId(201)));
        assert_eq!(
            view.state
                .row_selection
                .checked()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [RowId(5)]
        );
        let focus = view.state.focus;
        view.advance_find(&ctx);
        assert_eq!(view.state.focus, focus);
        assert!(view.find_cursor.is_none());
        view.begin_find(FindDirection::Backward);
        assert!(view.find_cursor.is_some());
        view.start_query();
        assert!(view.find_cursor.is_none());
    }
}
