use super::*;
use crate::remote_view::actions::ProductActions;
use erp_grid_core::{
    CellEdit,
    actions::{ActionOutcome, ActionState},
};

pub(super) struct SkuEditing {
    observed: RemoteRow,
    pending: Option<(RowId, u64)>,
    rejected: bool,
    latest: Option<RemoteRow>,
    loading: Option<HttpTask>,
}

impl Details {
    pub(in crate::remote_view) fn snapshot_current_draft(
        &self,
    ) -> Result<Option<Vec<crate::edit_drafts::DraftCell>>, String> {
        if self
            .editing
            .as_ref()
            .is_some_and(|editing| editing.pending.is_some())
        {
            return Err("当前编辑操作仍在提交".into());
        }
        if let Some(preview) = self.paste.as_ref().and_then(|paste| paste.unsent_preview()) {
            return super::paste::draft_cells(&preview, self.draft_parent()).map(Some);
        }
        let Some(draft) = self.state.edit_draft() else {
            return Ok(None);
        };
        let editing = self
            .editing
            .as_ref()
            .ok_or_else(|| "编辑草稿缺少原始行".to_string())?;
        if editing.observed.id != draft.row {
            return Err("编辑草稿行身份不匹配".into());
        }
        let expected_version = editing
            .observed
            .version
            .clone()
            .filter(|version| !version.is_empty())
            .ok_or_else(|| "编辑草稿缺少版本".to_string())?;
        Ok(Some(vec![crate::edit_drafts::DraftCell {
            row: draft.row,
            column: draft.column,
            before: draft.before.clone(),
            after: draft.after.clone(),
            expected_version,
            parent: self.draft_parent(),
        }]))
    }

    pub(in crate::remote_view) fn dirty(&self) -> bool {
        self.state.is_editing()
            || self.editing.is_some()
            || self.paste.is_some()
            || self.cross_fill.is_some()
            || self.fill_range.open
    }
    pub(in crate::remote_view) fn hold_for_leave(&mut self) {
        self.state.hold_edit();
    }
    pub(in crate::remote_view) fn leave_pending(&self) -> bool {
        self.editing
            .as_ref()
            .is_some_and(|editing| editing.pending.is_some())
            || self
                .paste
                .as_ref()
                .is_some_and(super::super::paste::PasteReview::submitted)
    }
    pub(in crate::remote_view) fn discard_for_leave(&mut self) {
        if self.leave_pending() {
            return;
        }
        self.state.cancel_edit();
        self.editing = None;
        self.paste = None;
    }
    pub(in crate::remote_view) fn request_save_for_leave(
        &mut self,
        actions: &mut ProductActions,
    ) -> Result<bool, String> {
        if self.paste.is_some() {
            return Err("请回到 SKU 批量编辑预览确认提交，不能由离开操作默认批准".into());
        }
        let Some(draft) = self.state.edit_draft() else {
            return Ok(!self.dirty());
        };
        let column = self
            .columns
            .iter()
            .find(|column| column.id == draft.column)
            .ok_or_else(|| "SKU 编辑列已不可用，请返回编辑器处理".to_string())?;
        if let Some((edit, ticket)) = self.state.prepare_edit_commit(column)? {
            self.submit_edit(vec![edit], Some(ticket), actions);
        }
        Ok(!self.dirty())
    }
    pub(super) fn observe_edit(&mut self) {
        if self.editing.is_none()
            && let Some(draft) = self.state.edit_draft()
            && let Some(row) = self
                .pages
                .page(&self.address())
                .and_then(|page| page.rows.iter().find(|row| row.id == draft.row))
        {
            self.editing = Some(SkuEditing {
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
    pub(super) fn submit_edit(
        &mut self,
        edits: Vec<CellEdit>,
        ticket: Option<u64>,
        actions: &mut ProductActions,
    ) {
        let Some(ticket) = ticket else {
            return;
        };
        if edits.is_empty() {
            return;
        }
        let result = (|| {
            if edits.len() != 1 {
                return Err("一次提交一个 SKU 单元格".into());
            }
            let edit = &edits[0];
            let observed = self.editing.as_ref().ok_or("SKU 缺少原始版本")?;
            if observed.pending.is_some()
                || observed.observed.id != edit.row
                || observed.observed.cells.get(&edit.column) != Some(&edit.before)
            {
                return Err("SKU 编辑源已变化，请读取最新版本".into());
            }
            if edit.column == ColumnId(4)
                && observed
                    .observed
                    .cells
                    .get(&ColumnId(3))
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("KIT"))
            {
                return Err("组合 SKU 成本由组件计算".into());
            }
            actions.update_sku(
                edit,
                observed.observed.version.clone().ok_or("SKU 缺少版本")?,
            )
        })();
        match result {
            Ok(operation) => {
                if let Some(edit) = &mut self.editing {
                    edit.pending = Some((operation, ticket));
                    edit.rejected = false;
                }
            }
            Err(error) => {
                self.state.resolve_edit(ticket, Err(error));
            }
        }
    }
    pub(super) fn finish_edit(&mut self, actions: &ProductActions) {
        let Some((operation, ticket)) = self.editing.as_ref().and_then(|edit| edit.pending) else {
            return;
        };
        if let Some(outcome) = actions.outcome(operation) {
            self.resolve_save(ticket, outcome);
        }
    }
    fn resolve_save(&mut self, ticket: u64, outcome: ActionState) {
        let Some(edit) = &mut self.editing else {
            return;
        };
        match outcome {
            ActionState::Complete(ActionOutcome::Applied { row, .. })
                if row == edit.observed.id =>
            {
                self.state.resolve_edit(ticket, Ok(()));
                self.editing = None;
                self.reset(false);
                self.state.message = None;
            }
            ActionState::Complete(ActionOutcome::Rejected { code, message }) => {
                self.state
                    .resolve_edit(ticket, Err(format!("{code}: {message}")));
                edit.pending = None;
                edit.rejected = true;
            }
            ActionState::Cancelled => {
                self.state
                    .resolve_edit(ticket, Err("发送前已取消，输入保留".into()));
                edit.pending = None;
            }
            ActionState::Complete(ActionOutcome::Applied { .. }) => {
                self.state.message = Some("SKU 回执身份不匹配，请核对原操作".into());
            }
            _ => {}
        }
    }
    pub(super) fn edit_tools(&mut self, ui: &mut egui::Ui, session: &Session) {
        let Some(edit) = &mut self.editing else {
            return;
        };
        if let Some(result) = edit.loading.as_mut().and_then(HttpTask::poll) {
            edit.loading = None;
            if let Some((_, parent)) = self.parent {
                match result
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| decode_one(session.tenant, parent, edit.observed.id, &bytes))
                {
                    Ok(row) => edit.latest = Some(row),
                    Err(error) => self.state.message = Some(error),
                }
            }
        }
        let mut cancel = false;
        ui.horizontal_wrapped(|ui| {
            if edit.pending.is_some() {
                ui.spinner();
            }
            cancel = ui
                .add_enabled(edit.pending.is_none(), egui::Button::new("取消 SKU 编辑"))
                .clicked();
            if edit.rejected {
                if ui
                    .add_enabled(edit.loading.is_none(), egui::Button::new("读取最新 SKU"))
                    .clicked()
                {
                    match session.client.start(
                        HttpRequest {
                            path: format!(
                                "/v1/tenants/{}/skus/{}",
                                session.tenant.to_uuid(),
                                edit.observed.id.to_uuid()
                            ),
                            query: vec![],
                            json: None,
                            bearer: Some(session.token.clone()),
                            idempotency_key: None,
                        },
                        ui.ctx(),
                    ) {
                        Ok(task) => edit.loading = Some(task),
                        Err(error) => self.state.message = Some(error.to_string()),
                    }
                }
                if let Some(latest) = &edit.latest
                    && let Some(draft) = self.state.edit_draft()
                {
                    let value = latest.cells.get(&draft.column).cloned().unwrap_or_default();
                    ui.label(format!("最新值：{value}"));
                    if ui
                        .button("采用新版本")
                        .on_hover_text("保留输入，核对后按 Enter 重新保存")
                        .clicked()
                        && self.state.rebase_edit(draft.row, draft.column, value)
                    {
                        edit.observed = latest.clone();
                        edit.rejected = false;
                        self.state.hold_edit();
                    }
                }
            }
        });
        if cancel {
            self.state.cancel_edit();
            self.editing = None;
            self.state.message = None;
        }
        if let Some(message) = &self.state.message {
            ui.label(message);
        }
    }
}

pub(in crate::remote_view) fn decode_one(
    tenant: RowId,
    parent: RowId,
    sku: RowId,
    bytes: &[u8],
) -> Result<RemoteRow, String> {
    if bytes.len() > RemotePages::MAX_BYTES {
        return Err("SKU response too large".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "Invalid SKU response")?;
    let wrapped = serde_json::to_vec(&serde_json::json!({"data":[value],"total":1}))
        .map_err(|_| "Invalid SKU response")?;
    let mut page = decode(
        tenant,
        parent,
        &wrapped,
        &PageAddress::Offset { start: 0, limit: 1 },
    )?;
    let row = page
        .rows
        .pop()
        .filter(|row| {
            row.id == sku
                && row
                    .cells
                    .get(&ColumnId(6))
                    .is_none_or(|status| status != "DELETED")
        })
        .ok_or("SKU 身份已变化或被删除")?;
    if row
        .version
        .as_ref()
        .is_none_or(|version| version.is_empty() || version.len() > 128)
    {
        return Err("SKU 版本无效".into());
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        details: &mut Details,
        events: Vec<egui::Event>,
    ) -> erp_ui_egui::GridOutput {
        let mut grid = erp_ui_egui::GridOutput::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 500.0),
                )),
                events,
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    grid = DataGrid {
                        id: egui::Id::new("sku-edit-test"),
                        columns: &mut details.columns,
                        rows: 1,
                        state: &mut details.state,
                        row_id: Some(&|_| RowId(42)),
                    }
                    .show(ui, |_, _| "Old".into());
                });
            },
        );
        output.textures_delta.clear();
        grid
    }
    #[test]
    fn sku_edit_conflict_and_parent_clear_preserve_input_until_confirmed_save() {
        let ctx = egui::Context::default();
        let mut details = Details {
            parent: Some((1, RowId(10))),
            ..Default::default()
        };
        details.state.read_only = false;
        details.state.request_edit(
            erp_grid_core::Cell { row: 0, column: 2 },
            RowId(42),
            ColumnId(2),
        );
        frame(&ctx, &mut details, vec![]);
        frame(
            &ctx,
            &mut details,
            vec![egui::Event::Text(" changed".into())],
        );
        let proposed = details.state.edit_draft().unwrap().after;
        let enter = || egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let output = frame(&ctx, &mut details, vec![enter()]);
        let ticket = output.edit_ticket.unwrap();
        details.editing = Some(SkuEditing {
            observed: RemoteRow {
                id: RowId(42),
                version: Some("v1".into()),
                cells: [(ColumnId(2), "Old".into())].into(),
            },
            pending: Some((RowId(99), ticket)),
            rejected: false,
            latest: None,
            loading: None,
        });
        details.resolve_save(
            ticket,
            ActionState::Complete(ActionOutcome::Rejected {
                code: "conflict".into(),
                message: "changed".into(),
            }),
        );
        details.clear();
        assert_eq!(details.parent, Some((1, RowId(10))));
        assert_eq!(details.state.edit_draft().unwrap().after, proposed);
        assert!(details.editing.as_ref().unwrap().rejected);
        assert!(
            details
                .state
                .rebase_edit(RowId(42), ColumnId(2), "Latest".into())
        );
        assert_eq!(details.state.edit_draft().unwrap().after, proposed);
        frame(&ctx, &mut details, vec![]);
        let output = frame(&ctx, &mut details, vec![enter()]);
        let ticket = output.edit_ticket.unwrap();
        details.editing.as_mut().unwrap().pending = Some((RowId(100), ticket));
        details.resolve_save(
            ticket,
            ActionState::Complete(ActionOutcome::Applied {
                row: RowId(42),
                version: "v2".into(),
                values: [(ColumnId(2), proposed)].into(),
            }),
        );
        assert!(!details.dirty());
        assert_eq!(details.parent, Some((1, RowId(10))));
    }
}
