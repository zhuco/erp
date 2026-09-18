use super::*;
use erp_grid_core::{
    Cell, CellEdit, EditKind,
    actions::{ActionOutcome, ActionState},
};
use erp_ui_egui::EditNavigation;

pub(super) struct Editing {
    pub(super) observed: RemoteRow,
    pending: Option<(RowId, u64)>,
    rejected: bool,
    pub(super) latest: Option<RemoteRow>,
    loading: Option<HttpTask>,
}
pub(super) struct NextEditor {
    row: Option<RowId>,
    column: ColumnId,
    page: u64,
    backwards: bool,
}

impl RemoteProductView {
    pub fn dirty(&self) -> bool {
        self.product_dirty() || self.orders.dirty()
    }

    fn product_dirty(&self) -> bool {
        self.state.is_editing()
            || self.editing.is_some()
            || self.details.dirty()
            || self.paste.is_some()
            || self.cross_page_fill.is_some()
            || self.copy_batches.dirty()
            || self.copy_batches.leave_pending()
            || self.drafts.busy()
            || self.inspector.dirty()
    }

    pub(crate) fn page_dirty(&self, orders: bool) -> bool {
        if orders {
            self.orders.dirty()
        } else {
            self.product_dirty()
        }
    }

    pub(crate) fn background_tick(&mut self, ctx: &egui::Context) {
        self.tick(ctx);
    }

    pub(crate) fn discard_page(&mut self, orders: bool) -> Result<(), &'static str> {
        if orders {
            return self.orders.discard_for_leave();
        }
        if self
            .editing
            .as_ref()
            .is_some_and(|edit| edit.pending.is_some())
            || self.details.leave_pending()
            || self.copy_batches.leave_pending()
            || self
                .paste
                .as_ref()
                .is_some_and(super::paste::PasteReview::submitted)
            || self.drafts.busy()
        {
            return Err("A product operation is still pending; wait for its result.");
        }
        self.state.cancel_edit();
        self.editing = None;
        self.resume_editor = None;
        self.inspector.discard();
        self.paste = None;
        if let Some(fill) = &mut self.cross_page_fill {
            fill.cancel();
        }
        self.cross_page_fill = None;
        self.cross_page_fill_range = Default::default();
        self.copy_batches.discard_unsent();
        self.details.discard_for_leave();
        Ok(())
    }

    pub(crate) fn request_save_page(
        &mut self,
        ctx: &egui::Context,
        orders: bool,
    ) -> Result<bool, String> {
        if orders {
            let session = self
                .session
                .as_ref()
                .filter(|session| web_time::Instant::now() < session.expires)
                .ok_or("Sign in again before saving order edits")?;
            return self.orders.request_save_for_leave(session, ctx);
        }
        if self
            .editing
            .as_ref()
            .is_some_and(|edit| edit.pending.is_some())
            || self.details.leave_pending()
            || self.copy_batches.leave_pending()
            || self
                .paste
                .as_ref()
                .is_some_and(super::paste::PasteReview::submitted)
            || self.drafts.busy()
        {
            return Err("A product operation is still pending; wait for its result.".into());
        }
        if !self.product_dirty() {
            return Ok(true);
        }
        if self.paste.is_some() || self.cross_page_fill.is_some() || self.copy_batches.dirty() {
            return Err("Return to the open review and confirm it explicitly; save and leave cannot approve a multi-item operation.".into());
        }
        if self.inspector.dirty() {
            self.begin_inspector_save(self.inspector.changes());
            return Ok(!self.product_dirty());
        }
        if self.details.dirty() {
            return self.details.request_save_for_leave(&mut self.actions);
        }
        let Some(draft) = self.state.edit_draft() else {
            return Ok(!self.product_dirty());
        };
        let column = self
            .columns
            .iter()
            .find(|column| column.id == draft.column)
            .ok_or_else(|| {
                "Edited column is unavailable; return to the editor to resolve it".to_string()
            })?;
        if let Some((edit, ticket)) = self.state.prepare_edit_commit(column)? {
            self.submit_editor(vec![edit], Some(ticket), None);
        }
        Ok(!self.product_dirty())
    }

    pub(crate) fn hold_for_leave(&mut self) {
        self.state.hold_edit();
        self.details.hold_for_leave();
        self.orders.hold_for_leave();
    }

    /// A sent edit remains protected until the durable action runner returns a
    /// terminal receipt. It cannot be converted back into an unsent draft.
    pub(crate) fn leave_blocked(&self) -> Option<&'static str> {
        if let Some(reason) = self.orders.leave_blocked() {
            return Some(reason);
        }
        if self
            .editing
            .as_ref()
            .is_some_and(|editing| editing.pending.is_some())
            || self.details.leave_pending()
            || self.copy_batches.leave_pending()
            || self
                .paste
                .as_ref()
                .is_some_and(super::paste::PasteReview::submitted)
        {
            Some(
                "A remote save is still being confirmed. Wait for its result or keep editing; it cannot be discarded safely.",
            )
        } else if self.drafts.busy() {
            Some("Draft storage is still running. Wait for it to finish or keep editing.")
        } else {
            None
        }
    }

    pub(crate) fn discard_for_leave(&mut self) -> Result<(), &'static str> {
        if let Some(reason) = self.leave_blocked() {
            return Err(reason);
        }
        self.state.cancel_edit();
        self.editing = None;
        self.resume_editor = None;
        self.inspector.discard();
        self.paste = None;
        if let Some(fill) = &mut self.cross_page_fill {
            fill.cancel();
        }
        self.cross_page_fill = None;
        self.cross_page_fill_range = Default::default();
        self.copy_batches.discard_unsent();
        self.details.discard_for_leave();
        self.orders.discard_for_leave()?;
        Ok(())
    }

    pub(crate) fn request_save_for_leave(&mut self, ctx: &egui::Context) -> Result<bool, String> {
        if let Some(reason) = self.leave_blocked() {
            return Err(reason.into());
        }
        if !self.dirty() {
            return Ok(true);
        }
        if self.orders.dirty() {
            let session = self
                .session
                .as_ref()
                .filter(|session| web_time::Instant::now() < session.expires)
                .ok_or("Sign in again before saving order edits")?;
            return self.orders.request_save_for_leave(session, ctx);
        }
        if self.paste.is_some() || self.cross_page_fill.is_some() || self.copy_batches.dirty() {
            return Err("Return to the open review and confirm it explicitly; save and leave cannot approve a multi-item operation.".into());
        }
        if self.inspector.dirty() {
            self.begin_inspector_save(self.inspector.changes());
            return Ok(!self.dirty());
        }
        if self.details.dirty() {
            return self.details.request_save_for_leave(&mut self.actions);
        }
        let Some(draft) = self.state.edit_draft() else {
            return Ok(!self.dirty());
        };
        let column = self
            .columns
            .iter()
            .find(|column| column.id == draft.column)
            .ok_or_else(|| {
                "Edited column is unavailable; return to the editor to resolve it".to_string()
            })?;
        if let Some((edit, ticket)) = self.state.prepare_edit_commit(column)? {
            self.submit_editor(vec![edit], Some(ticket), None);
        }
        Ok(!self.dirty())
    }

    pub(super) fn snapshot_product_draft(
        &self,
    ) -> Result<Option<Vec<crate::edit_drafts::DraftCell>>, String> {
        if let Some(paste) = &self.paste {
            return paste
                .unsent_preview()
                .map(|preview| super::paste::draft_cells(&preview, None))
                .transpose();
        }
        let Some(draft) = self.state.edit_draft() else {
            return Ok(None);
        };
        let observed = self.editing.as_ref().ok_or("编辑原版本尚未就绪")?;
        if observed.pending.is_some() {
            return Err("已发送的操作不能作为未提交草稿暂存".into());
        }
        let version = observed.observed.version.clone().ok_or("编辑缺少原版本")?;
        if draft.row != observed.observed.id {
            return Err("编辑记录身份不匹配".into());
        }
        Ok(Some(vec![crate::edit_drafts::DraftCell {
            row: draft.row,
            column: draft.column,
            before: draft.before,
            after: draft.after,
            expected_version: version,
            parent: None,
        }]))
    }

    pub(super) fn observe_editor(&mut self) {
        if self.editing.is_none()
            && let Some(draft) = self.state.edit_draft()
            && let Some(row) = self
                .pages
                .page(&self.address)
                .and_then(|page| page.rows.iter().find(|row| row.id == draft.row))
        {
            self.editing = Some(Editing {
                observed: row.clone(),
                pending: None,
                rejected: false,
                latest: None,
                loading: None,
            });
        }
        if !self.state.is_editing()
            && self
                .editing
                .as_ref()
                .is_some_and(|edit| edit.pending.is_none())
        {
            self.editing = None;
        }
    }

    pub(super) fn submit_editor(
        &mut self,
        edits: Vec<CellEdit>,
        ticket: Option<u64>,
        navigation: Option<EditNavigation>,
    ) {
        if edits.is_empty() && navigation.is_none() {
            return;
        }
        let next = navigation.and_then(|navigation| self.next_editor(navigation));
        if edits.is_empty() {
            self.resume_editor = next;
            return;
        }
        let Some(ticket) = ticket else {
            return;
        };
        let result = (|| {
            if edits.len() != 1 {
                return Err("Submit one cell at a time".into());
            }
            let edit = &edits[0];
            let observation = self
                .editing
                .as_ref()
                .ok_or("Edit has no observed version")?;
            if observation.pending.is_some()
                || observation.observed.id != edit.row
                || observation.observed.cells.get(&edit.column) != Some(&edit.before)
            {
                return Err(
                    "Edit source changed; load the latest version and review your input".into(),
                );
            }
            let version = observation
                .observed
                .version
                .clone()
                .ok_or("Product version is missing")?;
            self.actions.update(edit, version)
        })();
        match result {
            Ok(operation) => {
                if let Some(editing) = &mut self.editing {
                    editing.pending = Some((operation, ticket));
                    editing.rejected = false;
                }
                self.resume_editor = next;
            }
            Err(error) => {
                self.state.resolve_edit(ticket, Err(error));
                self.resume_editor = None;
            }
        }
    }

    pub(super) fn finish_editor(&mut self) {
        let Some(editing) = &mut self.editing else {
            return;
        };
        let Some((operation, ticket)) = editing.pending else {
            return;
        };
        match self.actions.outcome(operation) {
            Some(ActionState::Complete(ActionOutcome::Applied { row, .. }))
                if row == editing.observed.id =>
            {
                let saved_draft = self
                    .state
                    .edit_draft()
                    .filter(|draft| draft.row == editing.observed.id);
                if let Some(draft) = saved_draft
                    && let ActionState::Complete(ActionOutcome::Applied { version, .. }) = self
                        .actions
                        .outcome(operation)
                        .unwrap_or(ActionState::Cancelled)
                {
                    self.inspector.mark_saved(
                        editing.observed.id,
                        draft.column,
                        draft.after,
                        version,
                    );
                }
                self.state.resolve_edit(ticket, Ok(()));
                self.state.message = None;
                let saved_row = editing.observed.id;
                self.editing = None;
                if let Some(next) = self.inspector.next_queued() {
                    self.begin_inspector_step(saved_row, next.0, next.1, next.2);
                }
            }
            Some(ActionState::Complete(ActionOutcome::Rejected { code, message })) => {
                self.state
                    .resolve_edit(ticket, Err(format!("{code}: {message}")));
                editing.pending = None;
                editing.rejected = true;
                self.resume_editor = None;
                self.inspector.clear_queue();
            }
            Some(ActionState::Cancelled) => {
                self.state.resolve_edit(
                    ticket,
                    Err("Save cancelled before sending; input kept".into()),
                );
                editing.pending = None;
                self.resume_editor = None;
            }
            Some(ActionState::Complete(ActionOutcome::Applied { .. })) => {
                self.state.message = Some(
                    "Save receipt returned another product; verify the original operation".into(),
                );
            }
            _ => {}
        }
    }

    pub(super) fn edit_tools(&mut self, ui: &mut egui::Ui) {
        let Some(editing) = &mut self.editing else {
            return;
        };
        let Some(session) = &self.session else {
            return;
        };
        if let Some(result) = editing.loading.as_mut().and_then(HttpTask::poll) {
            editing.loading = None;
            match result.map_err(|error| error.to_string()).and_then(|bytes| {
                products::decode_product(session.tenant, editing.observed.id, &bytes)
            }) {
                Ok(row) => editing.latest = Some(row),
                Err(error) => self.state.message = Some(error),
            }
        }
        if editing.rejected {
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(editing.loading.is_none(), egui::Button::new("Load latest version")).clicked() {
                    match session.client.start(HttpRequest {
                        path: format!("/v1/tenants/{}/products/{}", session.tenant.to_uuid(), editing.observed.id.to_uuid()),
                        query: vec![], json: None, bearer: Some(session.token.clone()),
                    }, ui.ctx()) {
                        Ok(task) => editing.loading = Some(task),
                        Err(error) => self.state.message = Some(error.to_string()),
                    }
                }
                if editing.loading.is_some() { ui.spinner(); }
                if let Some(latest) = &editing.latest && let Some(draft) = self.state.edit_draft() {
                    let value = latest.cells.get(&draft.column).cloned().unwrap_or_default();
                    ui.label(format!("Server: {value}"));
                    if ui.button("Use latest version").on_hover_text("Keep your input, adopt this version, then press Enter to save as a new operation").clicked()
                        && self.state.rebase_edit(draft.row, draft.column, value)
                    {
                        editing.observed = latest.clone();
                        editing.rejected = false;
                        self.state.message = Some("Input kept. Review it and press Enter to save.".into());
                    }
                }
            });
        }
    }

    fn next_editor(&self, navigation: EditNavigation) -> Option<NextEditor> {
        let page = self.pages.page(&self.address)?;
        let row_index = page.rows.iter().position(|row| row.id == navigation.row)?;
        let visible: Vec<_> = self
            .columns
            .iter()
            .filter(|column| column.visible)
            .collect();
        let index = visible
            .iter()
            .position(|column| column.id == navigation.column)?;
        let editable = |index: usize| !matches!(visible[index].edit_kind, EditKind::ReadOnly);
        let next = if navigation.backwards {
            (0..index).rev().find(|&index| editable(index))
        } else {
            (index + 1..visible.len()).find(|&index| editable(index))
        };
        if let Some(index) = next {
            return Some(NextEditor {
                row: Some(navigation.row),
                column: visible[index].id,
                page: self.page_number,
                backwards: navigation.backwards,
            });
        }
        let column = if navigation.backwards {
            (0..visible.len()).rev().find(|&index| editable(index))?
        } else {
            (0..visible.len()).find(|&index| editable(index))?
        };
        let next_row = if navigation.backwards {
            row_index.checked_sub(1)
        } else {
            Some(row_index + 1)
        };
        if let Some(row) = next_row.and_then(|index| page.rows.get(index)) {
            return Some(NextEditor {
                row: Some(row.id),
                column: visible[column].id,
                page: self.page_number,
                backwards: navigation.backwards,
            });
        }
        let next_page = if navigation.backwards {
            self.page_number.checked_sub(1)?
        } else {
            let RowCount::Exact(total) = page.total else {
                return None;
            };
            ((self.page_number + 1) * (self.address.limit() as u64) < total)
                .then_some(self.page_number + 1)?
        };
        Some(NextEditor {
            row: None,
            column: visible[column].id,
            page: next_page,
            backwards: navigation.backwards,
        })
    }

    pub(super) fn resume_editing(&mut self) {
        if self.state.is_editing() || self.editing.is_some() || !self.actions.available() {
            return;
        }
        let Some(next) = &self.resume_editor else {
            return;
        };
        if next.page != self.page_number {
            self.page_number = next.page;
            self.address = PageAddress::Offset {
                start: self.page_number * self.address.limit() as u64,
                limit: self.address.limit() as u16,
            };
            self.generation += 1;
            self.state.rebind_page(self.generation, true);
            return;
        }
        let Some(page) = self.pages.page(&self.address) else {
            return;
        };
        let row_index = match next.row {
            Some(id) => page.rows.iter().position(|row| row.id == id),
            None if next.backwards => page.rows.len().checked_sub(1),
            None => (!page.rows.is_empty()).then_some(0),
        };
        if let Some(index) = row_index
            && let Some(column) = self
                .columns
                .iter()
                .filter(|column| column.visible)
                .position(|column| column.id == next.column)
        {
            self.state.request_edit(
                Cell { row: index, column },
                page.rows[index].id,
                next.column,
            );
        } else {
            self.state.message =
                Some("Next edit row moved outside this page; select it again".into());
        }
        self.resume_editor = None;
    }
}
