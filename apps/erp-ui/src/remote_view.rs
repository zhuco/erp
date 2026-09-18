use crate::{
    http::{HttpClient, HttpError, HttpRequest, HttpTask},
    remote_products as products,
};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, FilterDraft, FilterKind, RowId, remote::*};
use erp_ui_egui::{DataGrid, GridState};
use std::time::Duration;
mod actions;
mod copy_batches;
mod cross_page_fill;
mod details;
mod draft_restore;
mod drafts;
mod editing;
mod exports;
mod find;
mod inventory_report;
mod options;
mod orders;
mod paste;
mod preferences;
mod product_inspector;
mod selection;
mod views;
mod warehouse_tree;
pub(crate) mod workbench_layout;

struct Session {
    client: HttpClient,
    tenant: RowId,
    token: String,
    expires: web_time::Instant,
}
struct Login {
    client: HttpClient,
    tenant: RowId,
    task: HttpTask,
}

pub struct RemoteProductView {
    server: String,
    tenant: String,
    email: String,
    password: String,
    session: Option<Session>,
    login: Option<Login>,
    pages: RemotePages,
    jobs: Vec<(PageRequest, HttpTask)>,
    query: Option<RemoteQuery>,
    address: PageAddress,
    page_number: u64,
    advanced_tools: bool,
    state: GridState,
    columns: Vec<Column>,
    generation: u64,
    session_number: u64,
    actions: actions::ProductActions,
    copy_batches: copy_batches::CopyBatches,
    drafts: drafts::Drafts,
    exports: exports::Exports,
    find: find::Find,
    find_pending: Option<find::Hit>,
    inventory_report: inventory_report::InventoryReport,
    warehouse_tree: warehouse_tree::WarehouseTree,
    orders: orders::Orders,
    views: views::Views,
    views_storage_ready: bool,
    editing: Option<editing::Editing>,
    paste: Option<paste::PasteReview>,
    cross_page_fill_range: cross_page_fill::CrossPageFillRange,
    cross_page_fill: Option<cross_page_fill::CrossPageFill>,
    resume_editor: Option<editing::NextEditor>,
    details: details::Details,
    inspector: product_inspector::ProductInspector,
    options: options::ProductOptions,
    frozen: selection::FrozenSelection,
    conditions: erp_ui_egui::query_builder::QueryBuilder,
    conditions_open: bool,
    displayed_ids: Vec<RowId>,
    search: String,
    deadline: Option<f64>,
    composing: bool,
    notice: Option<String>,
    elapsed: Option<Duration>,
    render_pending: Option<web_time::Instant>,
    render_elapsed: Option<Duration>,
    delivery_elapsed: Option<Duration>,
    invalid_query: bool,
    formal_host: bool,
    workbench: workbench_layout::WorkbenchLayout,
}
impl Default for RemoteProductView {
    fn default() -> Self {
        let mut state = GridState::default();
        state.metrics = erp_ui_egui::GridMetrics::legacy();
        state.read_only = true;
        Self {
            server: String::new(),
            tenant: String::new(),
            email: String::new(),
            password: String::new(),
            session: None,
            login: None,
            pages: RemotePages::default(),
            jobs: Vec::new(),
            query: None,
            address: PageAddress::Offset {
                start: 0,
                limit: 100,
            },
            page_number: 0,
            advanced_tools: false,
            state,
            columns: [
                "编码",
                "商品",
                "品牌",
                "分类",
                "状态",
                "创建时间（UTC）",
                "更新时间（UTC）",
            ]
            .into_iter()
            .enumerate()
            .map(|(index, title)| {
                let mut column = Column::new(
                    ColumnId(index as u64),
                    title,
                    if index == 1 { 240.0 } else { 160.0 },
                )
                .with_filter(if index == 4 {
                    FilterKind::Enum(vec!["ACTIVE".into(), "INACTIVE".into()])
                } else if index >= 5 {
                    FilterKind::DateTime
                } else {
                    FilterKind::Text
                });
                column.edit_kind = match index {
                    1 => erp_grid_core::EditKind::Text,
                    0 | 2 | 3 => erp_grid_core::EditKind::OptionalText,
                    _ => erp_grid_core::EditKind::ReadOnly,
                };
                column
            })
            .collect(),
            generation: 0,
            session_number: 0,
            actions: Default::default(),
            copy_batches: Default::default(),
            drafts: Default::default(),
            exports: Default::default(),
            find: Default::default(),
            find_pending: None,
            inventory_report: Default::default(),
            warehouse_tree: Default::default(),
            orders: Default::default(),
            views: Default::default(),
            views_storage_ready: false,
            editing: None,
            paste: None,
            cross_page_fill_range: Default::default(),
            cross_page_fill: None,
            resume_editor: None,
            details: Default::default(),
            inspector: Default::default(),
            options: Default::default(),
            frozen: Default::default(),
            conditions: Default::default(),
            conditions_open: false,
            displayed_ids: Vec::new(),
            search: String::new(),
            deadline: None,
            composing: false,
            notice: None,
            elapsed: None,
            render_pending: None,
            render_elapsed: None,
            delivery_elapsed: None,
            invalid_query: false,
            formal_host: false,
            workbench: Default::default(),
        }
    }
}
impl RemoteProductView {
    /// Selects the production workbench.  Fixture-only inventory and warehouse
    /// entry points remain available to GridLab, but are hidden here.
    pub(crate) fn set_formal_host(&mut self, formal: bool) {
        self.formal_host = formal;
        if formal {
            self.state.theme = Some(crate::erp_theme::grid());
        }
    }

    pub(crate) fn page_scope(&self) -> Option<String> {
        let session = self
            .session
            .as_ref()
            .filter(|s| web_time::Instant::now() < s.expires)?;
        let actor = self.actions.actor()?;
        Some(format!(
            "{}:{}:{}",
            session.client.base_url(),
            session.tenant.to_uuid(),
            actor.to_uuid()
        ))
    }

    pub(crate) fn suspend_closed_page(&mut self, orders_mode: bool) {
        if orders_mode {
            self.orders.suspend_reads();
        } else {
            self.suspend_page_queries();
        }
    }

    pub(crate) fn refresh_page(&mut self, orders_mode: bool) {
        if self.page_dirty(orders_mode) {
            return;
        }
        if orders_mode {
            self.orders.suspend_reads();
        } else {
            self.reset_query(false);
        }
    }

    /// Leaving the host cancels disposable reads, never a write/receipt workflow.
    pub(crate) fn suspend_page_queries(&mut self) {
        self.jobs.clear();
        self.pages.invalidate();
        self.composing = false;
        self.render_pending = None;
        self.find.clear();
        self.find_pending = None;
        self.options.clear();
        self.frozen.suspend_reads();
        self.exports.suspend_reads();
        self.details.suspend_page_queries();
    }
    fn invalidate_selection_page(&mut self) {
        if let Some(fill) = &mut self.cross_page_fill {
            fill.cancel();
        }
        self.cross_page_fill = None;
        self.cross_page_fill_range = Default::default();
        self.find.clear();
        self.find_pending = None;
        self.jobs.clear();
        self.pages.invalidate();
        if let Some(query) = &self.query {
            self.pages.bind(query.clone());
        }
        self.page_number = 0;
        self.address = PageAddress::Offset {
            start: 0,
            limit: self.address.limit() as u16,
        };
        self.generation += 1;
        self.state.bind_view(self.generation);
        self.displayed_ids.clear();
        self.elapsed = None;
        self.render_pending = None;
        self.render_elapsed = None;
        self.delivery_elapsed = None;
    }
    fn clear_frozen(&mut self) {
        let changed = self.frozen.is_active() || self.frozen.pending();
        self.frozen.clear();
        if changed {
            self.invalidate_selection_page();
        }
    }
    fn start_frozen(&mut self, ctx: &egui::Context) {
        if self.dirty() || self.deadline.is_some() || self.invalid_query {
            return;
        }
        if let (Some(session), Some(query)) = (&self.session, &self.query) {
            let request = PageRequest {
                ticket: 0,
                generation: self.generation,
                query: query.clone(),
                address: self.address.clone(),
            };
            match self.frozen.start(session, &request, ctx, unix_seconds()) {
                Ok(()) => self.invalidate_selection_page(),
                Err(error) => self.notice = Some(error),
            }
        }
    }
    fn disconnect(&mut self) {
        if let Some(fill) = &mut self.cross_page_fill {
            fill.cancel();
        }
        self.cross_page_fill = None;
        self.cross_page_fill_range = Default::default();
        self.orders.clear();
        self.inventory_report.clear();
        self.warehouse_tree.clear();
        self.find.clear();
        self.find_pending = None;
        self.exports.clear();
        self.copy_batches.clear();
        self.frozen.clear();
        self.options = Default::default();
        self.details.clear();
        self.inspector = Default::default();
        self.editing = None;
        self.resume_editor = None;
        self.actions = Default::default();
        self.login = None;
        self.session = None;
        self.jobs.clear();
        self.pages.clear();
        self.query = None;
        self.page_number = 0;
        self.deadline = None;
        self.displayed_ids.clear();
        self.search.clear();
        self.conditions.clear();
        let defaults = Self::default();
        self.columns = defaults.columns;
        self.address = defaults.address;
        self.state.quick_search_open = true;
        self.state.quick_search.clear();
        self.state.sorts.clear();
        self.generation += 1;
        self.state.bind_view(self.generation);
        self.password.clear();
        self.elapsed = None;
        self.render_pending = None;
        self.render_elapsed = None;
        self.delivery_elapsed = None;
        self.invalid_query = false;
    }
    fn start_login(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let tenant = RowId::from_uuid(self.tenant.trim()).ok_or("Invalid merchant UUID")?;
        if self.email.trim().is_empty()
            || self.password.is_empty()
            || self.email.len() > 320
            || self.password.len() > 4096
            || self.server.len() > 4096
        {
            return Err("Email and password are required".into());
        }
        let client = HttpClient::new(self.server.trim()).map_err(|e| e.to_string())?;
        let json = serde_json::to_vec(&serde_json::json!({ "tenant_id": tenant.to_uuid(), "email": self.email.trim(), "password": self.password })).map_err(|_| "Cannot encode login")?;
        let task = client
            .start(
                HttpRequest {
                    path: "/auth/login".into(),
                    query: vec![],
                    json: Some(json),
                    bearer: None,
                },
                ctx,
            )
            .map_err(|e| e.to_string())?;
        self.login = Some(Login {
            client,
            tenant,
            task,
        });
        self.password.clear();
        self.notice = None;
        Ok(())
    }
    fn reset_query(&mut self, clear_checks: bool) {
        if let Some(fill) = &mut self.cross_page_fill {
            fill.cancel();
        }
        self.cross_page_fill = None;
        self.cross_page_fill_range = Default::default();
        self.find.clear();
        self.find_pending = None;
        if clear_checks {
            self.clear_frozen();
        }
        let Some(session) = &self.session else {
            return;
        };
        let mut filters = std::collections::BTreeMap::new();
        for column in &self.columns {
            if let Some(draft) = self.state.quick_search.get(&column.id) {
                match draft.compile(&column.filter) {
                    Ok(Some(filter)) => {
                        filters.insert(column.id, filter);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        self.invalid_query = true;
                        self.jobs.clear();
                        self.pages.invalidate();
                        self.deadline = None;
                        self.notice = Some(error.into());
                        return;
                    }
                }
            }
        }
        let expression = match self.conditions.compile(&self.columns) {
            Ok(expression) => expression,
            Err(error) => {
                self.invalid_query = true;
                self.jobs.clear();
                self.pages.invalidate();
                self.deadline = None;
                self.notice = Some(error);
                return;
            }
        };
        let mut query = RemoteQuery {
            expression: expression.map(Box::new),
            scope: DataScope {
                source: format!("products:{}", session.tenant.to_uuid()),
                schema: 1,
                authorization: format!("session:{}", self.session_number),
            },
            filters,
            sorts: self.state.sorts.clone(),
            search: (!self.search.trim().is_empty()).then(|| self.search.trim().to_owned()),
        };
        self.invalid_query = false;
        let clear_checks = clear_checks
            || self
                .query
                .as_ref()
                .is_none_or(|previous| !previous.same_selection_scope(&query));
        if clear_checks {
            self.clear_frozen();
            query.sorts = self.state.sorts.clone();
        }
        self.jobs.clear();
        self.pages.invalidate();
        self.pages.bind(query.clone());
        self.query = Some(query);
        self.address = PageAddress::Offset {
            start: 0,
            limit: self.address.limit() as u16,
        };
        self.page_number = 0;
        self.deadline = None;
        self.notice = None;
        self.elapsed = None;
        self.render_pending = None;
        self.render_elapsed = None;
        self.delivery_elapsed = None;
        self.generation += 1;
        if clear_checks {
            self.state.bind_view(self.generation);
        } else {
            self.state.rebind_page(self.generation, true);
        }
    }
    fn poll_cross_page_fill(&mut self, ctx: &egui::Context) -> bool {
        let Some(query) = self.query.clone() else {
            return false;
        };
        let Some(session) = self.session.as_ref() else {
            return false;
        };
        let Some(fill) = &mut self.cross_page_fill else {
            return false;
        };
        fill.poll(session, &query, self.generation, ctx, unix_seconds());
        let authentication_failed = fill.authentication_failed();
        let preview = fill
            .take_preview()
            .map(|preview| (preview, fill.columns().to_vec()));
        if authentication_failed {
            fill.cancel();
            self.cross_page_fill = None;
            self.cross_page_fill_range = Default::default();
            self.actions.suspend();
            self.state.hold_edit();
            self.details.clear();
            if let Some(session) = &mut self.session {
                session.expires = web_time::Instant::now();
            }
            self.notice = Some("Session expired or product access denied".into());
            return true;
        }
        if let Some((preview, columns)) = preview {
            self.cross_page_fill = None;
            if preview.edits.is_empty() {
                self.state.message = Some("填充内容没有变化".into());
            } else {
                self.paste = Some(paste::PasteReview::new(preview, &columns));
            }
        }
        false
    }
    fn tick(&mut self, ctx: &egui::Context) {
        let previous = self.frozen.credential().map(|credential| credential.token);
        self.frozen.poll(ctx, unix_seconds());
        let current = self.frozen.credential().map(|credential| credential.token);
        if previous != current {
            self.invalidate_selection_page();
        }
        let login_result = self.login.as_mut().and_then(|login| login.task.poll());
        if let Some(result) = login_result
            && let Some(login) = self.login.take()
        {
            #[derive(serde::Deserialize)]
            struct Token {
                access_token: String,
                token_type: String,
                expires_in: u64,
            }
            let token = result.map_err(|e| e.to_string()).and_then(|bytes| {
                serde_json::from_slice::<Token>(&bytes).map_err(|_| "Invalid login response".into())
            });
            match token {
                Ok(token)
                    if token.token_type.eq_ignore_ascii_case("bearer")
                        && !token.access_token.is_empty()
                        && token.access_token.len() <= 16_384
                        && (1..=86_400).contains(&token.expires_in) =>
                {
                    let reauth = self.session.is_some();
                    self.clear_frozen();
                    self.session = Some(Session {
                        client: login.client,
                        tenant: login.tenant,
                        token: token.access_token,
                        expires: web_time::Instant::now() + Duration::from_secs(token.expires_in),
                    });
                    self.session_number += 1;
                    self.inventory_report.clear();
                    self.warehouse_tree.clear();
                    if let Some(session) = &self.session {
                        self.actions.start(session, ctx);
                    }
                    if !reauth {
                        self.orders.clear();
                        self.reset_query(true);
                    } else if !self.dirty() {
                        self.pages.invalidate();
                    }
                    self.notice = None;
                }
                Ok(_) => self.notice = Some("Invalid login response".into()),
                Err(error) => self.notice = Some(error),
            }
        }
        if self
            .session
            .as_ref()
            .is_some_and(|session| web_time::Instant::now() >= session.expires)
        {
            self.inventory_report.clear();
            self.warehouse_tree.clear();
            self.drafts.clear();
            self.exports.clear();
            self.find.clear();
            self.find_pending = None;
            self.clear_frozen();
            self.actions.suspend();
            self.state.hold_edit();
            self.details.clear();
            self.jobs.clear();
            self.notice = Some("Session expired; sign in again".into());
            return;
        }
        if self.poll_cross_page_fill(ctx) {
            return;
        }
        if self.tick_orders(ctx) {
            return;
        }
        let changed = self
            .session
            .as_ref()
            .is_some_and(|session| self.actions.tick(session, ctx));
        self.finish_editor();
        if changed && !self.dirty() {
            self.jobs.clear();
            self.pages.invalidate();
            if !self.invalid_query {
                self.notice = None;
            }
        }
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
        });
        if self.deadline.is_some_and(|at| ctx.input(|i| i.time) >= at) && !self.composing {
            self.reset_query(true);
        }
        let mut index = 0;
        while index < self.jobs.len() {
            let Some(result) = self.jobs[index].1.poll() else {
                index += 1;
                continue;
            };
            let completed_duration = self.jobs[index].1.completed_duration();
            let completed_at = self.jobs[index].1.completed_at();
            let delivery_delay = self.jobs[index].1.delivery_delay();
            let (request, _) = self.jobs.remove(index);
            if matches!(result, Err(HttpError::Status(410))) && self.frozen.is_active() {
                self.clear_frozen();
                self.notice = Some("冻结选择已失效，请刷新后重新勾选".into());
                return;
            }
            if matches!(result, Err(HttpError::Status(401 | 403))) {
                self.clear_frozen();
                self.actions.suspend();
                self.state.hold_edit();
                self.details.clear();
                self.jobs.clear();
                if let Some(session) = &mut self.session {
                    session.expires = web_time::Instant::now();
                }
                self.notice = Some("Session expired or access denied".into());
                return;
            }
            let Some(session) = &self.session else {
                continue;
            };
            let result = result.map_err(|e| e.to_string()).and_then(|bytes| {
                if let Some(credential) = self.frozen.credential() {
                    crate::remote_selection::decode_page(
                        session.tenant,
                        credential,
                        &request,
                        &bytes,
                        unix_seconds(),
                    )
                } else {
                    products::decode_page(session.tenant, &request, &bytes)
                }
            });
            match self.pages.accept(&request, result) {
                PageAcceptance::Stored if request.address == self.address => {
                    self.elapsed = completed_duration;
                    self.render_pending = completed_at;
                    self.delivery_elapsed = delivery_delay;
                    self.render_elapsed = None;
                    self.notice = None;
                    let ids: Vec<_> = self
                        .pages
                        .page(&self.address)
                        .into_iter()
                        .flat_map(|page| page.rows.iter().map(|row| row.id))
                        .collect();
                    if ids != self.displayed_ids {
                        self.generation += 1;
                        self.state.rebind_page(self.generation, false);
                        self.displayed_ids = ids;
                    }
                }
                PageAcceptance::Failed(error) if request.address == self.address => {
                    self.notice = Some(error)
                }
                _ => {}
            }
        }
        if self.session.is_some()
            && self.deadline.is_none()
            && self.notice.is_none()
            && let Err(error) = self.request_page(ctx)
        {
            self.notice = Some(error);
        }
        if self.deadline.is_some() && !self.composing {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        self.resume_editing();
        if self.resume_editor.is_some() {
            ctx.request_repaint();
        }
    }
    fn request_page(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.frozen.pending() && !self.frozen.is_active() {
            return Ok(());
        }
        let Some(session) = &self.session else {
            return Ok(());
        };
        if let PageStart::Fetch(request) = self.pages.start(self.address.clone())? {
            let encoded = if let Some(credential) = self.frozen.credential() {
                let PageAddress::Offset { start, limit } = &request.address else {
                    return Err("Frozen selection requires offset pages".into());
                };
                crate::remote_selection::encode_page(
                    session.tenant,
                    credential,
                    *start,
                    *limit,
                    &request.query.sorts,
                )
            } else {
                products::encode_query(session.tenant, &request)
            };
            let result = encoded.and_then(|query| {
                session
                    .client
                    .start(
                        HttpRequest {
                            path: query.path,
                            query: vec![],
                            json: Some(query.body),
                            bearer: Some(session.token.clone()),
                        },
                        ctx,
                    )
                    .map_err(|e| e.to_string())
            });
            match result {
                Ok(task) => self.jobs.push((request, task)),
                Err(error) => {
                    self.pages.accept(&request, Err(error.clone()));
                    return Err(error);
                }
            }
        }
        Ok(())
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.ui_mode(ui, false);
    }
    pub fn orders_ui(&mut self, ui: &mut egui::Ui) {
        self.ui_mode(ui, true);
    }
    pub(crate) fn tick_orders(&mut self, ctx: &egui::Context) -> bool {
        self.exports.tick_download(ctx);
        let denied = self
            .session
            .as_ref()
            .filter(|session| web_time::Instant::now() < session.expires)
            .is_some_and(|session| self.orders.tick(ctx, session));
        if denied {
            if let Some(session) = &mut self.session {
                session.expires = web_time::Instant::now();
            }
            self.notice = Some("Session expired or order access denied".into());
        }
        denied
    }
    pub(crate) fn close_orders_page(&mut self) {
        if !self.orders.dirty() {
            self.orders.clear();
        }
    }
    fn ui_mode(&mut self, ui: &mut egui::Ui, orders_mode: bool) {
        self.tick(ui.ctx());
        if self.deadline.is_none()
            && !self.invalid_query
            && let (Some(session), Some(query)) = (&self.session, &self.query)
            && web_time::Instant::now() < session.expires
        {
            if self.options.tick(ui.ctx(), session, query) {
                self.actions.suspend();
                self.state.hold_edit();
                if let Some(session) = &mut self.session {
                    session.expires = web_time::Instant::now();
                }
            } else {
                self.options
                    .update_columns(&mut self.columns, &mut self.state.quick_search);
            }
        }
        if self
            .session
            .as_ref()
            .is_some_and(|session| web_time::Instant::now() >= session.expires)
        {
            ui.horizontal(|ui| {
                ui.label("Session expired. Sign in again to keep working.");
                ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                if ui
                    .add_enabled(self.login.is_none(), egui::Button::new("登录"))
                    .clicked()
                    && let Err(error) = self.start_login(ui.ctx())
                {
                    self.notice = Some(error);
                }
                if self.login.is_some() {
                    ui.spinner();
                }
                if let Some(draft) = self.state.edit_draft()
                    && ui.button("Copy draft").clicked()
                {
                    ui.ctx().copy_text(draft.after);
                }
            });
            if let Some(error) = &self.notice {
                ui.label(error);
            }
            return;
        }
        if self.session.is_none() {
            ui.horizontal_wrapped(|ui| {
                ui.label("服务地址");
                ui.add(
                    egui::TextEdit::singleline(&mut self.server)
                        .desired_width(220.0)
                        .hint_text("https://erp.example"),
                );
                ui.label("商户");
                ui.add(
                    egui::TextEdit::singleline(&mut self.tenant)
                        .desired_width(280.0)
                        .hint_text("UUID"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("邮箱");
                ui.text_edit_singleline(&mut self.email);
                ui.label("密码");
                ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                if ui
                    .add_enabled(self.login.is_none(), egui::Button::new("登录"))
                    .clicked()
                    && let Err(error) = self.start_login(ui.ctx())
                {
                    self.notice = Some(error);
                }
                if self.login.is_some() && ui.button("Cancel").clicked() {
                    self.login = None;
                }
            });
            if self.login.is_some() {
                ui.spinner();
            }
            if let Some(error) = &self.notice {
                ui.label(error);
            }
            return;
        }
        if orders_mode {
            let sign_out = self.orders.take_sign_out() && !self.dirty();
            if sign_out {
                self.disconnect();
                return;
            }
            if let Some(session) = &self.session {
                self.orders.ui(ui, session);
            }
            return;
        }
        let mut sign_out = false;
        if self.formal_host {
            ui.horizontal(|ui| {
                if ui
                    .toggle_value(&mut self.workbench.preferences.left_open, "筛选面板")
                    .changed()
                {
                    self.workbench.mark_dirty();
                }
                if ui
                    .add_enabled(
                        !self.details.dirty(),
                        egui::Button::new("底部明细")
                            .selected(self.workbench.preferences.bottom_open),
                    )
                    .clicked()
                {
                    self.workbench.preferences.bottom_open =
                        !self.workbench.preferences.bottom_open;
                    self.workbench.mark_dirty();
                }
            });
        }
        let mut left_status_changed = None;
        if self.formal_host && self.workbench.preferences.left_open {
            let width = self.workbench.preferences.left_width;
            let response = egui::Panel::left(
                ui.id()
                    .with(("product-business-filter", self.workbench.scope_key())),
            )
            .resizable(true)
            .exact_size(width)
            .size_range(160.0..=420.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label("商品状态");
                let selected_status = match self.state.quick_search.get(&products::STATUS) {
                    Some(FilterDraft::Enum(values))
                        if values.len() == 1 && values.contains("ACTIVE") =>
                    {
                        Some("ACTIVE")
                    }
                    Some(FilterDraft::Enum(values))
                        if values.len() == 1 && values.contains("INACTIVE") =>
                    {
                        Some("INACTIVE")
                    }
                    _ => None,
                };
                for (value, label) in [
                    (None, "全部"),
                    (Some("ACTIVE"), "启用"),
                    (Some("INACTIVE"), "停用"),
                ] {
                    if ui
                        .add_enabled(
                            !self.dirty(),
                            egui::Button::selectable(selected_status == value, label),
                        )
                        .clicked()
                    {
                        left_status_changed = Some(value);
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("筛选");
                    if ui
                        .add_enabled(!self.dirty(), egui::Button::new("品牌"))
                        .clicked()
                    {
                        self.options.open_column(products::BRAND);
                    }
                    if ui
                        .add_enabled(!self.dirty(), egui::Button::new("分类"))
                        .clicked()
                    {
                        self.options.open_column(products::CATEGORY);
                    }
                });
            });
            self.workbench
                .set_left_width(response.response.rect.width());
        }
        if let Some(status) = left_status_changed.filter(|_| !self.dirty()) {
            let mut values = std::collections::BTreeSet::new();
            if let Some(status) = status {
                values.insert(status.to_owned());
            }
            self.state
                .quick_search
                .insert(products::STATUS, FilterDraft::Enum(values));
            self.jobs.clear();
            self.pages.invalidate();
            self.clear_frozen();
            self.details.clear();
            self.options.clear();
            self.deadline = Some(ui.input(|i| i.time) + 0.25);
            self.notice = None;
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
        ui.add_enabled_ui(!self.dirty(), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.toggle_value(&mut self.advanced_tools, "更多操作");
                ui.label("搜索")
                    .on_hover_text("Server search across product name, code, alias and barcode");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.search)
                            .desired_width(240.0)
                            .char_limit(4096),
                    )
                    .changed()
                {
                    self.jobs.clear();
                    self.pages.invalidate();
                    self.clear_frozen();
                    self.deadline = Some(ui.input(|i| i.time) + 0.25);
                    self.options.clear();
                    self.notice = None;
                    ui.ctx().request_repaint_after(Duration::from_millis(250));
                }
                if ui.button("刷新").clicked() {
                    self.options.clear();
                    if self.invalid_query {
                        self.reset_query(false);
                    } else {
                        self.jobs.clear();
                        self.pages.invalidate();
                        self.notice = None;
                    }
                    ui.ctx().request_repaint();
                }
                if !self.formal_host {
                    self.inventory_report.open_button(ui, self.session_number);
                    self.warehouse_tree.open_button(ui, self.session_number);
                }
                self.state.quick_search_open = true;
                if ui.button("重置").clicked() {
                    self.conditions.clear();
                    self.options.clear();
                    self.search.clear();
                    self.state.quick_search.clear();
                    self.reset_query(true);
                    ui.ctx().request_repaint();
                }
                if ui.button("清空勾选").clicked() {
                    self.clear_frozen();
                    self.state.row_selection.clear_checked();
                }
                if ui.add_enabled(!self.frozen.pending() && !self.frozen.is_active() && self.deadline.is_none() && !self.invalid_query && self.pages.page(&self.address).is_some(), egui::Button::new("全部匹配"))
                    .on_hover_text("冻结当前查询的全部匹配记录，有效十分钟；排序和翻页保留选择。再次勾选可排除或恢复当前页记录。")
                    .clicked() {
                    self.start_frozen(ui.ctx());
                }
                if ui.button("组合条件").on_hover_text("Combine nested AND/OR conditions with the quick search filters").clicked() {
                    self.conditions_open = !self.conditions_open;
                }
                if ui.button("筛选选项").on_hover_text("Search and select brand, category, or status values across the complete server query").clicked() {
                    self.options.open = true;
                    ui.ctx().request_repaint();
                }
                sign_out = ui.button("退出登录").clicked();
            });
        });
        if !self.formal_host
            && let Some(session) = &self.session
        {
            self.inventory_report
                .ui(ui.ctx(), session, self.session_number);
            if self
                .warehouse_tree
                .ui(ui.ctx(), session, self.session_number)
            {
                self.actions.suspend();
                self.state.hold_edit();
                if let Some(session) = &mut self.session {
                    session.expires = web_time::Instant::now();
                }
            }
        }
        let enabled = !self.dirty();
        if self.conditions_open {
            let mut changed = false;
            egui::Window::new("Conditions")
                .id(ui.id().with("conditions"))
                .open(&mut self.conditions_open)
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(420.0)
                            .show(ui, |ui| {
                                changed = self.conditions.show(
                                    ui,
                                    ui.id().with("builder"),
                                    &self.columns,
                                );
                            });
                    });
                });
            if changed {
                self.jobs.clear();
                self.pages.invalidate();
                self.options.clear();
                self.clear_frozen();
                self.deadline = Some(ui.input(|i| i.time) + 0.25);
                self.notice = None;
                ui.ctx().request_repaint_after(Duration::from_millis(250));
            }
        }
        if let Some((column, selected)) = self.options.ui(ui, &self.state.quick_search, enabled) {
            if let Some(schema) = self.columns.iter_mut().find(|schema| schema.id == column) {
                schema.filter = FilterKind::Enum(selected.iter().cloned().collect());
            }
            self.state
                .quick_search
                .insert(column, erp_grid_core::FilterDraft::Enum(selected));
            self.jobs.clear();
            self.pages.invalidate();
            self.details.clear();
            self.options.clear();
            self.clear_frozen();
            self.deadline = Some(ui.input(|i| i.time) + 0.25);
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
        if sign_out {
            self.disconnect();
            return;
        }
        if let Some(error) = self.notice.clone() {
            ui.horizontal(|ui| {
                ui.label(error);
                if ui.button("Retry").clicked() {
                    if self.invalid_query {
                        self.reset_query(false);
                    } else {
                        self.notice = None;
                    }
                    ui.ctx().request_repaint();
                }
            });
        }
        let ready = self.deadline.is_none()
            && self.notice.is_none()
            && !(self.frozen.pending() && !self.frozen.is_active())
            && self.pages.page(&self.address).is_some();
        let actions_ready = ready
            && !self.invalid_query
            && !self.dirty()
            && !self.frozen.is_active()
            && !self.frozen.pending();
        if let Some(error) = self.frozen.notice() {
            ui.label(error);
        }
        if self.frozen.is_active()
            && self.frozen.count().is_none()
            && !self.frozen.pending()
            && ui.button("重试计数").clicked()
            && let Some(session) = &self.session
            && let Err(error) = self.frozen.retry_count(session, ui.ctx(), unix_seconds())
        {
            self.state.message = Some(error);
        }
        if self.frozen.is_active() {
            ui.label("冻结选择")
                .on_hover_text("当前查看冻结值。清除勾选后返回实时查询。");
        }
        let can_prepare = ready && !self.dirty() && self.actions.available();
        let visible_columns: Vec<_> = self
            .columns
            .iter()
            .filter(|column| column.visible)
            .cloned()
            .collect();
        let cross_source =
            (self.state.selection.regions().len() == 1).then(|| self.state.selection.regions()[0]);
        let cross_fill_ready = can_prepare
            && !self.frozen.is_active()
            && !self.frozen.pending()
            && cross_source.is_some_and(|source| {
                source.top < source.bottom
                    && source.right <= visible_columns.len()
                    && source.left < source.right
                    && self
                        .pages
                        .page(&self.address)
                        .is_some_and(|page| source.bottom <= page.rows.len())
                    && visible_columns[source.left..source.right]
                        .iter()
                        .all(|column| {
                            !matches!(column.edit_kind, erp_grid_core::EditKind::ReadOnly)
                        })
            });
        if self.actions.available() {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(cross_fill_ready, egui::Button::new("跨页填充"))
                    .on_hover_text("以当前单个矩形选区为源，按当前查询与排序填充指定行范围")
                    .clicked()
                {
                    self.cross_page_fill_range.open();
                }
                if ui
                    .add_enabled(!self.drafts.busy(), egui::Button::new("商品草稿"))
                    .clicked()
                    && let Some(store) = &self.actions.store
                {
                    self.drafts.open(store, None, self.session_number, ui.ctx());
                }
                if ui
                    .add_enabled(
                        !self.drafts.busy() && self.details.draft_parent().is_some(),
                        egui::Button::new("SKU草稿"),
                    )
                    .clicked()
                    && let Some(store) = &self.actions.store
                {
                    self.drafts.open(
                        store,
                        self.details.draft_parent(),
                        self.session_number,
                        ui.ctx(),
                    );
                }
            });
            let columns = if self.drafts.parent().is_some() {
                self.details.draft_columns()
            } else {
                &self.columns
            };
            let restore_ready = ready && !self.dirty();
            let intent = self.session.as_ref().and_then(|session| {
                self.drafts
                    .ui(ui, session, self.session_number, restore_ready, columns)
            });
            match intent {
                Some(drafts::Intent::Save(parent)) => {
                    let cells = if parent.is_some() {
                        self.details.snapshot_current_draft()
                    } else {
                        self.snapshot_product_draft()
                    };
                    self.drafts.save(cells, ui.ctx());
                }
                Some(drafts::Intent::Apply(parent, preview)) => {
                    if let Some(parent) = parent {
                        if let Err(error) = self.details.restore_preview(parent, preview) {
                            self.drafts.fail(error);
                        }
                    } else if !self.dirty() {
                        self.paste = Some(paste::PasteReview::new(preview, &self.columns));
                    } else {
                        self.drafts.fail("请先结束当前编辑，再恢复草稿".into());
                    }
                }
                Some(drafts::Intent::ExpireSession) => {
                    self.actions.suspend();
                    if let Some(session) = &mut self.session {
                        session.expires = web_time::Instant::now();
                    }
                    ui.ctx().request_repaint();
                    return;
                }
                None => {}
            }
        } else {
            self.drafts.clear();
        }
        if let Some((first, last)) = self
            .cross_page_fill_range
            .show(ui.ctx(), egui::Id::new("product-cross-page-fill-range"))
        {
            let result = (|| {
                if !cross_fill_ready {
                    return Err("查询、选区或编辑状态已变化，请重新准备跨页填充".into());
                }
                let session = self.session.as_ref().ok_or("会话已失效")?;
                let query = self.query.as_ref().ok_or("当前查询尚未就绪")?;
                let source = cross_source.ok_or("请保留一个完整矩形选区")?;
                let page = self.pages.page(&self.address).ok_or("当前页尚未加载")?;
                let start = match &self.address {
                    PageAddress::Offset { start, .. } => *start,
                    PageAddress::Cursor { .. } => {
                        return Err("跨页填充需要页码分页".into());
                    }
                };
                cross_page_fill::CrossPageFill::start(
                    session,
                    query,
                    self.generation,
                    start,
                    source,
                    &page.rows,
                    &visible_columns,
                    first,
                    last,
                    ui.ctx(),
                )
            })();
            match result {
                Ok(fill) => self.cross_page_fill = Some(fill),
                Err(error) => self.state.message = Some(error),
            }
        }
        let mut close_cross_fill = false;
        if let Some(fill) = &mut self.cross_page_fill {
            egui::Window::new("准备跨页填充")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    if let Some(error) = fill.error() {
                        ui.label(error);
                    } else if fill.pending() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("正在读取冻结查询范围…");
                        });
                    }
                    if ui
                        .button(if fill.error().is_some() {
                            "关闭"
                        } else {
                            "取消"
                        })
                        .clicked()
                    {
                        close_cross_fill = true;
                    }
                });
        }
        if close_cross_fill {
            if let Some(fill) = &mut self.cross_page_fill {
                fill.cancel();
            }
            self.cross_page_fill = None;
        }
        let find_enabled = !self.dirty()
            && !self.invalid_query
            && self.deadline.is_none()
            && !self.composing
            && !(self.frozen.pending() && !self.frozen.is_active());
        let mut action_intent = actions::Intent::default();
        if self.advanced_tools || self.state.is_editing() || self.editing.is_some() {
            if self.views_storage_ready
                && let Some(current) = self.view_preferences()
            {
                let enabled = !self.dirty()
                    && !self.invalid_query
                    && self.deadline.is_none()
                    && !self.composing
                    && !self.frozen.is_active();
                if let Some(preferences) = self.views.ui(ui, &current, enabled) {
                    if let Err(error) = self.apply_view(preferences) {
                        self.notice = Some(error);
                    }
                    ui.ctx().request_repaint();
                    return;
                }
            }
            ui.horizontal_wrapped(|ui| {
        if ui
            .button(if self.exports.busy() {
                "导出任务 · 下载中"
            } else {
                "导出任务"
            })
            .on_hover_text("查看服务端导出任务并下载结果")
            .clicked()
        {
            self.exports.open();
        }
        if ui.add_enabled(self.query.is_some() && !self.dirty() && !self.invalid_query && self.deadline.is_none() && !self.composing && !self.frozen.is_active() && !self.exports.creating(), egui::Button::new("导出当前查询"))
            .on_hover_text("导出完整商品查询及排序，包含全部匹配行；数据以服务端任务开始时的快照为准。冻结勾选视图需先退出")
            .clicked()
            && let (Some(session),Some(query))=(&self.session,&self.query) {
                let request=PageRequest {ticket:0,generation:self.generation,query:query.clone(),address:PageAddress::Offset {start:0,limit:100}};
                self.exports.create(session,&request,ui.ctx());
            }
        let checked_export_ready = self.query.is_some()
            && !self.state.row_selection.checked().is_empty()
            && !self.dirty()
            && !self.invalid_query
            && self.deadline.is_none()
            && !self.composing
            && self.actions.actor().is_some()
            && !self.exports.creating();
        if !self.frozen.is_active() {
            if ui
                .add_enabled(checked_export_ready, egui::Button::new("导出勾选"))
                .on_hover_text("导出跨页保留的全部勾选记录")
                .clicked()
                && let (Some(session), Some(query)) = (&self.session, &self.query)
            {
                self.exports.create_checked(
                    session,
                    self.state.row_selection.checked(),
                    &query.sorts,
                    ui.ctx(),
                );
            }
        } else if ui
            .add_enabled(
                self.frozen.count().is_some()
                    && !self.frozen.pending()
                    && !self.dirty()
                    && !self.invalid_query
                    && self.deadline.is_none()
                    && !self.composing
                    && self.actions.actor().is_some()
                    && !self.exports.creating(),
                egui::Button::new("导出勾选"),
            )
            .on_hover_text("导出冻结的全部匹配记录并扣除已取消勾选项")
            .clicked()
            && let (Some(session), Some(credential), Some(query)) =
                (&self.session, self.frozen.credential(), &self.query)
        {
            self.exports.create_frozen(
                session,
                credential.token,
                self.frozen.exclusions(),
                &query.sorts,
                ui.ctx(),
            );
        }
        });
            let export_auth_failed = self
                .session
                .as_ref()
                .is_some_and(|session| self.exports.ui(ui, session, self.session_number));
            if export_auth_failed {
                self.exports.clear();
                if let Some(session) = &mut self.session {
                    session.expires = web_time::Instant::now();
                }
                self.actions.suspend();
                ui.ctx().request_repaint();
                return;
            }
            if ui
                .button("表内查找")
                .on_hover_text("Ctrl+F；查找完整查询结果，不改变筛选或勾选")
                .clicked()
            {
                self.find.open();
            }
            if let (Some(session), Some(query)) = (&self.session, &self.query) {
                let target = if let Some(credential) = self.frozen.credential() {
                    let frozen_query = erp_grid_core::paged_query::PageQuery {
                        expression: None,
                        schema: 1,
                        offset: 0,
                        limit: 100,
                        filters: std::collections::BTreeMap::new(),
                        sorts: query.sorts.clone(),
                        search: None,
                    };
                    frozen_query
                        .validate()
                        .map_err(str::to_owned)
                        .and_then(|_| {
                            serde_json::to_vec(&frozen_query)
                                .map_err(|_| "Cannot encode frozen selection find query".to_owned())
                        })
                        .map(|body| find::Target {
                            resource: format!("products-selection:{}", credential.token.to_uuid()),
                            endpoint: format!(
                                "/v1/tenants/{}/products/selections/{}/find",
                                session.tenant.to_uuid(),
                                credential.token.to_uuid()
                            ),
                            query: Ok(body),
                            window_id: egui::Id::new(("remote-find", "products")),
                        })
                } else {
                    let request = PageRequest {
                        ticket: 0,
                        generation: self.generation,
                        query: query.clone(),
                        address: PageAddress::Offset {
                            start: 0,
                            limit: 100,
                        },
                    };
                    crate::remote_products::encode_query(session.tenant, &request).map(|wire| {
                        find::Target {
                            resource: "products".into(),
                            endpoint: format!(
                                "/v1/tenants/{}/products/find",
                                session.tenant.to_uuid()
                            ),
                            query: Ok(wire.body),
                            window_id: egui::Id::new(("remote-find", "products")),
                        }
                    })
                };
                let found = self.find.ui(
                    ui,
                    session,
                    self.session_number,
                    target.unwrap_or_else(|error| find::Target {
                        resource: "products".into(),
                        endpoint: String::new(),
                        query: Err(error),
                        window_id: egui::Id::new(("remote-find", "products")),
                    }),
                    &self.columns,
                    find_enabled,
                );
                if found.expired {
                    if let Some(session) = &mut self.session {
                        session.expires = web_time::Instant::now();
                    }
                    ui.ctx().request_repaint();
                    return;
                }
                if found.selection_expired {
                    self.clear_frozen();
                    self.notice = Some("冻结选择已失效，请重建冻结选择".into());
                    ui.ctx().request_repaint();
                    return;
                }
                if let Some(hit) = found.hit.filter(|_| find_enabled) {
                    let limit = self.address.limit() as u16;
                    self.page_number = hit.offset / u64::from(limit);
                    self.address = PageAddress::Offset {
                        start: self.page_number * u64::from(limit),
                        limit,
                    };
                    self.pages.invalidate();
                    self.jobs.clear();
                    self.notice = None;
                    self.generation += 1;
                    self.state.rebind_page(self.generation, true);
                    self.find_pending = Some(hit);
                    ui.ctx().request_repaint();
                    return;
                }
            }
            if let Some(session) = &self.session {
                self.copy_batches.ui(
                    ui,
                    session,
                    self.session_number,
                    &self.frozen,
                    can_prepare,
                    self.actions.available(),
                );
            }
            if let Some(code) = self.copy_batches.take_location() {
                if self.dirty() {
                    self.state.message = Some("请先结束当前编辑，再查找记录".into());
                } else {
                    self.search = code;
                    self.reset_query(true);
                    ui.ctx().request_repaint();
                    return;
                }
            }
            action_intent = self
                .session
                .as_ref()
                .map(|session| {
                    self.actions.ui(
                        ui,
                        session,
                        actions_ready,
                        self.state.row_selection.target_count(),
                    )
                })
                .unwrap_or_default();
            if let Some(code) = action_intent.locate {
                if self.dirty() {
                    self.state.message =
                        Some("Finish or cancel this edit before locating another product".into());
                    return;
                }
                self.search = code;
                self.reset_query(true);
                ui.ctx().request_repaint();
                return;
            }
            if !self.frozen.is_active() && !self.frozen.pending() {
                self.edit_tools(ui);
            }
        }
        if self.actions.available() && !self.frozen.is_active() && !self.frozen.pending() {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.state.row_selection.active.is_some() && !self.dirty(),
                        egui::Button::new("编辑商品"),
                    )
                    .clicked()
                {
                    self.inspector.open = true;
                }
            });
        }
        if let Some(session) = &self.session
            && self
                .paste
                .as_mut()
                .is_some_and(|review| review.ui(ui, &mut self.actions, session, None))
        {
            self.paste = None;
            self.reset_query(false);
        }
        if let Some(message) = &self.state.message {
            ui.label(message);
        }
        self.state.read_only = !ready
            || !self.actions.available()
            || self.frozen.is_active()
            || self.frozen.pending()
            || self.details.dirty()
            || self.copy_batches.dirty()
            || self.drafts.busy()
            || self.paste.is_some();
        self.state.fill_enabled = !self.state.read_only;
        self.state.filtering_enabled = !self.dirty();
        self.state.sorting_enabled =
            !self.dirty() && !(self.frozen.pending() && !self.frozen.is_active());
        let full_area = ui.available_rect_before_wrap();
        let detail_height = if self.workbench.preferences.bottom_open
            && (full_area.height() >= 400.0 || self.details.dirty())
        {
            self.workbench
                .preferences
                .bottom_height
                .min((full_area.height() - 240.0).max(170.0))
        } else {
            0.0
        };
        let area = egui::Rect::from_min_max(
            full_area.min,
            egui::pos2(full_area.right(), full_area.bottom() - detail_height),
        );
        let footer = egui::Rect::from_min_max(
            egui::pos2(area.left(), (area.bottom() - 26.0).max(area.top())),
            area.max,
        );
        let page = ready.then(|| self.pages.page(&self.address)).flatten();
        let rows = page.map_or(&[][..], |page| page.rows.as_slice());
        let inspector_width = if self.inspector.open {
            self.workbench
                .preferences
                .right_width
                .min((area.width() - 240.0).max(180.0))
        } else {
            0.0
        };
        let grid = egui::Rect::from_min_max(
            area.min,
            egui::pos2(area.right() - inspector_width, footer.top()),
        );
        let active_row = self
            .state
            .row_selection
            .active
            .and_then(|id| rows.iter().find(|row| row.id == id));
        self.inspector.select(active_row);
        self.inspector.sync(active_row);
        if self.frozen.is_active() && ready {
            self.state.row_selection.clear_checked();
            for row in rows {
                self.state
                    .row_selection
                    .set_checked(row.id, !self.frozen.exclusions().contains(&row.id));
            }
        }
        if ready {
            self.displayed_ids = rows.iter().map(|row| row.id).collect();
        }
        if ready && let Some(hit) = self.find_pending.take() {
            let local = (hit.offset % self.address.limit() as u64) as usize;
            let column = self
                .columns
                .iter()
                .filter(|column| column.visible)
                .position(|column| column.id == hit.column);
            if rows.get(local).is_some_and(|row| row.id == hit.row_id)
                && let Some(column) = column
            {
                self.state.row_selection.active = Some(hit.row_id);
                self.state
                    .request_focus(erp_grid_core::Cell { row: local, column });
            } else {
                self.state.message = Some("数据已变化，请再次查找定位".into());
            }
        }
        self.state.row_number_start = self.page_number * self.address.limit() as u64;
        let checked_before = self
            .frozen
            .is_active()
            .then(|| self.state.row_selection.checked().clone());
        let active_before = self.state.row_selection.active;
        let output = ui
            .scope_builder(egui::UiBuilder::new().max_rect(grid), |ui| {
                DataGrid {
                    id: egui::Id::new("remote-products"),
                    columns: &mut self.columns,
                    rows: rows.len(),
                    state: &mut self.state,
                    row_id: Some(&|index| rows[index].id),
                }
                .show(ui, |row, column| {
                    rows[row].cells.get(&column).cloned().unwrap_or_default()
                })
            })
            .inner;
        let mut inspector_changes = None;
        if self.inspector.open {
            let rect = egui::Rect::from_min_max(
                egui::pos2(grid.right() + 8.0, area.top()),
                egui::pos2(area.right(), footer.top()),
            );
            let splitter = egui::Rect::from_min_max(
                egui::pos2(grid.right() - 4.0, area.top()),
                egui::pos2(grid.right() + 4.0, footer.top()),
            );
            let response = ui.interact(
                splitter,
                ui.id().with("workbench-right-splitter"),
                egui::Sense::drag(),
            );
            if response.dragged() {
                self.workbench.preferences.right_width = (self.workbench.preferences.right_width
                    - response.drag_delta().x)
                    .clamp(180.0, (area.width() - 240.0).max(180.0));
                self.workbench.mark_dirty();
                ui.ctx().request_repaint();
            }
            let allow = ready
                && !self.state.is_editing()
                && self.editing.is_none()
                && self.paste.is_none()
                && self.cross_page_fill.is_none()
                && !self.drafts.busy();
            let changes = ui
                .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| self.inspector.ui(ui, allow))
                        .inner
                })
                .inner;
            inspector_changes = changes;
        }
        if ready && let Some(completed_at) = self.render_pending.take() {
            self.render_elapsed = Some(completed_at.elapsed());
        }
        if self.details.dirty() {
            self.state.row_selection.active = active_before;
        }
        if self.frozen.is_active()
            && ready
            && checked_before
                .as_ref()
                .is_some_and(|before| before != self.state.row_selection.checked())
            && let Some(session) = &self.session
        {
            let ids: Vec<_> = rows.iter().map(|row| row.id).collect();
            if let Err(error) = self.frozen.set_page_checked(
                session,
                &ids,
                self.state.row_selection.checked(),
                ui.ctx(),
                unix_seconds(),
            ) {
                self.state.message = Some(error);
            }
        }
        let total = page.and_then(|page| match page.total {
            RowCount::Exact(total) => Some(total),
            RowCount::Unknown => None,
        });
        let pages = total.map(|total| total.div_ceil(self.address.limit() as u64).max(1));
        let loaded = rows.len();
        let mut next_page = false;
        let mut previous = false;
        let mut first = false;
        let mut last = false;
        let mut requested_page = self.page_number + 1;
        let mut limit = self.address.limit() as u16;
        let editor_open = self.dirty();
        ui.scope_builder(egui::UiBuilder::new().max_rect(footer), |ui| {
            ui.add_enabled_ui(!editor_open, |ui| {
                ui.horizontal(|ui| {
                    ui.label(if self.frozen.is_active() {
                        self.frozen
                            .count()
                            .map_or_else(|| "Checked —".into(), |count| format!("Checked {count}"))
                    } else {
                        format!("Checked {}", self.state.row_selection.checked().len())
                    });
                    if ready {
                        let timing = |value: Option<Duration>| value.map_or_else(|| "—".into(), |duration| format!("{:.1} ms", duration.as_secs_f64() * 1000.0));
                        ui.label(format!(
                            "Loaded {loaded} · {:.1} ms",
                            self.elapsed.unwrap_or_default().as_secs_f64() * 1000.0
                        )).on_hover_text(format!(
                            "Transport: {}\nCompletion to UI poll: {}\nCompletion to first grid draw: {}\nDraw timing excludes GPU presentation and OS input",
                            timing(self.elapsed),
                            timing(self.delivery_elapsed),
                            timing(self.render_elapsed),
                        ));
                    } else if self.notice.is_none() {
                        ui.spinner();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            total
                                .map_or_else(|| "Total —".into(), |total| format!("Total {total}")),
                        );
                        egui::ComboBox::from_id_salt("remote-page-size")
                            .selected_text(format!("{limit} / page"))
                            .show_ui(ui, |ui| {
                                for count in [10, 20, 50, 100, 200, 500] {
                                    ui.selectable_value(&mut limit, count, count.to_string());
                                }
                            });
                        last = ui
                            .add_enabled(
                                ready && pages.is_some_and(|pages| self.page_number + 1 < pages),
                                egui::Button::new(">|"),
                            )
                            .clicked();
                        next_page = ui
                            .add_enabled(
                                ready && pages.is_some_and(|pages| self.page_number + 1 < pages),
                                egui::Button::new(">"),
                            )
                            .clicked();
                        ui.add_enabled(
                            ready,
                            egui::DragValue::new(&mut requested_page)
                                .range(1..=pages.unwrap_or(self.page_number + 1))
                                .prefix("Page "),
                        );
                        previous = ui
                            .add_enabled(ready && self.page_number > 0, egui::Button::new("<"))
                            .clicked();
                        first = ui
                            .add_enabled(self.page_number > 0, egui::Button::new("|<"))
                            .clicked();
                    });
                });
            });
        });
        if detail_height > 0.0
            && let Some(session) = &self.session
        {
            let parent = ready.then_some(self.state.row_selection.active).flatten();
            let rect = egui::Rect::from_min_max(
                egui::pos2(full_area.left(), area.bottom()),
                full_area.max,
            );
            let allow_detail_edit = !self.state.is_editing()
                && self.editing.is_none()
                && self.paste.is_none()
                && self.cross_page_fill.is_none()
                && !self.copy_batches.dirty();
            let allow_detail_edit = allow_detail_edit && !self.drafts.busy();
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                self.details.ui(
                    ui,
                    session,
                    self.session_number,
                    parent,
                    &mut self.actions,
                    allow_detail_edit,
                )
            });
        } else {
            self.details.clear();
        }
        if detail_height > 0.0 {
            let handle = egui::Rect::from_min_max(
                egui::pos2(full_area.left(), area.bottom() - 3.0),
                egui::pos2(full_area.right(), area.bottom() + 3.0),
            );
            let response = ui
                .interact(
                    handle,
                    ui.id().with("product-detail-height"),
                    egui::Sense::drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
            if (response.dragged() || response.drag_stopped())
                && let Some(pointer) = response.interact_pointer_pos()
            {
                self.workbench.preferences.bottom_height = (full_area.bottom() - pointer.y)
                    .clamp(170.0, (full_area.height() - 240.0).max(170.0));
                self.workbench.mark_dirty();
                ui.ctx().request_repaint();
            }
        }
        if self.details.take_authentication_failed() {
            if let Some(session) = self.session.as_mut() {
                session.expires = web_time::Instant::now();
            }
            self.notice = Some("Session expired or SKU access denied".into());
            ui.ctx().request_repaint();
            return;
        }
        let find_shortcut =
            ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
        if find_shortcut && !self.details.find_open() && find_enabled {
            self.advanced_tools = true;
            self.find.open();
        }
        if let Some(changes) = inspector_changes {
            self.begin_inspector_save(changes);
        }
        self.observe_editor();
        self.submit_editor(output.edits, output.edit_ticket, output.edit_navigation);
        if self.resume_editor.is_some() || self.state.is_editing() {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
        if let Some((start, text)) = output.paste {
            let columns: Vec<_> = self
                .columns
                .iter()
                .filter(|column| column.visible)
                .cloned()
                .collect();
            if let Some(page) = self.pages.page(&self.address) {
                match crate::remote_paste::plan(
                    "products.update",
                    start,
                    &text,
                    &columns,
                    &page.rows,
                ) {
                    Ok(preview) if preview.edits.is_empty() => {
                        self.state.message = Some("粘贴内容没有变化".into())
                    }
                    Ok(preview) => self.paste = Some(paste::PasteReview::new(preview, &columns)),
                    Err(error) => self.state.message = Some(error),
                }
            }
        }
        if let Some(fill) = output.fill {
            let columns: Vec<_> = self
                .columns
                .iter()
                .filter(|column| column.visible)
                .cloned()
                .collect();
            if let Some(page) = self.pages.page(&self.address) {
                match crate::remote_paste::plan_fill(
                    "products.update",
                    fill.source,
                    fill.target,
                    &columns,
                    &page.rows,
                ) {
                    Ok(preview) if preview.edits.is_empty() => {
                        self.state.message = Some("填充内容没有变化".into())
                    }
                    Ok(preview) => self.paste = Some(paste::PasteReview::new(preview, &columns)),
                    Err(error) => self.state.message = Some(error),
                }
            }
        }
        let size_changed = limit != self.address.limit() as u16;
        let page_input_changed = ready && requested_page != self.page_number + 1;
        if output.quick_search_changed {
            self.options.clear();
            self.details.clear();
            self.jobs.clear();
            self.pages.invalidate();
            self.clear_frozen();
            self.deadline = Some(ui.input(|i| i.time) + 0.25);
            self.notice = None;
            ui.ctx().request_repaint_after(Duration::from_millis(250));
            return;
        }
        if output.sorts_changed {
            self.reset_query(false);
            ui.ctx().request_repaint();
            return;
        }
        if size_changed {
            self.address = PageAddress::Offset { start: 0, limit };
            self.reset_query(false);
        } else if next_page || previous || first || last || page_input_changed {
            self.page_number = if first {
                0
            } else if last {
                pages.unwrap_or(1) - 1
            } else if next_page {
                self.page_number + 1
            } else if previous {
                self.page_number.saturating_sub(1)
            } else {
                requested_page.saturating_sub(1)
            };
            self.address = PageAddress::Offset {
                start: self.page_number * u64::from(limit),
                limit,
            };
            self.generation += 1;
            self.state.rebind_page(self.generation, true);
            self.elapsed = None;
            self.render_pending = None;
            self.render_elapsed = None;
            self.delivery_elapsed = None;
        }
        if next_page || previous || first || last || size_changed || page_input_changed {
            ui.ctx().request_repaint();
        }
        if action_intent.copy
            && ready
            && !self.frozen.is_active()
            && !self.frozen.pending()
            && !size_changed
            && !page_input_changed
            && !next_page
            && !previous
            && !first
            && !last
        {
            self.actions.copy(&self.state.row_selection);
            ui.ctx().request_repaint();
        }
    }

    fn begin_inspector_save(&mut self, changes: Vec<(ColumnId, String, String)>) {
        let Some(row) = self.inspector.selected() else {
            return;
        };
        let Some((column, before, after)) = self.inspector.begin_queue(changes) else {
            return;
        };
        self.begin_inspector_step(row, column, before, after);
    }

    fn begin_inspector_step(
        &mut self,
        row: RowId,
        column: ColumnId,
        before: String,
        after: String,
    ) {
        if self.state.is_editing() || self.editing.is_some() {
            return;
        }
        let Some(page) = self.pages.page(&self.address) else {
            self.state.message = Some("当前商品页尚未加载".into());
            return;
        };
        let Some(row_index) = page.rows.iter().position(|item| item.id == row) else {
            self.state.message = Some("商品已离开当前页，请重新选择".into());
            return;
        };
        let Some(column_index) = self
            .columns
            .iter()
            .filter(|item| item.visible)
            .position(|item| item.id == column)
        else {
            self.state.message = Some("编辑字段当前不可见".into());
            return;
        };
        if self
            .state
            .restore_edit(
                erp_grid_core::Cell {
                    row: row_index,
                    column: column_index,
                },
                row,
                column,
                before,
                after,
            )
            .is_err()
        {
            self.state.message = Some("当前仍有未完成编辑".into());
            return;
        }
        self.observe_editor();
        if let Some(version) = self.inspector.version()
            && let Some(editing) = &mut self.editing
        {
            editing.observed.version = Some(version);
        }
        let Some(definition) = self
            .columns
            .iter()
            .find(|column_def| column_def.id == column)
        else {
            return;
        };
        match self.state.prepare_edit_commit(definition) {
            Ok(Some((edit, ticket))) => self.submit_editor(vec![edit], Some(ticket), None),
            Ok(None) => {}
            Err(error) => self.state.message = Some(error),
        }
    }
}

fn unix_seconds() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod edit_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod selection_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
