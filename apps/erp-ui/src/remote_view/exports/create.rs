use super::*;
use erp_grid_core::{SortRule, remote::PageRequest};
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct Create {
    body: Option<Vec<u8>>,
    id: Option<RowId>,
    task: Option<HttpTask>,
    feedback: Option<String>,
}

impl Create {
    pub fn pending(&self) -> bool {
        self.body.is_some()
    }

    pub fn begin(&mut self, session: &Session, request: &PageRequest, ctx: &egui::Context) {
        if self.pending() {
            return;
        }
        let result =
            crate::remote_products::encode_query(session.tenant, request).and_then(|encoded| {
                serde_json::from_slice::<serde_json::Value>(&encoded.body)
                    .map_err(|_| "无法编码导出查询".into())
            });
        match result {
            Ok(query) => {
                let id = uuid::Uuid::new_v4();
                let body = serde_json::json!({"type":"PRODUCT_EXPORT", "params":{"request_id":id,"query":query}});
                match serde_json::to_vec(&body) {
                    Ok(body) => {
                        self.body = Some(body);
                        self.id = Some(RowId(id.as_u128()));
                        self.send(session, ctx);
                    }
                    Err(_) => self.feedback = Some("无法编码导出任务".into()),
                }
            }
            Err(error) => self.feedback = Some(error),
        }
    }

    pub fn begin_frozen(
        &mut self,
        session: &Session,
        token: RowId,
        excluded: &BTreeSet<RowId>,
        sorts: &[SortRule],
        ctx: &egui::Context,
    ) {
        if self.pending() {
            return;
        }
        let id = uuid::Uuid::new_v4();
        let excluded: Vec<_> = excluded.iter().map(|row| row.to_uuid()).collect();
        let body = serde_json::json!({
            "type": "PRODUCT_EXPORT",
            "params": {
                "request_id": id,
                "selection": {
                    "token": token.to_uuid(),
                    "excluded": excluded,
                    "sorts": sorts,
                }
            }
        });
        match serde_json::to_vec(&body) {
            Ok(body) => {
                self.body = Some(body);
                self.id = Some(RowId(id.as_u128()));
                self.send(session, ctx);
            }
            Err(_) => self.feedback = Some("无法编码冻结选择导出任务".into()),
        }
    }

    pub fn begin_checked(
        &mut self,
        session: &Session,
        checked: &BTreeSet<RowId>,
        sorts: &[SortRule],
        ctx: &egui::Context,
    ) {
        if self.pending() {
            return;
        }
        if checked.is_empty() || checked.len() > 100_000 || checked.iter().any(|row| row.0 == 0) {
            self.feedback = Some("勾选导出需要 1 至 100000 个有效行标识".into());
            return;
        }
        let id = uuid::Uuid::new_v4();
        let ids: Vec<_> = checked.iter().map(|row| row.to_uuid()).collect();
        let body = serde_json::json!({
            "type": "PRODUCT_EXPORT",
            "params": {
                "request_id": id,
                "checked": {
                    "ids": ids,
                    "sorts": sorts,
                }
            }
        });
        match serde_json::to_vec(&body) {
            Ok(body) => {
                self.body = Some(body);
                self.id = Some(RowId(id.as_u128()));
                self.send(session, ctx);
            }
            Err(_) => self.feedback = Some("无法编码勾选导出任务".into()),
        }
    }

    fn send(&mut self, session: &Session, ctx: &egui::Context) {
        let Some(body) = self.body.clone() else {
            return;
        };
        match session.client.start(
            HttpRequest {
                path: "/exports/jobs".into(),
                query: vec![],
                json: Some(body),
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            ctx,
        ) {
            Ok(task) => {
                self.task = Some(task);
                self.feedback = Some("正在创建导出任务".into());
            }
            Err(error) => self.feedback = Some(format!("{error}；保留原请求，可重试")),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, session: &Session) -> (bool, bool) {
        let mut completed = false;
        let mut expired = false;
        if let Some(result) = self.task.as_mut().and_then(HttpTask::poll) {
            self.task = None;
            match result {
                Ok(bytes) => {
                    let valid = serde_json::from_slice::<WireJob>(&bytes)
                        .ok()
                        .is_some_and(|job| {
                            RowId::from_uuid(&job.job_id) == self.id
                                && RowId::from_uuid(&job.tenant_id) == Some(session.tenant)
                                && job.job_type == "PRODUCT_EXPORT"
                        });
                    if valid {
                        self.body = None;
                        completed = true;
                        self.feedback = Some("导出任务已创建".into());
                    } else {
                        self.feedback = Some("回执身份无效；保留原请求，可重试确认".into());
                    }
                }
                Err(HttpError::Status(401 | 403)) => expired = true,
                Err(HttpError::Status(410)) => {
                    self.body = None;
                    self.feedback =
                        Some("冻结选择已过期或不可用，请重新选择全部匹配后创建导出".into());
                }
                Err(HttpError::Status(code @ (400 | 409 | 422))) => {
                    self.body = None;
                    self.feedback = Some(format!(
                        "导出请求被拒绝（HTTP {code}），请调整条件后重新创建"
                    ));
                }
                Err(error) => self.feedback = Some(format!("{error}；保留原请求，可重试确认")),
            }
        }
        ui.horizontal(|ui| {
            if self.task.is_some() {
                ui.spinner();
            }
            if self.pending()
                && self.task.is_none()
                && ui
                    .button("重试确认导出任务")
                    .on_hover_text("复用原请求标识，不重复创建任务")
                    .clicked()
            {
                self.send(session, ui.ctx());
            }
            if let Some(text) = &self.feedback {
                ui.label(text);
            }
        });
        if self.task.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        (completed, expired)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn frozen_export_sends_selection_and_reuses_request_after_unknown_result() {
        let tenant = RowId(1);
        let token = RowId(9);
        let excluded = BTreeSet::from([RowId(11), RowId(10)]);
        let (client, server) = crate::remote_actions::tests::server(vec![None, None]);
        let session = Session {
            client,
            tenant,
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let mut create = Create::default();
        create.begin_frozen(
            &session,
            token,
            &excluded,
            &[SortRule {
                column: ColumnId(2),
                descending: true,
            }],
            &ctx,
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while create
            .task
            .as_mut()
            .is_some_and(|task| task.poll().is_none())
        {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(3));
        }
        create.task = None;
        let original = create.body.clone().expect("unknown result retains body");
        create.send(&session, &ctx);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        let bodies: Vec<serde_json::Value> = requests
            .iter()
            .map(|request| serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap())
            .collect();
        assert_eq!(bodies[0], bodies[1]);
        assert_eq!(serde_json::to_vec(&bodies[0]).unwrap(), original);
        assert_eq!(bodies[0]["type"], "PRODUCT_EXPORT");
        assert!(bodies[0]["params"].get("query").is_none());
        assert_eq!(bodies[0]["params"]["selection"]["token"], token.to_uuid());
        assert_eq!(
            bodies[0]["params"]["selection"]["excluded"],
            serde_json::json!([RowId(10).to_uuid(), RowId(11).to_uuid()])
        );
        assert_eq!(bodies[0]["params"]["selection"]["sorts"][0]["column"], 2);
        assert_eq!(
            bodies[0]["params"]["selection"]["sorts"][0]["descending"],
            true
        );
    }

    #[test]
    fn frozen_export_drops_request_after_expired_selection() {
        let tenant = RowId(1);
        let (client, server) =
            crate::remote_actions::tests::server(vec![Some((410, "Gone".into()))]);
        let session = Session {
            client,
            tenant,
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let mut create = Create::default();
        create.begin_frozen(&session, RowId(9), &BTreeSet::new(), &[], &ctx);
        let deadline = Instant::now() + Duration::from_secs(5);
        while create.pending() {
            assert!(Instant::now() < deadline);
            let mut output = ctx.run_ui(egui::RawInput::default(), |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    let (_, expired) = create.ui(ui, &session);
                    assert!(!expired);
                });
            });
            output.platform_output.commands.clear();
            output.textures_delta.clear();
            if create.pending() {
                std::thread::sleep(Duration::from_millis(3));
            }
        }
        assert!(create.task.is_none());
        assert!(create.body.is_none());
        assert!(
            create
                .feedback
                .as_deref()
                .is_some_and(|text| text.contains("重新选择全部匹配"))
        );
        assert_eq!(server.join().unwrap().len(), 1);
    }

    #[test]
    fn checked_export_sends_every_cross_page_id_exclusively_and_reuses_request() {
        let tenant = RowId(1);
        let checked = BTreeSet::from([RowId(31), RowId(11), RowId(21)]);
        let (client, server) = crate::remote_actions::tests::server(vec![None, None]);
        let session = Session {
            client,
            tenant,
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        let mut create = Create::default();
        create.begin_checked(
            &session,
            &checked,
            &[SortRule {
                column: ColumnId(4),
                descending: false,
            }],
            &ctx,
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while create
            .task
            .as_mut()
            .is_some_and(|task| task.poll().is_none())
        {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(3));
        }
        create.task = None;
        let original = create.body.clone().expect("unknown result retains body");
        create.send(&session, &ctx);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        let bodies: Vec<serde_json::Value> = requests
            .iter()
            .map(|request| serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap())
            .collect();
        assert_eq!(bodies[0], bodies[1]);
        assert_eq!(serde_json::to_vec(&bodies[0]).unwrap(), original);
        assert_eq!(bodies[0]["type"], "PRODUCT_EXPORT");
        assert!(bodies[0]["params"].get("query").is_none());
        assert!(bodies[0]["params"].get("selection").is_none());
        assert_eq!(
            bodies[0]["params"]["checked"]["ids"],
            serde_json::json!([
                RowId(11).to_uuid(),
                RowId(21).to_uuid(),
                RowId(31).to_uuid()
            ])
        );
        assert_eq!(bodies[0]["params"]["checked"]["sorts"][0]["column"], 4);
        assert_eq!(
            bodies[0]["params"]["checked"]["sorts"][0]["descending"],
            false
        );
    }
}
