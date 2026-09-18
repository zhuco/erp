use super::*;
use erp_ui_egui::EditNavigation;

impl TableView {
    fn next_editor(&self, navigation: EditNavigation) -> Option<(usize, RowId, ColumnId)> {
        let visible: Vec<_> = self.columns.iter().filter(|c| c.visible).collect();
        let column = visible.iter().position(|c| c.id == navigation.column)?;
        let page = self.pagination.range(self.total());
        let position = page
            .clone()
            .find(|&index| self.data.row_id(self.source(index)) == navigation.row)?;
        let editable = |index: usize| !matches!(visible[index].edit_kind, EditKind::ReadOnly);
        let adjacent = if navigation.backwards {
            (0..column).rev().find(|&index| editable(index))
        } else {
            (column + 1..visible.len()).find(|&index| editable(index))
        };
        let (position, column) = if let Some(next) = adjacent {
            (position, next)
        } else if navigation.backwards {
            (
                position.checked_sub(1)?,
                (0..visible.len()).rev().find(|&index| editable(index))?,
            )
        } else {
            (
                position.checked_add(1).filter(|&p| p < self.total())?,
                (0..visible.len()).find(|&index| editable(index))?,
            )
        };
        Some((
            position,
            self.data.row_id(self.source(position)),
            visible[column].id,
        ))
    }

    pub(super) fn submit_editor(
        &mut self,
        edits: Vec<CellEdit>,
        navigation: Option<EditNavigation>,
        ticket: Option<u64>,
    ) {
        if edits.is_empty() && navigation.is_none() {
            return;
        }
        let next = navigation.and_then(|n| self.next_editor(n));
        if !edits.is_empty() {
            if !self.apply(edits) {
                if let Some(ticket) = ticket {
                    self.state.resolve_edit(
                        ticket,
                        Err(self
                            .pending_error
                            .clone()
                            .unwrap_or_else(|| "Save failed".into())),
                    );
                }
                self.resume_editor = None;
                return;
            }
            if let Some(ticket) = ticket {
                self.state.resolve_edit(ticket, Ok(()));
            }
        }
        self.resume_editor = next;
        self.navigation_scan = 0;
        if self.query.is_none() && self.deadline.is_none() && self.valid {
            self.resume_editing();
        }
    }

    pub(super) fn resume_editing(&mut self) {
        let Some((hint, row, column)) = self.resume_editor else {
            return;
        };
        let position = if hint < self.total() && self.data.row_id(self.source(hint)) == row {
            Some(hint)
        } else {
            let end = (self.navigation_scan + 2000).min(self.total());
            let found = (self.navigation_scan..end)
                .find(|&index| self.data.row_id(self.source(index)) == row);
            self.navigation_scan = end;
            if found.is_none() && end < self.total() {
                return;
            }
            found
        };
        self.resume_editor = None;
        let Some(position) = position else {
            self.notice = Some("Next edit row no longer matches the query".into());
            return;
        };
        let Some(column_index) = self
            .columns
            .iter()
            .filter(|c| c.visible)
            .position(|c| c.id == column && !matches!(c.edit_kind, EditKind::ReadOnly))
        else {
            return;
        };
        let page = position / self.pagination.page_size();
        if page != self.pagination.page() {
            self.pagination.set_page(page, self.total());
            self.generation += 1;
            self.state.rebind_page(self.generation, true);
        }
        self.state.request_edit(
            Cell {
                row: position % self.pagination.page_size(),
                column: column_index,
            },
            row,
            column,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moved_edit_target_is_located_by_id_in_bounded_steps() {
        let mut view = TableView::new("navigation", Kind::Products);
        view.set_rows(10_000);
        let ctx = egui::Context::default();
        for _ in 0..10 {
            view.tick(&ctx);
        }
        assert!(view.query.is_none());
        view.resume_editor = Some((0, RowId(10_000), ColumnId(1)));
        view.resume_editing();
        assert_eq!(view.navigation_scan, 2000);
        assert!(view.busy());
        assert!(!view.state.is_editing());
        for _ in 0..4 {
            view.resume_editing();
        }
        assert!(view.resume_editor.is_none());
        assert_eq!(view.pagination.page(), 99);
        assert!(view.state.is_editing());
    }
    fn frame(ctx: &egui::Context, view: &mut TableView, events: Vec<egui::Event>) {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| view.ui(ui));
            },
        );
        out.textures_delta.clear();
    }
    fn tab(backwards: bool) -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: if backwards {
                egui::Modifiers::SHIFT
            } else {
                egui::Modifiers::NONE
            },
        }
    }
    #[test]
    fn tab_crosses_pages_skips_hidden_readonly_and_returns_without_changing_checks() {
        let ctx = egui::Context::default();
        let mut view = TableView::new("navigation", Kind::Products);
        // This scenario deliberately ends editing at Warehouse; the Lab also has an entity column.
        view.columns[7].edit_kind = erp_grid_core::EditKind::ReadOnly;
        view.set_rows(30);
        view.pagination.set_page_size(10);
        view.columns[2].visible = false;
        for _ in 0..3 {
            frame(&ctx, &mut view, vec![]);
        }
        view.state.row_selection.toggle(RowId(25));
        view.state
            .request_edit(Cell { row: 9, column: 5 }, RowId(10), ColumnId(6));
        for _ in 0..2 {
            frame(&ctx, &mut view, vec![]);
        }
        assert!(view.state.is_editing());
        frame(&ctx, &mut view, vec![tab(false)]);
        for _ in 0..2 {
            frame(&ctx, &mut view, vec![]);
        }
        assert_eq!(view.pagination.page(), 1);
        assert_eq!(view.state.row_selection.active, Some(RowId(11)));
        assert_eq!(view.state.focus, Some(Cell { row: 0, column: 1 }));
        assert!(view.state.is_editing());
        frame(&ctx, &mut view, vec![tab(true)]);
        for _ in 0..2 {
            frame(&ctx, &mut view, vec![]);
        }
        assert_eq!(view.pagination.page(), 0);
        assert_eq!(view.state.row_selection.active, Some(RowId(10)));
        assert_eq!(view.state.focus, Some(Cell { row: 9, column: 5 }));
        assert_eq!(view.state.row_selection.targets(), [RowId(25)]);
        view.state.cancel_edit();
        view.state
            .request_edit(Cell { row: 0, column: 1 }, RowId(1), ColumnId(1));
        for _ in 0..2 {
            frame(&ctx, &mut view, vec![]);
        }
        frame(&ctx, &mut view, vec![egui::Event::Text(" Updated".into())]);
        frame(&ctx, &mut view, vec![tab(false)]);
        for _ in 0..3 {
            frame(&ctx, &mut view, vec![]);
        }
        assert_eq!(view.data.text(0, ColumnId(1)), "Product 1 Updated");
        assert_eq!(
            view.state.focus,
            Some(Cell { row: 0, column: 2 }),
            "skip hidden quantity to price"
        );
        assert!(view.state.is_editing());
    }
    #[test]
    fn invalid_or_conflicting_batch_cannot_schedule_next_editor() {
        let mut view = TableView::new("navigation", Kind::Products);
        view.columns[7].edit_kind = erp_grid_core::EditKind::ReadOnly;
        view.set_rows(30);
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&ctx, &mut view, vec![]);
        }
        let navigation = EditNavigation {
            row: RowId(1),
            column: ColumnId(2),
            backwards: false,
        };
        view.submit_editor(
            vec![CellEdit {
                row: RowId(1),
                column: ColumnId(2),
                before: "stale".into(),
                after: "42".into(),
            }],
            Some(navigation),
            None,
        );
        assert!(view.resume_editor.is_none());
        assert_eq!(view.data.text(0, ColumnId(2)), "0");
        assert!(view.notice.as_deref().unwrap().contains("changed"));
        let first = EditNavigation {
            row: RowId(1),
            column: ColumnId(1),
            backwards: true,
        };
        assert!(view.next_editor(first).is_none());
        view.pagination.set_page(0, 30);
        let last = EditNavigation {
            row: RowId(30),
            column: ColumnId(6),
            backwards: false,
        };
        assert!(view.next_editor(last).is_none());
    }
}
