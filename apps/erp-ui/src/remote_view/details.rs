use super::*;
use erp_grid_core::{Pagination, paged_query::PageQuery};
use erp_ui_egui::show_pagination;
use std::collections::BTreeMap;
#[path = "details_editing.rs"]
pub(super) mod editing;
#[path = "details_fill.rs"]
mod fill;
#[path = "details_views.rs"]
mod views;

pub(super) struct Details {
    parent: Option<(u64, RowId)>,
    pages: RemotePages,
    pending: Option<(PageRequest, HttpTask)>,
    pager: Pagination,
    state: GridState,
    columns: Vec<Column>,
    search: String,
    deadline: Option<f64>,
    composing: bool,
    generation: u64,
    error: Option<String>,
    elapsed: Option<Duration>,
    total: Option<usize>,
    editing: Option<editing::SkuEditing>,
    paste: Option<super::paste::PasteReview>,
    find: super::find::Find,
    find_pending: Option<super::find::Hit>,
    views: views::SkuViews,
    fill_range: super::cross_page_fill::CrossPageFillRange,
    cross_fill: Option<fill::DetailsFill>,
    authentication_failed: bool,
}
impl Default for Details {
    fn default() -> Self {
        let mut state = GridState::default();
        state.metrics = erp_ui_egui::GridMetrics::legacy();
        state.read_only = true;
        state.quick_search_open = true;
        let columns = [
            "Code", "Barcode", "Name", "Kind", "Cost", "Active", "Status", "Created", "Updated",
        ]
        .into_iter()
        .enumerate()
        .map(|(i, title)| {
            let filter = match i {
                4 => FilterKind::Money,
                5 => FilterKind::Enum(vec!["false".into(), "true".into()]),
                7 | 8 => FilterKind::DateTime,
                _ => FilterKind::Text,
            };
            let mut column = Column::new(
                ColumnId(i as u64),
                title,
                if i == 2 { 220.0 } else { 145.0 },
            )
            .with_filter(filter);
            column.edit_kind = match i {
                0 => erp_grid_core::EditKind::Text,
                1 | 2 | 4 => erp_grid_core::EditKind::OptionalText,
                5 => erp_grid_core::EditKind::Choice(vec!["true".into(), "false".into()]),
                _ => erp_grid_core::EditKind::ReadOnly,
            };
            column
        })
        .collect();
        Self {
            parent: None,
            pages: RemotePages::default(),
            pending: None,
            pager: Pagination::default(),
            state,
            columns,
            search: String::new(),
            deadline: None,
            composing: false,
            generation: 0,
            error: None,
            elapsed: None,
            total: None,
            editing: None,
            paste: None,
            find: Default::default(),
            find_pending: None,
            views: Default::default(),
            fill_range: fill::range_ui(),
            cross_fill: None,
            authentication_failed: false,
        }
    }
}
impl Details {
    pub(super) fn suspend_page_queries(&mut self) {
        self.pending = None;
        self.pages.invalidate();
        self.composing = false;
        self.find.clear();
        self.find_pending = None;
    }
    pub(super) fn draft_parent(&self) -> Option<RowId> {
        self.parent.map(|(_, parent)| parent)
    }

    pub(super) fn draft_columns(&self) -> &[Column] {
        &self.columns
    }

    pub(super) fn restore_preview(
        &mut self,
        parent: RowId,
        preview: crate::remote_paste::Preview,
    ) -> Result<(), String> {
        if self.draft_parent() != Some(parent) {
            return Err("草稿所属父记录不匹配".into());
        }
        if self.dirty() || self.pending.is_some() {
            return Err("当前仍有未完成编辑或粘贴操作".into());
        }
        self.paste = Some(super::paste::PasteReview::new(preview, &self.columns));
        Ok(())
    }

    fn address(&self) -> PageAddress {
        let range = self.pager.range(usize::MAX);
        PageAddress::Offset {
            start: range.start as u64,
            limit: self.pager.page_size() as u16,
        }
    }
    pub(super) fn clear(&mut self) {
        if self.dirty() {
            self.pending = None;
            self.state.hold_edit();
            return;
        }
        self.parent = None;
        self.reset(true);
        self.search.clear();
        self.state.quick_search.clear();
        self.composing = false;
        self.find.clear();
        self.find_pending = None;
    }
    fn reset(&mut self, clear_checks: bool) {
        self.cross_fill = None;
        self.fill_range.open = false;
        self.pending = None;
        self.find_pending = None;
        self.pages.clear();
        if clear_checks {
            self.pager.set_page(0, 0);
            self.total = None;
        }
        self.deadline = None;
        self.error = None;
        self.elapsed = None;
        self.generation = self.generation.wrapping_add(1);
        if clear_checks {
            self.state.bind_view(self.generation);
        } else {
            self.state.rebind_page(self.generation, true);
        }
    }
    fn query(&self, tenant: RowId, parent: RowId, generation: u64) -> Result<RemoteQuery, String> {
        let mut filters = BTreeMap::new();
        for column in &self.columns {
            if let Some(draft) = self.state.quick_search.get(&column.id)
                && let Some(filter) = draft.compile(&column.filter).map_err(str::to_owned)?
            {
                filters.insert(column.id, filter);
            }
        }
        Ok(RemoteQuery {
            expression: None,
            scope: DataScope {
                source: format!("product-skus:{}:{}", tenant.to_uuid(), parent.to_uuid()),
                schema: 1,
                authorization: format!("session:{generation}"),
            },
            filters,
            sorts: self.state.sorts.clone(),
            search: (!self.search.trim().is_empty()).then(|| self.search.trim().to_owned()),
        })
    }
    fn tick(&mut self, session: &Session, ctx: &egui::Context) {
        let Some((generation, parent)) = self.parent else {
            return;
        };
        if let Some(result) = self.pending.as_mut().and_then(|(_, task)| task.poll()) {
            let completed_duration = self
                .pending
                .as_ref()
                .and_then(|(_, task)| task.completed_duration());
            let Some((request, _)) = self.pending.take() else {
                return;
            };
            let result = result
                .map_err(|e| e.to_string())
                .and_then(|bytes| decode(session.tenant, parent, &bytes, &request.address));
            let total = result.as_ref().ok().and_then(|page| match page.total {
                RowCount::Exact(total) => usize::try_from(total).ok(),
                RowCount::Unknown => None,
            });
            match self.pages.accept(&request, result) {
                PageAcceptance::Stored => {
                    self.total = total;
                    self.elapsed = completed_duration;
                }
                PageAcceptance::Failed(e) => self.error = Some(e),
                PageAcceptance::Stale => {}
            }
        }
        if self.deadline.is_some_and(|d| ctx.input(|i| i.time) >= d) && !self.composing {
            self.reset(true);
        }
        if self.deadline.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        }
        if self.error.is_some() || self.pending.is_some() {
            return;
        }
        let query = match self.query(session.tenant, parent, generation) {
            Ok(q) => q,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        self.pages.bind(query);
        let address = self.address();
        let request = match self.pages.start(address) {
            Ok(PageStart::Fetch(request)) => request,
            Ok(_) => return,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
        let offset = match &request.address {
            PageAddress::Offset { start, .. } => *start,
            _ => 0,
        };
        let body = PageQuery {
            schema: request.query.scope.schema,
            offset,
            limit: request.address.limit() as u16,
            filters: request.query.filters.clone(),
            sorts: request.query.sorts.clone(),
            search: request.query.search.clone(),
            expression: request.query.expression.as_deref().cloned(),
        };
        if let Err(error) = body.validate() {
            self.pages.cancel(request.ticket);
            self.error = Some(error.into());
            return;
        }
        let body = match serde_json::to_vec(&body) {
            Ok(body) => body,
            Err(_) => {
                self.error = Some("Cannot encode SKU query".into());
                self.pages.cancel(request.ticket);
                return;
            }
        };
        match session.client.start(
            HttpRequest {
                path: format!(
                    "/v1/tenants/{}/products/{}/skus/query",
                    session.tenant.to_uuid(),
                    parent.to_uuid()
                ),
                query: vec![],
                json: Some(body),
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            ctx,
        ) {
            Ok(task) => self.pending = Some((request, task)),
            Err(e) => {
                self.pages.cancel(request.ticket);
                self.error = Some(e.to_string());
            }
        }
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &Session,
        generation: u64,
        parent: Option<RowId>,
        actions: &mut super::actions::ProductActions,
        allow_edit: bool,
    ) {
        self.finish_edit(actions);
        let identity = parent.map(|p| (generation, p));
        if identity != self.parent {
            self.cross_fill = None;
            self.fill_range.open = false;
        }
        if identity != self.parent && !self.dirty() {
            self.clear();
            self.parent = identity;
        }
        ui.input(|input| {
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
        self.tick(session, ui.ctx());
        ui.horizontal(|ui| {
            ui.label("SKUs");
            if ui
                .add_enabled(
                    parent.is_some() && !self.dirty(),
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search")
                        .char_limit(4096)
                        .desired_width(210.0),
                )
                .changed()
            {
                self.deadline = Some(ui.input(|i| i.time) + 0.25);
                self.pending = None;
                self.pages.clear();
                self.total = None;
                self.pager.set_page(0, 0);
                self.generation = self.generation.wrapping_add(1);
                self.state.bind_view(self.generation);
                ui.ctx().request_repaint_after(Duration::from_millis(250));
            }
            if ui
                .add_enabled(
                    parent.is_some() && !self.dirty(),
                    egui::Button::new("Refresh SKUs"),
                )
                .clicked()
            {
                self.reset(false);
                ui.ctx().request_repaint();
            }
            if ui
                .add_enabled(
                    parent.is_some() && !self.dirty(),
                    egui::Button::new("表内查找"),
                )
                .on_hover_text("Ctrl+F；查找当前商品的完整 SKU 查询结果")
                .clicked()
            {
                self.find.open();
            }
            self.views_ui(ui, session, parent);
            let can_fill = actions.available()
                && allow_edit
                && self.cross_fill.is_none()
                && self.paste.is_none()
                && self.editing.is_none()
                && self.pending.is_none()
                && self.deadline.is_none()
                && self.error.is_none()
                && self.state.selection.regions().len() == 1;
            if ui
                .add_enabled(can_fill, egui::Button::new("跨页填充"))
                .clicked()
            {
                self.fill_range.open();
            }
            if let Some(e) = &self.error {
                ui.label(e);
            }
        });
        if let Some((first, last)) = self
            .fill_range
            .show(ui.ctx(), egui::Id::new("sku-cross-page-fill-range"))
        {
            let ready = actions.available()
                && allow_edit
                && self.cross_fill.is_none()
                && self.paste.is_none()
                && self.editing.is_none()
                && self.pending.is_none()
                && self.deadline.is_none()
                && self.error.is_none()
                && self.state.selection.regions().len() == 1;
            if ready {
                if let (Some((_, parent)), Some(page)) =
                    (self.parent, self.pages.page(&self.address()))
                {
                    let columns = self
                        .columns
                        .iter()
                        .filter(|column| column.visible)
                        .cloned()
                        .collect::<Vec<_>>();
                    let source = self.state.selection.regions()[0];
                    match self
                        .query(session.tenant, parent, self.generation)
                        .and_then(|query| {
                            let page_start = match self.address() {
                                PageAddress::Offset { start, .. } => start,
                                PageAddress::Cursor { .. } => 0,
                            };
                            fill::DetailsFill::start(
                                session,
                                parent,
                                self.generation,
                                query,
                                page_start,
                                source,
                                &page.rows,
                                &columns,
                                first,
                                last,
                                ui.ctx(),
                            )
                        }) {
                        Ok(fill) => self.cross_fill = Some(fill),
                        Err(error) => self.state.message = Some(error),
                    }
                }
            } else {
                self.state.message = Some("SKU 状态已变化，请重新打开跨页填充".into());
            }
        }
        let mut close_fill = false;
        let mut fill_preview = None;
        let current_fill_query = self.parent.and_then(|(_, parent)| {
            self.query(session.tenant, parent, self.generation)
                .ok()
                .map(|query| (parent, query))
        });
        if let (Some((_, parent)), Some(fill)) = (self.parent, self.cross_fill.as_mut()) {
            if let Some((_, query)) = &current_fill_query {
                fill.poll(session, parent, self.generation, query);
            } else {
                fill.cancel();
                close_fill = true;
                self.state.message = Some("SKU 查询无效，已取消跨页填充".into());
            }
            if fill.authentication_failed() {
                self.authentication_failed = true;
                self.error = Some("Session expired; sign in again".into());
            }
            if let Some(error) = fill.error() {
                self.state.message = Some(error.into());
                close_fill = true;
            }
            if let Some((preview, columns)) = fill.take_preview() {
                fill_preview = Some((preview, columns));
                close_fill = true;
            } else if fill.busy() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    if ui.button("取消跨页填充").clicked() {
                        fill.cancel();
                        close_fill = true;
                    }
                });
            }
        }
        if close_fill {
            self.cross_fill = None;
        }
        if let Some((preview, columns)) = fill_preview {
            self.paste = Some(super::paste::PasteReview::new(preview, &columns));
        }
        let find_enabled = parent.is_some()
            && !self.dirty()
            && self.deadline.is_none()
            && !self.composing
            && self.error.is_none();
        if ui.rect_contains_pointer(ui.max_rect())
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::F))
            && find_enabled
        {
            self.find.open();
        }
        if let Some((_, parent)) = self.parent {
            let query = self
                .query(session.tenant, parent, self.generation)
                .and_then(|query| {
                    let request = PageRequest {
                        ticket: 0,
                        generation: self.generation,
                        query,
                        address: PageAddress::Offset {
                            start: 0,
                            limit: 100,
                        },
                    };
                    encode_page_query(&request)
                });
            let found = self.find.ui(
                ui,
                session,
                generation,
                super::find::Target {
                    resource: format!("product-skus:{}", parent.to_uuid()),
                    endpoint: format!(
                        "/v1/tenants/{}/products/{}/skus/find",
                        session.tenant.to_uuid(),
                        parent.to_uuid()
                    ),
                    query,
                    window_id: egui::Id::new(("remote-find", "skus", parent.0)),
                },
                &self.columns,
                find_enabled,
            );
            if found.expired {
                self.error = Some("Session expired; sign in again".into());
            } else if let Some(hit) = found.hit.filter(|_| find_enabled) {
                let page = usize::try_from(hit.offset / self.pager.page_size() as u64)
                    .unwrap_or(usize::MAX);
                self.pager.set_page(page, usize::MAX);
                self.reset(false);
                self.find_pending = Some(hit);
                ui.ctx().request_repaint();
                return;
            }
        }
        self.edit_tools(ui, session);
        if self.paste.as_mut().is_some_and(|review| {
            review.ui(ui, actions, session, self.parent.map(|(_, parent)| parent))
        }) {
            self.paste = None;
            self.reset(false);
        }
        self.state.read_only = !actions.available()
            || !allow_edit
            || self.pending.is_some()
            || self.deadline.is_some()
            || self.error.is_some()
            || self.paste.is_some()
            || self.cross_fill.is_some()
            || self.fill_range.open;
        self.state.fill_enabled = !self.state.read_only;
        self.state.filtering_enabled = !self.dirty();
        self.state.sorting_enabled = !self.dirty();
        let address = self.address();
        let page = self.pages.page(&address);
        let rows = page.map_or(&[][..], |p| p.rows.as_slice());
        if page.is_some()
            && let Some(hit) = self.find_pending.take()
        {
            if let Some((local, column)) =
                verified_focus(rows, &self.columns, &hit, self.pager.page_size())
            {
                self.state.row_selection.active = Some(hit.row_id);
                self.state
                    .request_focus(erp_grid_core::Cell { row: local, column });
            } else {
                self.state.message = Some("数据已变化，请再次查找定位".into());
            }
        }
        let total = self.total.unwrap_or(0);
        let area = ui.available_rect_before_wrap();
        let footer = egui::Rect::from_min_max(
            egui::pos2(area.left(), (area.bottom() - 30.0).max(area.top())),
            area.max,
        );
        self.state.row_number_start = self.pager.range(total).start as u64;
        let output = ui
            .scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                    area.min,
                    egui::pos2(area.right(), footer.top()),
                )),
                |ui| {
                    DataGrid {
                        id: egui::Id::new("remote-product-skus"),
                        columns: &mut self.columns,
                        rows: rows.len(),
                        state: &mut self.state,
                        row_id: Some(&|i| rows[i].id),
                    }
                    .show(ui, |i, c| {
                        rows[i].cells.get(&c).cloned().unwrap_or_default()
                    })
                },
            )
            .inner;
        self.observe_edit();
        self.submit_edit(output.edits, output.edit_ticket, actions);
        if let Some((start, text)) = output.paste {
            let columns: Vec<_> = self
                .columns
                .iter()
                .filter(|column| column.visible)
                .cloned()
                .collect();
            if let Some(page) = self.pages.page(&address) {
                match crate::remote_paste::plan("skus.update", start, &text, &columns, &page.rows) {
                    Ok(preview) if preview.edits.is_empty() => {
                        self.state.message = Some("粘贴内容没有变化".into())
                    }
                    Ok(preview) => {
                        self.paste = Some(super::paste::PasteReview::new(preview, &columns))
                    }
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
            if let Some(page) = self.pages.page(&address) {
                match crate::remote_paste::plan_fill(
                    "skus.update",
                    fill.source,
                    fill.target,
                    &columns,
                    &page.rows,
                ) {
                    Ok(preview) if preview.edits.is_empty() => {
                        self.state.message = Some("填充内容没有变化".into())
                    }
                    Ok(preview) => {
                        self.paste = Some(super::paste::PasteReview::new(preview, &columns))
                    }
                    Err(error) => self.state.message = Some(error),
                }
            }
        }
        if self.dirty() {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
        if output.quick_search_changed {
            self.pending = None;
            self.pages.clear();
            self.total = None;
            self.pager.set_page(0, 0);
            self.deadline = Some(ui.input(|i| i.time) + 0.25);
            self.generation = self.generation.wrapping_add(1);
            self.state.bind_view(self.generation);
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
        if output.sorts_changed {
            self.pending = None;
            self.pages.clear();
            self.total = None;
            self.pager.set_page(0, 0);
            self.generation = self.generation.wrapping_add(1);
            self.state.rebind_page(self.generation, true);
            ui.ctx().request_repaint();
        }
        let can_page = self.pending.is_none()
            && self.deadline.is_none()
            && self.error.is_none()
            && !self.dirty();
        let changed = ui
            .scope_builder(egui::UiBuilder::new().max_rect(footer), |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Checked {}",
                        self.state.row_selection.checked().len()
                    ));
                    if let Some(elapsed) = self.elapsed {
                        ui.label(format!("{:.1} ms", elapsed.as_secs_f64() * 1000.0));
                    }
                    if self.pending.is_some() {
                        ui.spinner();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        show_pagination(
                            ui,
                            egui::Id::new("sku-pagination"),
                            &mut self.pager,
                            total,
                            can_page,
                        )
                    })
                    .inner
                })
                .inner
            })
            .inner;
        if changed {
            self.pending = None;
            self.pages.clear();
            self.generation = self.generation.wrapping_add(1);
            self.state.rebind_page(self.generation, true);
            ui.ctx().request_repaint();
        }
    }

    pub(super) fn find_open(&self) -> bool {
        self.find.is_open()
    }

    pub(super) fn take_authentication_failed(&mut self) -> bool {
        std::mem::take(&mut self.authentication_failed)
    }
}

fn encode_page_query(request: &PageRequest) -> Result<Vec<u8>, String> {
    let PageAddress::Offset { start, limit } = &request.address else {
        return Err("SKU grid API requires offset paging".into());
    };
    let query = PageQuery {
        schema: request.query.scope.schema,
        offset: *start,
        limit: *limit,
        filters: request.query.filters.clone(),
        sorts: request.query.sorts.clone(),
        search: request.query.search.clone(),
        expression: request.query.expression.as_deref().cloned(),
    };
    query.validate().map_err(str::to_owned)?;
    serde_json::to_vec(&query).map_err(|_| "Cannot encode SKU query".into())
}

fn verified_focus(
    rows: &[RemoteRow],
    columns: &[Column],
    hit: &super::find::Hit,
    page_size: usize,
) -> Option<(usize, usize)> {
    let local = usize::try_from(hit.offset % page_size as u64).ok()?;
    let column = columns
        .iter()
        .filter(|column| column.visible)
        .position(|column| column.id == hit.column)?;
    rows.get(local)
        .is_some_and(|row| row.id == hit.row_id)
        .then_some((local, column))
}
pub(super) fn decode(
    tenant: RowId,
    parent: RowId,
    bytes: &[u8],
    address: &PageAddress,
) -> Result<RemotePage, String> {
    #[derive(serde::Deserialize)]
    struct Response {
        data: Vec<Sku>,
        total: u64,
    }
    #[derive(serde::Deserialize)]
    struct Sku {
        sku_id: RowId,
        tenant_id: RowId,
        product_id: RowId,
        sku_code: String,
        barcode: Option<String>,
        name: Option<String>,
        sku_kind: String,
        cost_cent: Option<i64>,
        is_active: bool,
        status: String,
        created_at: String,
        updated_at: String,
    }
    if bytes.len() > RemotePages::MAX_BYTES {
        return Err("SKU response exceeds 8 MiB".into());
    }
    let response: Response = serde_json::from_slice(bytes).map_err(|_| "Invalid SKU response")?;
    if response.data.len() > address.limit() {
        return Err("SKU page exceeds requested page size".into());
    }
    let rows = response
        .data
        .into_iter()
        .map(|sku| {
            if sku.tenant_id != tenant || sku.product_id != parent {
                return Err("SKU belongs to another product or merchant".into());
            }
            let cost = sku.cost_cent.map(|c| {
                let abs = c.unsigned_abs();
                format!(
                    "{}{}.{:02}",
                    if c < 0 { "-" } else { "" },
                    abs / 100,
                    abs % 100
                )
            });
            Ok(RemoteRow {
                id: sku.sku_id,
                version: Some(sku.updated_at.clone()),
                cells: BTreeMap::from([
                    (ColumnId(0), sku.sku_code),
                    (ColumnId(1), sku.barcode.unwrap_or_default()),
                    (ColumnId(2), sku.name.unwrap_or_default()),
                    (ColumnId(3), sku.sku_kind),
                    (ColumnId(4), cost.unwrap_or_default()),
                    (ColumnId(5), sku.is_active.to_string()),
                    (ColumnId(6), sku.status),
                    (ColumnId(7), sku.created_at),
                    (ColumnId(8), sku.updated_at),
                ]),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RemotePage {
        rows,
        total: RowCount::Exact(response.total),
        next_cursor: None,
        snapshot: None,
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "details_tests.rs"]
mod http_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn decode_preserves_schema_order_and_fixed_point_cost() {
        let tenant = RowId(1);
        let parent = RowId(2);
        let body = serde_json::json!({"data": [{
            "sku_id": RowId(3).to_uuid(), "tenant_id": tenant.to_uuid(), "product_id": parent.to_uuid(),
            "sku_code": "SKU-3", "barcode": "490", "name": "Fixture", "sku_kind": "KIT",
            "cost_cent": -50, "is_active": false, "status": "ACTIVE",
            "created_at": "2026-09-10T00:00:00Z", "updated_at": "2026-09-10T00:00:00Z"
        }], "total": 1});
        let page = decode(
            tenant,
            parent,
            &serde_json::to_vec(&body).unwrap(),
            &PageAddress::Offset {
                start: 0,
                limit: 100,
            },
        )
        .unwrap();
        assert_eq!(page.total, RowCount::Exact(1));
        assert_eq!(page.rows[0].cells.get(&ColumnId(0)), Some(&"SKU-3".into()));
        assert_eq!(page.rows[0].cells.get(&ColumnId(4)), Some(&"-0.50".into()));
        assert_eq!(page.rows[0].cells.get(&ColumnId(5)), Some(&"false".into()));
    }

    #[test]
    fn sku_find_focus_requires_exact_row_and_visible_column() {
        let row = RemoteRow {
            id: RowId(8),
            version: None,
            cells: BTreeMap::new(),
        };
        let columns = Details::default().columns;
        let hit = super::super::find::Hit {
            row_id: RowId(8),
            column: ColumnId(8),
            offset: 40,
            wrapped: false,
        };
        assert_eq!(
            verified_focus(std::slice::from_ref(&row), &columns, &hit, 20),
            Some((0, 8))
        );
        let wrong_row = super::super::find::Hit {
            row_id: RowId(9),
            ..hit.clone()
        };
        assert_eq!(
            verified_focus(std::slice::from_ref(&row), &columns, &wrong_row, 20),
            None
        );
        let mut hidden = columns;
        hidden[8].visible = false;
        assert_eq!(verified_focus(&[row], &hidden, &hit, 20), None);
    }

    #[test]
    fn sku_query_reset_cancels_stale_pending_focus() {
        let mut details = Details {
            find_pending: Some(super::super::find::Hit {
                row_id: RowId(8),
                column: ColumnId(0),
                offset: 20,
                wrapped: false,
            }),
            ..Default::default()
        };
        details.reset(false);
        assert!(details.find_pending.is_none());
    }
}
