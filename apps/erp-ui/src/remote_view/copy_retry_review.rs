//! A bounded, read-only preview before creating a new copy job.
use super::{Session, wire};
use crate::{
    http::{HttpRequest, HttpTask},
    remote_selection::{self, Credential},
};
use eframe::egui;
use erp_grid_core::{
    Column, ColumnId, RowId,
    remote::{DataScope, PageAddress, PageRequest, RemoteQuery, RemoteRow},
};
use erp_ui_egui::{DataGrid, GridState};

enum Pending {
    Review(HttpTask),
    Page(PageRequest, HttpTask),
}

pub(super) struct Review {
    batch: RowId,
    request: RowId,
    credential: Option<Credential>,
    pending: Option<Pending>,
    rows: Vec<RemoteRow>,
    offset: u64,
    loaded: bool,
    error: Option<String>,
    state: GridState,
    columns: Vec<Column>,
}
impl Review {
    pub(super) fn new(batch: RowId) -> Self {
        Self {
            batch,
            request: RowId(uuid::Uuid::new_v4().as_u128()),
            credential: None,
            pending: None,
            rows: Vec::new(),
            offset: 0,
            loaded: false,
            error: None,
            state: GridState::default(),
            columns: [
                "编码",
                "名称",
                "品牌",
                "分类",
                "状态",
                "创建时间",
                "更新时间",
            ]
            .into_iter()
            .enumerate()
            .map(|(i, title)| Column::new(ColumnId(i as u64), title, 150.0))
            .collect(),
        }
    }
    fn start(&mut self, session: &Session, ctx: &egui::Context) {
        if self.pending.is_some() {
            return;
        }
        let result = wire::encode_retry_review(session.tenant, self.batch, self.request).and_then(
            |mut request| {
                request.bearer = Some(session.token.clone());
                session
                    .client
                    .start(request, ctx)
                    .map_err(|e| e.to_string())
            },
        );
        match result {
            Ok(task) => {
                self.pending = Some(Pending::Review(task));
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn page(&mut self, session: &Session, ctx: &egui::Context, offset: u64) {
        if self.pending.is_some() {
            return;
        }
        let Some(credential) = &self.credential else {
            return;
        };
        let request = PageRequest {
            ticket: 1,
            generation: 1,
            address: PageAddress::Offset {
                start: offset,
                limit: 100,
            },
            query: RemoteQuery {
                scope: DataScope {
                    source: format!("products:{}", session.tenant.to_uuid()),
                    schema: 1,
                    authorization: session.tenant.to_uuid(),
                },
                expression: None,
                filters: Default::default(),
                sorts: Vec::new(),
                search: None,
            },
        };
        let result = remote_selection::encode_page(session.tenant, credential, offset, 100, &[])
            .and_then(|encoded| {
                session
                    .client
                    .start(
                        HttpRequest {
                            path: encoded.path,
                            query: vec![],
                            json: Some(encoded.body),
                            bearer: Some(session.token.clone()),
                            idempotency_key: None,
                        },
                        ctx,
                    )
                    .map_err(|e| e.to_string())
            });
        match result {
            Ok(task) => {
                self.pending = Some(Pending::Page(request, task));
                self.rows.clear();
                self.loaded = false;
                self.offset = offset;
                self.error = None;
                self.state.clear_selection();
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn poll(&mut self, session: &Session, ctx: &egui::Context) {
        let result = self.pending.as_mut().and_then(|job| match job {
            Pending::Review(task) | Pending::Page(_, task) => task.poll(),
        });
        let Some(result) = result else {
            return;
        };
        let Some(job) = self.pending.take() else {
            return;
        };
        let result = result
            .map_err(|e| e.to_string())
            .and_then(|bytes| match job {
                Pending::Review(_) => wire::decode_retry_review(
                    &bytes,
                    self.batch,
                    self.request,
                    super::super::unix_seconds(),
                )
                .map(|review| {
                    self.credential = Some(review.selection);
                }),
                Pending::Page(request, _) => {
                    let credential = self.credential.as_ref().ok_or("审核快照不可用")?;
                    remote_selection::decode_page(
                        session.tenant,
                        credential,
                        &request,
                        &bytes,
                        super::super::unix_seconds(),
                    )
                    .map(|page| {
                        self.rows = page.rows;
                        self.loaded = true;
                    })
                }
            });
        match result {
            Ok(()) => {
                self.error = None;
                if !self.loaded {
                    self.page(session, ctx, self.offset);
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
    /// Closing only drops a preview. A submitted job is owned by the parent controller.
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &Session,
        can_write: bool,
    ) -> (bool, Option<wire::StartBody>) {
        self.poll(session, ui.ctx());
        let mut close = false;
        let mut prepared = None;
        egui::Window::new("复制失败项重新审核")
            .id(ui.id().with("copy-retry-review"))
            .collapsible(false)
            .default_size(egui::vec2(980.0, 370.0))
            .show(ui.ctx(), |ui| {
                let valid = self
                    .credential
                    .as_ref()
                    .is_some_and(|c| c.expires_at > super::super::unix_seconds());
                ui.horizontal(|ui| {
                    if self.credential.is_none()
                        && ui
                            .add_enabled(
                                can_write && self.pending.is_none(),
                                egui::Button::new("读取失败项最新版本"),
                            )
                            .on_hover_text("仅包含原任务已拒绝项；重复读取使用同一个请求身份")
                            .clicked()
                    {
                        self.start(session, ui.ctx());
                    }
                    if let Some(credential) = &self.credential {
                        ui.label(format!("失败项 {} 个", credential.total));
                        if ui
                            .add_enabled(
                                can_write && valid && self.loaded && self.pending.is_none(),
                                egui::Button::new("确认按新版本复制"),
                            )
                            .clicked()
                        {
                            prepared = Some(wire::StartBody {
                                batch_id: RowId(uuid::Uuid::new_v4().as_u128()),
                                token: credential.token,
                                excluded: vec![],
                                confirmed_count: credential.total,
                            });
                            close = true;
                        }
                        if !valid {
                            ui.label("审核快照已过期，请关闭后重新读取");
                        }
                    }
                    if ui.button("关闭审核").clicked() {
                        close = true;
                    }
                    if self.pending.is_some() {
                        ui.spinner();
                    }
                });
                if let Some(error) = &self.error {
                    ui.label(error);
                }
                self.state.read_only = true;
                self.state.filtering_enabled = false;
                self.state.sorting_enabled = false;
                ui.allocate_ui(egui::vec2(ui.available_width(), 240.0), |ui| {
                    DataGrid {
                        id: ui.id().with("review-rows"),
                        columns: &mut self.columns,
                        rows: self.rows.len(),
                        state: &mut self.state,
                        row_id: Some(&|row| self.rows[row].id),
                    }
                    .show(ui, |row, column| {
                        self.rows[row]
                            .cells
                            .get(&column)
                            .cloned()
                            .unwrap_or_default()
                    });
                });
                if let Some(credential) = &self.credential {
                    let total = credential.total;
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{}–{} / {total}",
                            self.offset + 1,
                            self.offset + self.rows.len() as u64
                        ));
                        if ui
                            .add_enabled(
                                valid && self.pending.is_none() && self.offset > 0,
                                egui::Button::new("上一页"),
                            )
                            .clicked()
                        {
                            self.page(session, ui.ctx(), self.offset.saturating_sub(100));
                        }
                        if ui
                            .add_enabled(
                                valid && self.pending.is_none() && self.offset + 100 < total,
                                egui::Button::new("下一页"),
                            )
                            .clicked()
                        {
                            self.page(session, ui.ctx(), self.offset + 100);
                        }
                        if ui
                            .add_enabled(
                                valid && self.pending.is_none(),
                                egui::Button::new("重读本页"),
                            )
                            .clicked()
                        {
                            self.page(session, ui.ctx(), self.offset);
                        }
                    });
                }
            });
        if self.pending.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(150));
        }
        (close, prepared)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait(review: &mut Review, session: &Session, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while review.pending.is_some() {
            assert!(Instant::now() < deadline, "review request stalled");
            review.poll(session, ctx);
            std::thread::sleep(Duration::from_millis(3));
        }
    }

    #[test]
    fn copy_retry_review_lost_response_reuses_identity_and_only_reads_bounded_preview() {
        let mut review = Review::new(RowId(55));
        let expires = super::super::super::unix_seconds() + 500;
        let credential = serde_json::json!({"token":RowId(9), "query_fingerprint":"0123456789abcdef0123456789abcdef", "total":1, "expires_at":expires});
        let response = serde_json::json!({"batch_id":review.batch,"request_id":review.request,"selection":credential});
        let page = serde_json::json!({"token":RowId(9),"total":1,"expires_at":expires,"data":[{
            "product_id":RowId(10),"tenant_id":RowId(1),"product_code":"CODE-10","name":"Latest product","status":"ACTIVE",
            "created_at":"2026-09-09T00:00:00Z","updated_at":"2026-09-10T00:00:00Z"
        }]});
        let (client, server) = crate::remote_actions::tests::server(vec![
            None,
            Some((200, response.to_string())),
            Some((200, page.to_string())),
        ]);
        let session = Session {
            client,
            tenant: RowId(1),
            token: "fixture-token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(3600),
        };
        let ctx = egui::Context::default();
        review.start(&session, &ctx);
        wait(&mut review, &session, &ctx);
        assert!(review.error.is_some());
        review.start(&session, &ctx);
        wait(&mut review, &session, &ctx);
        assert!(review.loaded, "{:?}", review.error);
        assert_eq!(review.rows[0].id, RowId(10));
        assert_eq!(
            review.rows[0].version.as_deref(),
            Some("2026-09-10T00:00:00Z")
        );
        let mut prepared = None;
        let mut initial = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    prepared = review.ui(ui, &session, true).1;
                });
            },
        );
        initial.textures_delta.clear();
        assert!(
            prepared.is_none(),
            "Reading a preview must never submit a new copy job"
        );
        let mut frame = |events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        if let Some(body) = review.ui(ui, &session, true).1 {
                            prepared = Some(body);
                        }
                    });
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = frame(vec![]);
        let point = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "确认按新版本复制"
                {
                    Some(text.pos + text.galley.size() / 2.0)
                } else {
                    None
                }
            })
            .expect("review confirmation button");
        frame(vec![egui::Event::PointerMoved(point)]);
        for pressed in [true, false] {
            frame(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }]);
        }
        let prepared = prepared.expect("explicit confirmation must prepare a fresh job");
        assert_ne!(prepared.batch_id, review.batch);
        assert_eq!(prepared.token, RowId(9));
        assert_eq!(prepared.confirmed_count, 1);
        assert!(prepared.excluded.is_empty());
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        let first: serde_json::Value =
            serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
        let second: serde_json::Value =
            serde_json::from_str(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(first, second);
        assert!(
            requests[0]
                .lines()
                .next()
                .unwrap()
                .contains("/retry-review")
        );
        assert!(requests[2].lines().next().unwrap().contains("/selections/"));
        let body: serde_json::Value =
            serde_json::from_str(requests[2].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["limit"], 100);
        assert_eq!(body["offset"], 0);
    }
}
