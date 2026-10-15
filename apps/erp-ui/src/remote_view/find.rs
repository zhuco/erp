use super::Session;
use crate::http::{HttpError, HttpRequest, HttpTask};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId, paged_query::PageQuery};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Hit {
    pub row_id: RowId,
    pub column: ColumnId,
    pub offset: u64,
    pub wrapped: bool,
}

#[derive(Default)]
pub(super) struct FindOutput {
    pub hit: Option<Hit>,
    pub expired: bool,
    pub selection_expired: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    tenant: RowId,
    generation: u64,
    _resource: String,
    endpoint: String,
    query: Vec<u8>,
    columns: Vec<ColumnId>,
}

pub(super) struct Target {
    pub resource: String,
    pub endpoint: String,
    pub query: Result<Vec<u8>, String>,
    pub window_id: egui::Id,
}

struct Job {
    identity: Identity,
    task: HttpTask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestFailure {
    Authentication,
    FrozenSelectionExpired,
    Other,
}

#[derive(Default)]
pub(super) struct Find {
    open: bool,
    focus_input: bool,
    text: String,
    case_sensitive: bool,
    composing: bool,
    after: Option<Hit>,
    binding: Option<Identity>,
    job: Option<Job>,
    feedback: Option<String>,
}

impl Find {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn clear(&mut self) {
        self.open = false;
        self.focus_input = false;
        self.text.clear();
        self.case_sensitive = false;
        self.composing = false;
        self.after = None;
        self.binding = None;
        self.job = None;
        self.feedback = None;
    }

    pub fn open(&mut self) {
        self.open = true;
        self.focus_input = true;
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &Session,
        generation: u64,
        target: Target,
        columns: &[Column],
        enabled: bool,
    ) -> FindOutput {
        let mut output = FindOutput::default();
        let visible = columns
            .iter()
            .filter(|column| column.visible)
            .map(|column| column.id)
            .collect::<Vec<_>>();
        let window_id = target.window_id;
        let identity = target.query.map(|query| Identity {
            tenant: session.tenant,
            generation,
            _resource: target.resource,
            endpoint: target.endpoint,
            query,
            columns: visible,
        });

        self.bind(identity);
        if !enabled {
            self.job = None;
            self.after = None;
        }

        if let Some(result) = self.job.as_mut().and_then(|job| job.task.poll()) {
            let Some(job) = self.job.take() else {
                return output;
            };
            if self.binding.as_ref() == Some(&job.identity) {
                match result {
                    Ok(bytes) => match decode_hit(&bytes, &job.identity) {
                        Ok(Some(hit)) => {
                            self.after = Some(hit.clone());
                            self.feedback = None;
                            output.hit = Some(hit);
                        }
                        Ok(None) => self.feedback = Some("未找到".into()),
                        Err(error) => self.feedback = Some(error),
                    },
                    Err(error) => match classify_failure(&error) {
                        RequestFailure::Authentication => {
                            output.expired = true;
                            self.clear();
                        }
                        RequestFailure::FrozenSelectionExpired => {
                            output.selection_expired = true;
                            self.clear();
                        }
                        RequestFailure::Other => self.feedback = Some(error.to_string()),
                    },
                }
            }
        }

        if !self.open {
            return output;
        }
        ui.ctx().input(|input| {
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => {
                            self.composing = false;
                        }
                        _ => {}
                    }
                }
            }
        });
        let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
        let mut open = self.open && !escape;
        let mut direction = None;
        let mut edited = false;
        egui::Window::new("查找")
            .id(window_id)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.text)
                            .char_limit(256)
                            .hint_text("查找"),
                    );
                    if self.focus_input {
                        response.request_focus();
                        self.focus_input = false;
                    }
                    edited |= response.changed();
                    edited |= ui
                        .checkbox(&mut self.case_sensitive, "区分大小写")
                        .on_hover_text("按字符大小写精确匹配")
                        .changed();
                    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                    let backward = ui.input(|input| input.modifiers.shift);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(self.job.is_none(), egui::Button::new("上一个"))
                            .on_hover_text("Shift+Enter")
                            .clicked()
                        {
                            direction = Some(true);
                        }
                        if ui
                            .add_enabled(self.job.is_none(), egui::Button::new("下一个"))
                            .on_hover_text("Enter")
                            .clicked()
                        {
                            direction = Some(false);
                        }
                    });
                    if enter && !self.composing && self.job.is_none() {
                        direction = Some(backward);
                    }
                });
                if self.job.is_some() {
                    ui.spinner();
                }
                if let Some(feedback) = &self.feedback {
                    ui.label(feedback);
                }
            });
        self.open = open;
        if !open {
            self.job = None;
            self.after = None;
            output.hit = None;
            return output;
        }
        if edited {
            self.job = None;
            self.after = None;
            self.feedback = None;
            output.hit = None;
        }
        if let Some(backward) = direction
            && enabled
            && !self.text.is_empty()
            && self.text.len() <= 1024
            && !self.composing
            && self.job.is_none()
            && let Some(identity) = self.binding.clone()
        {
            match start_request(
                session,
                ui.ctx(),
                &identity,
                &self.text,
                self.case_sensitive,
                backward,
                self.after.as_ref(),
            ) {
                Ok(task) => {
                    self.feedback = None;
                    self.job = Some(Job { identity, task });
                }
                Err(error) => self.feedback = Some(error),
            }
        } else if direction.is_some() && self.text.len() > 1024 {
            self.feedback = Some("查找文本超过 1024 字节".into());
        }
        if self.job.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(25));
        }
        output
    }

    fn bind(&mut self, identity: Result<Identity, String>) {
        match identity {
            Ok(identity) => {
                if self.binding.as_ref() != Some(&identity) {
                    self.job = None;
                    self.after = None;
                    self.feedback = None;
                    self.binding = Some(identity);
                }
            }
            Err(error) => {
                self.job = None;
                self.after = None;
                self.binding = None;
                self.feedback = Some(error);
            }
        }
    }
}

fn classify_failure(error: &HttpError) -> RequestFailure {
    match error {
        HttpError::Status(401 | 403) => RequestFailure::Authentication,
        HttpError::Status(410) => RequestFailure::FrozenSelectionExpired,
        _ => RequestFailure::Other,
    }
}

fn start_request(
    session: &Session,
    ctx: &egui::Context,
    identity: &Identity,
    text: &str,
    case_sensitive: bool,
    backward: bool,
    after: Option<&Hit>,
) -> Result<HttpTask, String> {
    let query: PageQuery =
        serde_json::from_slice(&identity.query).map_err(|_| "查找条件无效".to_owned())?;
    let after = after.map(|hit| {
        serde_json::json!({
            "row_id": hit.row_id.to_uuid(),
            "column": hit.column,
        })
    });
    let body = serde_json::to_vec(&serde_json::json!({
        "query": query,
        "text": text,
        "columns": identity.columns,
        "case_sensitive": case_sensitive,
        "backward": backward,
        "after": after,
    }))
    .map_err(|_| "无法编码查找请求".to_owned())?;
    session
        .client
        .start(
            HttpRequest {
                path: identity.endpoint.clone(),
                query: vec![],
                json: Some(body),
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            ctx,
        )
        .map_err(|error| error.to_string())
}

#[derive(serde::Deserialize)]
struct Response {
    hit: Option<WireHit>,
}

#[derive(serde::Deserialize)]
struct WireHit {
    row_id: String,
    column: ColumnId,
    offset: u64,
    wrapped: bool,
}

fn decode_hit(bytes: &[u8], identity: &Identity) -> Result<Option<Hit>, String> {
    let response: Response =
        serde_json::from_slice(bytes).map_err(|_| "查找响应无效".to_owned())?;
    let Some(hit) = response.hit else {
        return Ok(None);
    };
    let row_id = RowId::from_uuid(&hit.row_id)
        .filter(|row| row.0 != 0)
        .ok_or_else(|| "查找结果行身份无效".to_owned())?;
    if !identity.columns.contains(&hit.column) || hit.offset > i64::MAX as u64 - 500 {
        return Err("查找结果列或位置无效".into());
    }
    Ok(Some(Hit {
        row_id,
        column: hit.column,
        offset: hit.offset,
        wrapped: hit.wrapped,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> Identity {
        Identity {
            tenant: RowId(1),
            generation: 7,
            _resource: "products".into(),
            endpoint: "/products/find".into(),
            query: b"{}".to_vec(),
            columns: vec![ColumnId(1), ColumnId(2)],
        }
    }

    #[test]
    fn decoder_rejects_malformed_or_nil_row_identity() {
        for row in ["bad", "00000000-0000-0000-0000-000000000000"] {
            let body = serde_json::json!({
                "hit": {"row_id": row, "column": 1, "offset": 0, "wrapped": false}
            });
            assert!(decode_hit(body.to_string().as_bytes(), &identity()).is_err());
        }
    }

    #[test]
    fn decoder_rejects_hidden_column() {
        let body = serde_json::json!({
            "hit": {
                "row_id": RowId(9).to_uuid(),
                "column": 3,
                "offset": 12,
                "wrapped": true
            }
        });
        assert!(decode_hit(body.to_string().as_bytes(), &identity()).is_err());
    }

    #[test]
    fn resource_identity_keeps_parent_bound_jobs_separate() {
        let first = identity();
        let mut second = first.clone();
        second._resource = "product-skus:parent-2".into();
        assert_ne!(first, second);
    }

    #[test]
    fn frozen_token_endpoint_is_part_of_request_identity() {
        let first = identity();
        let mut second = first.clone();
        second.endpoint = "/products/selections/token-2/find".into();
        assert_ne!(first, second);
    }

    #[test]
    fn gone_is_a_frozen_selection_expiry_not_an_auth_expiry() {
        assert_eq!(
            classify_failure(&HttpError::Status(410)),
            RequestFailure::FrozenSelectionExpired
        );
        assert_eq!(
            classify_failure(&HttpError::Status(401)),
            RequestFailure::Authentication
        );
    }

    #[test]
    fn unchanged_binding_keeps_next_match_anchor() {
        let binding = identity();
        let anchor = Hit {
            row_id: RowId(9),
            column: ColumnId(1),
            offset: 120,
            wrapped: false,
        };
        let mut find = Find {
            after: Some(anchor.clone()),
            binding: Some(binding.clone()),
            ..Default::default()
        };
        find.bind(Ok(binding));
        assert_eq!(find.after, Some(anchor));
        let mut changed = identity();
        changed.query = br#"{"search":"new"}"#.to_vec();
        find.bind(Ok(changed));
        assert_eq!(find.after, None);
    }
}
