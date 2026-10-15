use super::*;
use crate::{
    http::{HttpRequest, HttpTask},
    remote_view::Session,
};
use erp_grid_core::{actions::ActionCommand, remote::RemoteRow};
use std::collections::BTreeMap;

pub(super) struct Retry {
    targets: Vec<ActionCommand>,
    latest: Vec<RemoteRow>,
    loading: Option<HttpTask>,
    paused: bool,
    pub(super) staged: Option<Preview>,
    bytes: usize,
}
impl PasteReview {
    fn retry_outcomes(&self, actions: &ProductActions) -> BTreeMap<RowId, ActionState> {
        actions.outcomes()
    }
    pub(super) fn retry_tools(
        &mut self,
        ui: &mut egui::Ui,
        actions: &mut ProductActions,
        session: &Session,
        parent: Option<RowId>,
    ) {
        let outcomes = self.retry_outcomes(actions);
        let settled = self.submitted
            && self.preview.commands.iter().all(|command| {
                matches!(
                    outcomes.get(&command.operation_id),
                    Some(ActionState::Complete(_) | ActionState::Cancelled)
                )
            });
        let targets: Vec<_> = self
            .preview
            .commands
            .iter()
            .filter(|command| {
                matches!(
                    outcomes.get(&command.operation_id),
                    Some(
                        ActionState::Complete(ActionOutcome::Rejected { .. })
                            | ActionState::Cancelled
                    )
                )
            })
            .cloned()
            .collect();
        if settled
            && !targets.is_empty()
            && ui
                .add_enabled(actions.available(), egui::Button::new("读取失败项最新版本"))
                .clicked()
        {
            self.retry = Some(Retry {
                targets,
                latest: Vec::new(),
                loading: None,
                paused: false,
                staged: None,
                bytes: 0,
            });
            self.error = None;
        }
        let Some(retry) = &mut self.retry else {
            return;
        };
        if !actions.available() || !settled {
            retry.loading = None;
            retry.paused = true;
            self.error = Some("请先恢复原操作并确认账号，输入仍保留".into());
        }
        if let Some(result) = retry.loading.as_mut().and_then(HttpTask::poll) {
            retry.loading = None;
            let command = &retry.targets[retry.latest.len()];
            let decoded = result
                .map_err(|error| error.to_string())
                .and_then(|bytes| match command.action.as_str() {
                    "products.update" => crate::remote_products::decode_product(
                        session.tenant,
                        command.source,
                        &bytes,
                    ),
                    "skus.update" => parent
                        .ok_or_else(|| "SKU 父记录已变化".to_owned())
                        .and_then(|parent| {
                            crate::remote_view::details::editing::decode_one(
                                session.tenant,
                                parent,
                                command.source,
                                &bytes,
                            )
                        }),
                    _ => Err("不支持的重试操作".into()),
                });
            match decoded {
                Ok(row) => {
                    let bytes = row.cells.values().map(String::len).sum::<usize>()
                        + row.version.as_ref().map_or(0, String::len);
                    if let Some(total) = retry
                        .bytes
                        .checked_add(bytes)
                        .filter(|total| *total <= 8 * 1024 * 1024)
                    {
                        retry.bytes = total;
                        retry.latest.push(row);
                    } else {
                        retry.paused = true;
                        self.error = Some("最新数据超过 8 MiB，请缩小重试范围".into());
                    }
                }
                Err(error) => {
                    retry.paused = true;
                    self.error = Some(error);
                }
            }
        }
        if !retry.paused && retry.loading.is_none() && retry.staged.is_none() {
            if let Some(command) = retry.targets.get(retry.latest.len()) {
                let resource = if command.action == "skus.update" {
                    "skus"
                } else {
                    "products"
                };
                match session.client.start(
                    HttpRequest {
                        path: format!(
                            "/v1/tenants/{}/{}/{}",
                            session.tenant.to_uuid(),
                            resource,
                            command.source.to_uuid()
                        ),
                        query: vec![],
                        json: None,
                        bearer: Some(session.token.clone()),
                        idempotency_key: None,
                    },
                    ui.ctx(),
                ) {
                    Ok(task) => retry.loading = Some(task),
                    Err(error) => {
                        retry.paused = true;
                        self.error = Some(error.to_string());
                    }
                }
            } else {
                match crate::remote_paste::prepare_retry(
                    &self.preview,
                    &outcomes,
                    &retry.latest,
                    &self.source_columns,
                ) {
                    Ok(staged) => retry.staged = Some(staged),
                    Err(error) => {
                        retry.paused = true;
                        self.error = Some(error);
                    }
                }
            }
        }
        let mut submit = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            if retry.loading.is_some() {
                ui.spinner();
                ui.label(format!(
                    "已读取 {}/{}",
                    retry.latest.len(),
                    retry.targets.len()
                ));
            }
            if let Some(staged) = &retry.staged {
                if staged.commands.is_empty() {
                    ui.label("最新值已与输入一致，无需重试");
                }
                submit = ui
                    .add_enabled(
                        !staged.commands.is_empty() && actions.available(),
                        egui::Button::new("确认按新版本重试"),
                    )
                    .clicked();
            }
            cancel = ui.button("取消重试预览").clicked();
        });
        if submit && let Some(staged) = retry.staged.take() {
            match actions.submit_paste(staged.commands.clone()) {
                Ok(()) => {
                    self.preview = staged;
                    self.submitted = true;
                    self.retry = None;
                    self.error = None;
                    self.panel = erp_ui_egui::ChangePreview::new(
                        egui::Id::new("remote-paste-review-grid"),
                        self.preview.edits.clone(),
                        self.source_columns
                            .iter()
                            .map(|column| (column.id, column.title.clone())),
                    )
                    .ok();
                    if let Some(panel) = &mut self.panel {
                        panel.mark_submitted();
                    }
                }
                Err(error) => {
                    retry.staged = Some(staged);
                    self.error = Some(error);
                }
            }
        } else if cancel {
            self.retry = None;
            self.error = None;
        }
        if self
            .retry
            .as_ref()
            .is_some_and(|retry| retry.loading.is_some())
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}
