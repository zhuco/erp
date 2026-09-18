use super::Session;
use crate::action_store::{
    StorageTask,
    order_recovery::{LoadedOrderRecovery, OrderRecoveryStore},
};
use crate::http::{HttpError, HttpRequest, HttpTask};
use eframe::egui;
use erp_grid_core::{Cell, CellEdit, Column, ColumnId, EditKind, FilterDraft, FilterKind, RowId};
use erp_ui_egui::{DataGrid, GridState};
use serde::Deserialize;

mod detail_modules;
mod drafts;
mod inspector;
mod model;
mod preferences;
mod query_ui;
use model::*;
use query_ui::*;

const ORDER_ID: ColumnId = ColumnId(1);
const PLATFORM: ColumnId = ColumnId(2);
const STATUS: ColumnId = ColumnId(3);
const TOTAL: ColumnId = ColumnId(4);
const TITLE: ColumnId = ColumnId(11);
const SKU: ColumnId = ColumnId(12);
const QTY: ColumnId = ColumnId(13);
const PRICE: ColumnId = ColumnId(14);

struct PendingEdit {
    task: HttpTask,
    ticket: u64,
    edit: CellEdit,
    expected_version: i32,
    path: String,
    json: Vec<u8>,
    retry: bool,
}

struct PreparedEdit {
    ticket: u64,
    edit: CellEdit,
    expected_version: i32,
    path: String,
    json: Vec<u8>,
}

struct UnknownEdit {
    edit: CellEdit,
    expected_version: i32,
    path: String,
    json: Vec<u8>,
}

pub(super) struct Orders {
    inspector: inspector::OrderInspector,
    detail_modules: detail_modules::DetailModules,
    sign_out: bool,
    keyword: String,
    period_days: u32,
    delivery: Option<crate::components::Delivery>,
    summary: crate::components::SelectionSummary,
    query_started: Option<web_time::Instant>,
    query_elapsed: Option<std::time::Duration>,
    offset: usize,
    page_size: usize,
    total: usize,
    orders: Vec<OrderSummary>,
    order_rows: Vec<RowId>,
    selected: Option<String>,
    detail: Option<OrderDetail>,
    item_rows: Vec<RowId>,
    list_task: Option<HttpTask>,
    list_initialized: bool,
    detail_task: Option<HttpTask>,
    detail_failed: bool,
    latest_task: Option<HttpTask>,
    pending: Option<PendingEdit>,
    prepared: Option<PreparedEdit>,
    unknown: Option<UnknownEdit>,
    conflict: bool,
    notice: Option<String>,
    search_deadline: Option<f64>,
    composing: bool,
    order_state: GridState,
    item_state: GridState,
    order_columns: Vec<Column>,
    item_columns: Vec<Column>,
    detail_generation: u64,
    views: super::views::Views,
    recovery_identity: Option<(String, RowId, RowId)>,
    recovery_identity_verified: bool,
    recovery_snapshot: Option<drafts::Snapshot>,
    recovery_corrupt: bool,
    recovery_clear_requested: bool,
    recovery_restored: bool,
    recovery_opening: Option<StorageTask<LoadedOrderRecovery>>,
    recovery_store: Option<OrderRecoveryStore>,
    recovery_revision: u64,
    recovery_saving: Option<(StorageTask<u64>, Option<Vec<u8>>)>,
    pub(super) workbench: super::workbench_layout::WorkbenchLayout,
}

impl Default for Orders {
    fn default() -> Self {
        let mut order_state = GridState::default();
        order_state.metrics = erp_ui_egui::GridMetrics::legacy();
        order_state.read_only = true;
        order_state.theme = Some(crate::erp_theme::grid());
        order_state.quick_search_open = true;
        order_state.sorting_enabled = false;
        order_state.filtering_enabled = true;
        let mut item_state = GridState::default();
        item_state.metrics = erp_ui_egui::GridMetrics::legacy();
        item_state.quick_search_open = false;
        item_state.theme = Some(crate::erp_theme::grid());
        item_state.sorting_enabled = false;
        item_state.filtering_enabled = false;
        Self {
            inspector: Default::default(),
            detail_modules: Default::default(),
            sign_out: false,
            keyword: String::new(),
            period_days: 0,
            delivery: None,
            summary: Default::default(),
            query_started: None,
            query_elapsed: None,
            offset: 0,
            page_size: 50,
            total: 0,
            orders: Vec::new(),
            order_rows: Vec::new(),
            selected: None,
            detail: None,
            item_rows: Vec::new(),
            list_task: None,
            list_initialized: false,
            detail_task: None,
            detail_failed: false,
            latest_task: None,
            pending: None,
            prepared: None,
            unknown: None,
            conflict: false,
            notice: None,
            search_deadline: None,
            composing: false,
            order_state,
            item_state,
            order_columns: order_columns(),
            item_columns: item_columns(),
            detail_generation: 0,
            views: super::views::Views::for_grid("orders-schema1", "main"),
            recovery_identity: None,
            recovery_identity_verified: false,
            recovery_snapshot: None,
            recovery_corrupt: false,
            recovery_clear_requested: false,
            recovery_restored: false,
            recovery_opening: None,
            recovery_store: None,
            recovery_revision: 0,
            recovery_saving: None,
            workbench: Default::default(),
        }
    }
}

impl Orders {
    pub(super) fn take_sign_out(&mut self) -> bool {
        std::mem::take(&mut self.sign_out)
    }
    pub(super) fn dirty(&self) -> bool {
        self.inspector.dirty()
            || self.item_state.is_editing()
            || self.prepared.is_some()
            || self.pending.is_some()
            || self.unknown.is_some()
            || self.recovery_snapshot.is_some()
            || self.recovery_corrupt
            || self.recovery_opening.is_some()
            || self.recovery_saving.is_some()
    }

    pub(super) fn hold_for_leave(&mut self) {
        self.item_state.hold_edit();
    }

    pub(super) fn leave_blocked(&self) -> Option<&'static str> {
        if let Some(reason) = self.inspector.leave_blocked() {
            return Some(reason);
        }
        if self.pending.is_some()
            || self.prepared.is_some()
            || self.recovery_opening.is_some()
            || self.recovery_saving.is_some()
        {
            Some("订单货品保存结果尚未确认")
        } else if self.unknown.is_some()
            || self
                .recovery_snapshot
                .as_ref()
                .is_some_and(drafts::Snapshot::unknown)
            || self.recovery_corrupt
        {
            Some("订单货品保存结果未知，必须读取最新订单核对")
        } else if self.recovery_snapshot.is_some() {
            Some("请返回订单页明确保存或放弃恢复草稿")
        } else {
            None
        }
    }

    pub(super) fn discard_for_leave(&mut self) -> Result<(), &'static str> {
        if let Some(reason) = self.leave_blocked() {
            return Err(reason);
        }
        self.inspector.discard_for_leave()?;
        self.item_state.cancel_edit();
        if self.recovery_snapshot.is_some() {
            self.recovery_clear_requested = true;
        }
        self.conflict = false;
        Ok(())
    }

    pub(super) fn request_save_for_leave(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
    ) -> Result<bool, String> {
        if let Some(reason) = self.leave_blocked() {
            return Err(reason.into());
        }
        let Some(draft) = self.item_state.edit_draft() else {
            return Ok(true);
        };
        let column = self
            .item_columns
            .iter()
            .find(|column| column.id == draft.column)
            .ok_or("编辑列已不存在")?;
        if let Some((edit, ticket)) = self.item_state.prepare_edit_commit(column)? {
            self.start_edit(session, ctx, edit, ticket)?;
        }
        Ok(!self.dirty())
    }

    /// Cancels reads when credentials change. A dispatched write remains owned
    /// until `tick` observes its result; an unsent editor remains intact.
    pub(super) fn clear(&mut self) {
        self.detail_modules.clear();
        self.list_task = None;
        self.list_initialized = false;
        self.search_deadline = None;
        self.composing = false;
        self.detail_task = None;
        self.latest_task = None;
        self.orders.clear();
        self.order_rows.clear();
        if !self.dirty() {
            self.detail = None;
            self.item_rows.clear();
            self.selected = None;
        }
        self.item_state.hold_edit();
    }

    pub(super) fn ui(&mut self, ui: &mut egui::Ui, session: &Session) {
        ui.horizontal_wrapped(|ui| {
            self.query_toolbar(ui);
            self.views_ui(ui, session);
            if ui
                .toggle_value(&mut self.workbench.preferences.left_open, "筛选面板")
                .changed()
            {
                self.workbench.mark_dirty();
            }
            if ui
                .toggle_value(&mut self.workbench.preferences.right_open, "订单详情")
                .changed()
            {
                self.workbench.mark_dirty();
            }
            if ui
                .add_enabled(
                    !self.dirty(),
                    egui::Button::new("底部明细").selected(self.workbench.preferences.bottom_open),
                )
                .clicked()
            {
                self.workbench.preferences.bottom_open = !self.workbench.preferences.bottom_open;
                self.workbench.mark_dirty();
            }
            if ui
                .add_enabled(!self.dirty(), egui::Button::new("退出登录"))
                .clicked()
            {
                self.sign_out = true;
            }
        });
        if self.workbench.preferences.left_open {
            let response = egui::Panel::left(
                ui.id()
                    .with(("orders-business-filter", self.workbench.scope_key())),
            )
            .resizable(true)
            .exact_size(self.workbench.preferences.left_width)
            .size_range(160.0..=420.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                self.filter_tree(ui);
            });
            self.workbench
                .set_left_width(response.response.rect.width());
        }
        self.inspector
            .sync_selection(self.selected.as_deref(), session, ui.ctx());
        let inspector_enabled = !self.item_state.is_editing()
            && self.pending.is_none()
            && self.prepared.is_none()
            && self.unknown.is_none();
        if self.workbench.preferences.right_open {
            let response = egui::Panel::right(
                ui.id()
                    .with(("orders-detail-panel", self.workbench.scope_key())),
            )
            .resizable(true)
            .exact_size(self.workbench.preferences.right_width)
            .size_range(240.0..=520.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.add_enabled_ui(inspector_enabled, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.inspector.ui(ui, session));
                });
            });
            self.workbench
                .set_right_width(response.response.rect.width());
        }
        if self.detail_task.is_none()
            && self.detail.is_none()
            && self.selected.is_some()
            && !self.detail_failed
        {
            self.load_detail(session, ui.ctx());
        }
        self.tick_recovery(session, ui.ctx());
        if let Err(error) = self.dispatch_prepared(session, ui.ctx()) {
            self.notice = Some(error);
        }
        if self.list_task.is_none() && !self.list_initialized {
            self.load_list(session, ui.ctx());
        }

        if self.list_task.is_some()
            || self.detail_task.is_some()
            || self.latest_task.is_some()
            || self.conflict
            || self.detail_failed
            || self.unknown.is_some()
            || self.recovery_snapshot.is_some()
            || self.notice.is_some()
        {
            ui.horizontal(|ui| {
                if self.list_task.is_some()
                    || self.detail_task.is_some()
                    || self.latest_task.is_some()
                {
                    ui.spinner();
                }
                if self.conflict
                    && ui
                        .button("读取最新")
                        .on_hover_text("保留当前输入并读取服务器最新版本")
                        .clicked()
                {
                    self.load_latest(session, ui.ctx());
                }
                if self.detail_failed && ui.button("重试详情").clicked() {
                    self.detail_failed = false;
                    self.load_detail(session, ui.ctx());
                }
                if self.unknown.is_some()
                    && self.pending.is_none()
                    && ui
                        .button("重试原保存")
                        .on_hover_text("使用原版本与原请求体重试")
                        .clicked()
                    && let Err(error) = self.retry_original(session, ui.ctx())
                {
                    self.notice = Some(error);
                }
                if self
                    .recovery_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| !snapshot.unknown())
                    && self.prepared.is_none()
                    && self.pending.is_none()
                    && ui.button("放弃恢复草稿").clicked()
                {
                    self.item_state.cancel_edit();
                    self.recovery_clear_requested = true;
                    self.notice = Some("正在删除订单恢复草稿".into());
                }
                if let Some(notice) = &self.notice {
                    ui.label(notice);
                }
            });
        }
        let available = ui.available_height();
        let height = if self.workbench.preferences.bottom_open {
            (available - self.workbench.preferences.bottom_height - 32.0).max(100.0)
        } else {
            (available - 28.0).max(100.0)
        };
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| self.order_grid(ui),
        );
        self.order_pager(ui, session);
        if self.workbench.preferences.bottom_open {
            let response = ui
                .allocate_response(egui::vec2(ui.available_width(), 6.0), egui::Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
            if response.dragged()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                self.workbench.preferences.bottom_height = (ui.max_rect().bottom() - pointer.y)
                    .clamp(100.0, (available - 130.0).max(100.0));
                self.workbench.mark_dirty();
            }
            let can_switch = !self.dirty();
            if self.detail_modules.ui(ui, can_switch) {
                self.item_grid(ui, session);
            }
            if self.detail_modules.take_retry_requested() {
                self.detail_modules.retry(session, ui.ctx());
            }
        }
    }

    pub(super) fn suspend_reads(&mut self) {
        self.detail_modules.clear();
        self.list_task = None;
        self.list_initialized = false;
        self.search_deadline = None;
        self.detail_task = None;
        self.latest_task = None;
    }

    fn invalidate_filter(&mut self, ctx: &egui::Context) {
        self.list_initialized = true;
        self.detail_modules.clear();
        self.list_task = None;
        self.orders.clear();
        self.order_rows.clear();
        self.total = 0;
        self.selected = None;
        self.detail_task = None;
        self.latest_task = None;
        self.detail = None;
        self.item_rows.clear();
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.order_state.bind_view(self.detail_generation);
        self.item_state.bind_view(self.detail_generation);
        self.search_deadline = Some(ctx.input(|input| input.time) + 0.25);
    }

    pub(super) fn tick(&mut self, ctx: &egui::Context, session: &Session) -> bool {
        let mut authentication_failed = self.detail_modules.tick();
        authentication_failed |= self.inspector.tick(ctx, session);
        if !self.inspector.dirty() && self.inspector.take_changed() {
            let selected = self.inspector.selected().map(str::to_owned);
            self.load_list(session, ctx);
            self.selected = selected;
            self.load_detail(session, ctx);
        }
        if let Some(result) = self
            .delivery
            .as_ref()
            .and_then(crate::components::Delivery::poll)
        {
            self.delivery = None;
            self.notice = Some(match result {
                Ok(path) => format!("已导出：{path}"),
                Err(error) => error,
            });
        }
        if self.delivery.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
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
        if self.search_deadline.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if !self.composing
            && self
                .search_deadline
                .is_some_and(|deadline| ctx.input(|input| input.time) >= deadline)
        {
            self.search_deadline = None;
            self.offset = 0;
            self.load_list(session, ctx);
        }
        if let Some(result) = self.list_task.as_mut().and_then(HttpTask::poll) {
            self.list_task = None;
            self.query_elapsed = self.query_started.take().map(|start| start.elapsed());
            authentication_failed |= auth_failed(&result);
            match decode::<OrderPage>(result) {
                Ok(page) => {
                    if page.total < 0
                        || page.items.len() > self.page_size
                        || page.total < page.items.len() as i64
                    {
                        self.notice = Some("订单分页响应无效".into());
                    } else {
                        self.total = page.total as usize;
                        self.order_rows = page.items.iter().map(|row| stable_id(&row.id)).collect();
                        self.orders = page.items;
                        self.notice = None;
                    }
                }
                Err(error) => self.notice = Some(error),
            }
        }
        if let Some(result) = self.detail_task.as_mut().and_then(HttpTask::poll) {
            self.detail_task = None;
            authentication_failed |= auth_failed(&result);
            if let Ok(bytes) = &result {
                self.detail_modules.accept_detail(bytes);
            }
            match decode::<OrderDetail>(result) {
                Ok(detail) if valid_detail(&detail, self.selected.as_deref()) => {
                    self.accept_detail(detail);
                    self.detail_failed = false;
                    self.notice = None;
                }
                Ok(_) => {}
                Err(error) => {
                    self.detail_failed = true;
                    self.notice = Some(error);
                }
            }
        }
        if let Some(result) = self.latest_task.as_mut().and_then(HttpTask::poll) {
            self.latest_task = None;
            authentication_failed |= auth_failed(&result);
            match decode::<OrderDetail>(result) {
                Ok(latest) if valid_detail(&latest, self.selected.as_deref()) => {
                    self.rebase_on_latest(latest);
                }
                Ok(_) => {}
                Err(error) => self.notice = Some(error),
            }
        }
        if let Some(result) = self
            .pending
            .as_mut()
            .and_then(|pending| pending.task.poll())
        {
            authentication_failed |= auth_failed(&result);
            let Some(pending) = self.pending.take() else {
                return authentication_failed;
            };
            match result {
                Ok(bytes) => match serde_json::from_slice::<OrderDetail>(&bytes) {
                    Ok(detail)
                        if valid_saved_detail(
                            &detail,
                            self.selected.as_deref(),
                            pending.expected_version,
                        ) =>
                    {
                        self.detail_modules.accept_detail(&bytes);
                        self.item_state.resolve_edit(pending.ticket, Ok(()));
                        self.unknown = None;
                        self.recovery_restored = true;
                        self.accept_detail(detail);
                        self.conflict = false;
                        self.notice = None;
                        self.recovery_clear_requested = true;
                    }
                    Ok(_) | Err(_) => {
                        self.item_state.resolve_edit(
                            pending.ticket,
                            Err("保存响应无效，请读取最新订单核对".into()),
                        );
                        self.notice = Some("保存结果未知，请读取最新订单核对".into());
                        self.conflict = true;
                        self.unknown = Some(UnknownEdit {
                            edit: pending.edit,
                            expected_version: pending.expected_version,
                            path: pending.path,
                            json: pending.json,
                        });
                    }
                },
                Err(HttpError::Status(409)) => {
                    self.item_state
                        .resolve_edit(pending.ticket, Err("订单版本冲突".into()));
                    self.conflict = true;
                    self.notice = Some("版本冲突，请读取最新后确认".into());
                    if pending.retry {
                        self.unknown = Some(UnknownEdit {
                            edit: pending.edit,
                            expected_version: pending.expected_version,
                            path: pending.path,
                            json: pending.json,
                        });
                    } else {
                        self.recovery_clear_requested = true;
                    }
                }
                Err(HttpError::Status(code)) if (400..500).contains(&code) => {
                    self.item_state
                        .resolve_edit(pending.ticket, Err(format!("保存被拒绝：HTTP {code}")));
                    self.notice = Some(format!("保存被拒绝：HTTP {code}"));
                    if pending.retry {
                        self.unknown = Some(UnknownEdit {
                            edit: pending.edit,
                            expected_version: pending.expected_version,
                            path: pending.path,
                            json: pending.json,
                        });
                    } else {
                        self.recovery_clear_requested = true;
                    }
                }
                Err(error) => {
                    self.item_state
                        .resolve_edit(pending.ticket, Err(format!("保存未确认：{error}")));
                    self.notice = Some("保存结果未知，请读取最新订单核对".into());
                    self.conflict = true;
                    self.unknown = Some(UnknownEdit {
                        edit: pending.edit,
                        expected_version: pending.expected_version,
                        path: pending.path,
                        json: pending.json,
                    });
                }
            }
        }
        if self.list_task.is_some()
            || self.detail_task.is_some()
            || self.latest_task.is_some()
            || self.pending.is_some()
        {
            ctx.request_repaint();
        }
        let _ = session;
        authentication_failed
    }

    fn order_grid(&mut self, ui: &mut egui::Ui) {
        let query_locked = self.dirty();
        self.order_state.read_only = query_locked;
        self.order_state.filtering_enabled = !query_locked;
        self.order_state.row_number_start = self.offset as u64;
        let output = DataGrid {
            id: egui::Id::new("remote-orders"),
            columns: &mut self.order_columns,
            rows: self.orders.len(),
            state: &mut self.order_state,
            row_id: Some(&|index| self.order_rows[index]),
        }
        .show(ui, |index, column| order_text(&self.orders[index], column));
        if output.quick_search_changed && !query_locked {
            self.invalidate_filter(ui.ctx());
        }
        let selected = self.order_state.row_selection.active;
        if output.edits.is_empty() && selected != self.selected.as_deref().map(stable_id) {
            if self.dirty() {
                self.notice = Some("请先保存或放弃当前货品编辑".into());
                self.order_state.row_selection.active = self.selected.as_deref().map(stable_id);
            } else if let Some(index) =
                selected.and_then(|id| self.order_rows.iter().position(|v| *v == id))
            {
                self.detail_task = None;
                self.latest_task = None;
                self.selected = Some(self.orders[index].id.clone());
                self.detail = None;
                self.detail_failed = false;
                self.item_rows.clear();
                self.detail_generation = self.detail_generation.wrapping_add(1);
                self.item_state.bind_view(self.detail_generation);
            }
        }
    }

    fn item_grid(&mut self, ui: &mut egui::Ui, session: &Session) {
        if self.detail_task.is_none()
            && self.detail.is_none()
            && self.selected.is_some()
            && !self.detail_failed
        {
            self.load_detail(session, ui.ctx());
        }
        let items = self
            .detail
            .as_ref()
            .map_or(&[][..], |detail| detail.items.as_slice());
        self.item_state.read_only = self.inspector.dirty()
            || self.prepared.is_some()
            || self.pending.is_some()
            || self.unknown.is_some()
            || self.recovery_opening.is_some()
            || self.recovery_saving.is_some()
            || self.recovery_store.is_none()
            || self.latest_task.is_some()
            || self.detail.as_ref().is_some_and(|detail| !editable(detail));
        let output = DataGrid {
            id: egui::Id::new("remote-order-items"),
            columns: &mut self.item_columns,
            rows: items.len(),
            state: &mut self.item_state,
            row_id: Some(&|index| self.item_rows[index]),
        }
        .show(ui, |index, column| item_text(&items[index], column));
        if let (Some(edit), Some(ticket)) = (output.edits.into_iter().next(), output.edit_ticket)
            && let Err(error) = self.start_edit(session, ui.ctx(), edit, ticket)
        {
            self.item_state.resolve_edit(ticket, Err(error));
        }
    }

    fn order_pager(&mut self, ui: &mut egui::Ui, session: &Session) {
        self.summary.update(
            self.detail_generation,
            self.offset as u64,
            &self.order_state.selection,
            &self.order_columns,
            self.orders.len(),
            |i, column| order_text(&self.orders[i], column),
        );
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} 条 · 勾选 {}",
                self.total,
                self.order_state.row_selection.checked().len()
            ));
            if let Some(elapsed) = self.query_elapsed {
                ui.label(format!("{} ms", elapsed.as_millis()));
            }
            ui.label(&self.summary.label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let enabled =
                    !self.dirty() && self.list_task.is_none() && self.search_deadline.is_none();
                if ui
                    .add_enabled(
                        enabled && self.offset + self.orders.len() < self.total,
                        egui::Button::new("下一页"),
                    )
                    .clicked()
                {
                    self.offset += self.page_size;
                    self.load_list(session, ui.ctx());
                }
                if ui
                    .add_enabled(enabled && self.offset > 0, egui::Button::new("上一页"))
                    .clicked()
                {
                    self.offset = self.offset.saturating_sub(self.page_size);
                    self.load_list(session, ui.ctx());
                }
                let previous_size = self.page_size;
                ui.add_enabled_ui(enabled, |ui| {
                    egui::ComboBox::from_id_salt("remote-order-page-size")
                        .selected_text(self.page_size.to_string())
                        .show_ui(ui, |ui| {
                            for size in [10, 25, 50, 100] {
                                ui.selectable_value(&mut self.page_size, size, size.to_string());
                            }
                        });
                });
                if self.page_size != previous_size {
                    self.offset = 0;
                    self.load_list(session, ui.ctx());
                }
            });
        });
    }

    fn load_list(&mut self, session: &Session, ctx: &egui::Context) {
        self.list_initialized = true;
        let (_, platform, status) = selection_filters(&self.order_state);
        let mut query = vec![
            ("offset".into(), self.offset.to_string()),
            ("limit".into(), self.page_size.to_string()),
            ("platform".into(), platform),
            ("oms_status".into(), status),
        ];
        match extra_query(
            &self.order_state,
            &self.keyword,
            self.period_days,
            chrono::Utc::now(),
        )
        .and_then(|mut query| {
            query.extend(amount_query(self.order_state.quick_search.get(&TOTAL))?);
            Ok(query)
        }) {
            Ok(amount) => query.extend(amount),
            Err(error) => {
                self.list_task = None;
                self.orders.clear();
                self.order_rows.clear();
                self.total = 0;
                self.notice = Some(error.into());
                return;
            }
        }
        self.orders.clear();
        self.order_rows.clear();
        self.selected = None;
        self.detail = None;
        self.detail_task = None;
        self.latest_task = None;
        self.item_rows.clear();
        self.detail_modules.clear();
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.order_state.rebind_page(self.detail_generation, true);
        self.item_state.bind_view(self.detail_generation);
        self.query_started = Some(web_time::Instant::now());
        self.list_task = session
            .client
            .start(
                HttpRequest {
                    path: format!("/v1/tenants/{}/orders", session.tenant.to_uuid()),
                    query,
                    json: None,
                    bearer: Some(session.token.clone()),
                },
                ctx,
            )
            .map_err(|error| self.notice = Some(error.to_string()))
            .ok();
    }

    fn load_detail(&mut self, session: &Session, ctx: &egui::Context) {
        let Some(order) = self.selected.as_deref() else {
            return;
        };
        self.detail_modules.load(session, ctx, order);
        self.detail_task = get_detail(session, order, ctx)
            .map_err(|error| self.notice = Some(error.to_string()))
            .ok();
    }

    fn load_latest(&mut self, session: &Session, ctx: &egui::Context) {
        let Some(order) = self.selected.as_deref() else {
            return;
        };
        self.latest_task = get_detail(session, order, ctx)
            .map_err(|error| self.notice = Some(error.to_string()))
            .ok();
    }

    fn start_edit(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
        edit: CellEdit,
        ticket: u64,
    ) -> Result<(), String> {
        if self.conflict {
            return Err("请先读取最新版本".into());
        }
        let detail = self.detail.as_ref().ok_or("订单详情尚未加载")?;
        let index = self
            .item_rows
            .iter()
            .position(|row| *row == edit.row)
            .ok_or("货品身份已变化")?;
        let body = edit_body(detail.order.version, &detail.items[index], &edit)?;
        let json = serde_json::to_vec(&body).map_err(|_| "无法编码保存请求")?;
        let path = format!(
            "/v1/tenants/{}/orders/{}",
            session.tenant.to_uuid(),
            detail.order.id
        );
        self.prepared = Some(PreparedEdit {
            ticket,
            edit,
            expected_version: detail.order.version,
            path,
            json,
        });
        self.notice = Some("正在保存恢复检查点，尚未派发".into());
        ctx.request_repaint();
        Ok(())
    }

    fn dispatch_prepared(&mut self, session: &Session, ctx: &egui::Context) -> Result<(), String> {
        let Some(prepared) = self.prepared.as_ref() else {
            return Ok(());
        };
        if !self.recovery_account_matches(session) {
            return Err("订单恢复账号身份未确认，禁止派发".into());
        }
        let Some(snapshot) = &self.recovery_snapshot else {
            return Ok(());
        };
        let Some((server, tenant, actor)) = &self.recovery_identity else {
            return Err("订单恢复账号身份不可用".into());
        };
        if snapshot.scope.server != *server
            || snapshot.scope.tenant != *tenant
            || snapshot.scope.actor != *actor
            || snapshot.scope.tenant != session.tenant
        {
            return Err("订单恢复检查点不属于当前账号".into());
        }
        let drafts::Recovery::Unknown { request } = &snapshot.recovery else {
            return Ok(());
        };
        if request.path != prepared.path
            || request.body != prepared.json
            || request.expected_version != prepared.expected_version
        {
            return Err("恢复检查点与待派发请求不一致".into());
        }
        let task = session
            .client
            .put(
                HttpRequest {
                    path: prepared.path.clone(),
                    query: Vec::new(),
                    json: Some(prepared.json.clone()),
                    bearer: Some(session.token.clone()),
                },
                ctx,
            )
            .map_err(|error| error.to_string())?;
        let prepared = self.prepared.take().ok_or("待派发请求已变化")?;
        self.pending = Some(PendingEdit {
            task,
            ticket: prepared.ticket,
            edit: prepared.edit,
            expected_version: prepared.expected_version,
            path: prepared.path,
            json: prepared.json,
            retry: false,
        });
        self.notice = Some("订单货品保存已派发".into());
        Ok(())
    }

    fn retry_original(&mut self, session: &Session, ctx: &egui::Context) -> Result<(), String> {
        if !self.recovery_account_matches(session) {
            return Err("订单恢复账号身份未确认，禁止重试".into());
        }
        let unknown = self.unknown.take().ok_or("没有待核对的保存请求")?;
        let Some(column) = self
            .item_columns
            .iter()
            .find(|column| column.id == unknown.edit.column)
        else {
            self.unknown = Some(unknown);
            return Err("编辑列已不存在".into());
        };
        let prepared = match self.item_state.prepare_edit_commit(column) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.unknown = Some(unknown);
                return Err(error);
            }
        };
        let Some((edit, ticket)) = prepared else {
            self.unknown = Some(unknown);
            return Err("原草稿已变化，不能重试旧请求".into());
        };
        if edit.row != unknown.edit.row
            || edit.column != unknown.edit.column
            || edit.after != unknown.edit.after
        {
            self.item_state
                .resolve_edit(ticket, Err("原草稿已变化，不能重试旧请求".into()));
            self.unknown = Some(unknown);
            return Err("原草稿已变化，不能重试旧请求".into());
        }
        let task = match session.client.put(
            HttpRequest {
                path: unknown.path.clone(),
                query: Vec::new(),
                json: Some(unknown.json.clone()),
                bearer: Some(session.token.clone()),
            },
            ctx,
        ) {
            Ok(task) => task,
            Err(error) => {
                self.item_state.resolve_edit(ticket, Err(error.to_string()));
                self.unknown = Some(unknown);
                return Err(error.to_string());
            }
        };
        self.pending = Some(PendingEdit {
            task,
            ticket,
            edit,
            expected_version: unknown.expected_version,
            path: unknown.path,
            json: unknown.json,
            retry: true,
        });
        Ok(())
    }

    fn accept_detail(&mut self, detail: OrderDetail) {
        self.item_rows = detail
            .items
            .iter()
            .map(|item| stable_child_id(&detail.order.id, &item.id))
            .collect();
        self.detail = Some(detail);
        self.restore_recovery_edit();
    }

    fn restore_recovery_edit(&mut self) {
        if self.recovery_restored || self.item_state.is_editing() {
            return;
        }
        let Some(snapshot) = self.recovery_snapshot.clone() else {
            return;
        };
        let Some(detail) = &self.detail else {
            return;
        };
        if detail.order.id != snapshot.scope.order_id {
            return;
        }
        let (edit, unknown) = match snapshot.recovery {
            drafts::Recovery::Unsent { edit } => (edit, None),
            drafts::Recovery::Unknown { request } => (request.edit.clone(), Some(request)),
        };
        let Some(row_index) = detail.items.iter().position(|item| item.id == edit.line_id) else {
            self.notice = Some("恢复记录的订单货品已不存在，保留检查点等待处理".into());
            self.conflict = true;
            return;
        };
        let Some(column_index) = self
            .item_columns
            .iter()
            .position(|column| column.id == edit.column)
        else {
            self.notice = Some("恢复记录的编辑列已不存在".into());
            self.conflict = true;
            return;
        };
        let before = if unknown.is_some() {
            edit.before.clone()
        } else {
            item_text(&detail.items[row_index], edit.column)
        };
        if let Err(error) = self.item_state.restore_edit(
            Cell {
                row: row_index,
                column: column_index,
            },
            edit.row,
            edit.column,
            before,
            edit.after.clone(),
        ) {
            self.notice = Some(error.into());
            return;
        }
        if let Some(request) = unknown {
            self.unknown = Some(UnknownEdit {
                edit: CellEdit {
                    row: request.edit.row,
                    column: request.edit.column,
                    before: request.edit.before,
                    after: request.edit.after,
                },
                expected_version: request.expected_version,
                path: request.path,
                json: request.body,
            });
            self.conflict = true;
            self.notice = Some("已恢复结果未知的原请求，必须读取最新订单人工核对".into());
        } else {
            self.notice = Some("已恢复未派发的订单货品草稿，请检查后明确保存".into());
        }
        self.recovery_restored = true;
    }

    fn rebase_on_latest(&mut self, latest: OrderDetail) {
        let Some(draft) = self.item_state.edit_draft() else {
            self.accept_detail(latest);
            self.conflict = false;
            return;
        };
        let Some(previous_index) = self.item_rows.iter().position(|row| *row == draft.row) else {
            self.notice = Some("最新订单已删除该货品，草稿保持未提交".into());
            return;
        };
        let line_id = self
            .detail
            .as_ref()
            .and_then(|detail| detail.items.get(previous_index))
            .map(|item| item.id.as_str());
        let Some(index) =
            line_id.and_then(|line_id| latest.items.iter().position(|item| item.id == line_id))
        else {
            self.notice = Some("最新订单货品身份已变化，草稿保持未提交".into());
            return;
        };
        let before = item_text(&latest.items[index], draft.column);
        if let Some(unknown) = &self.unknown {
            if latest.order.version <= unknown.expected_version || before != unknown.edit.after {
                self.accept_detail(latest);
                self.notice = Some("最新数据不能确认上次保存结果，禁止重发".into());
                return;
            }
            self.unknown = None;
            self.recovery_clear_requested = true;
            self.item_state.cancel_edit();
            self.accept_detail(latest);
            self.conflict = false;
            self.notice = Some("已从最新订单确认原保存结果".into());
            return;
        }
        self.accept_detail(latest);
        if before == draft.after {
            self.item_state.cancel_edit();
            self.conflict = false;
            self.notice = Some("最新订单已包含目标值".into());
        } else {
            self.item_state.rebase_edit(draft.row, draft.column, before);
            self.conflict = false;
            self.notice = Some("已读取最新版本，请检查草稿后明确保存".into());
        }
    }
}

fn order_columns() -> Vec<Column> {
    let mut amount = Column::new(TOTAL, "金额", 160.0).with_filter(FilterKind::Money);
    amount.value_kind = erp_grid_core::ValueKind::Money;
    vec![
        Column::new(ORDER_ID, "订单号", 180.0).with_filter(FilterKind::Text),
        Column::new(PLATFORM, "平台", 100.0).with_filter(FilterKind::Enum(
            ["taobao", "jd", "pdd", "douyin"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )),
        Column::new(STATUS, "状态", 130.0).with_filter(FilterKind::Enum(
            [
                "NEW",
                "REVIEW_PENDING",
                "PENDING_SHIPMENT",
                "ALLOCATED",
                "PICKING",
                "PACKED",
                "SHIPPED",
                "DONE",
                "CANCELLED",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        )),
        amount,
        Column::new(RECEIVER, "收件人", 90.0).with_filter(FilterKind::Text),
        Column::new(PHONE, "联系电话", 130.0).with_filter(FilterKind::Text),
        Column::new(CUSTOMER, "客户", 110.0).with_filter(FilterKind::None),
        Column::new(SHOP, "店铺ID", 180.0).with_filter(FilterKind::None),
        Column::new(MAIN_STATUS, "订单状态", 100.0).with_filter(FilterKind::None),
        Column::new(REVIEW, "审核状态", 120.0).with_filter(FilterKind::None),
        Column::new(FULFILLMENT, "履约状态", 110.0).with_filter(FilterKind::None),
        Column::new(EXCEPTION, "异常状态", 110.0).with_filter(FilterKind::None),
        Column::new(FLAGS, "异常标记", 150.0).with_filter(FilterKind::None),
        Column::new(PAYMENT, "付款状态", 110.0).with_filter(FilterKind::None),
        Column::new(TRACKING, "物流单号", 150.0).with_filter(FilterKind::None),
        Column::new(CREATED, "创建时间", 155.0).with_filter(FilterKind::DateTime),
        Column::new(UPDATED, "更新时间", 155.0).with_filter(FilterKind::DateTime),
    ]
}

fn amount_query(draft: Option<&FilterDraft>) -> Result<Vec<(String, String)>, &'static str> {
    let Some(draft) = draft else {
        return Ok(Vec::new());
    };
    let Some(erp_grid_core::Filter::Money { min, max }) = draft.compile(&FilterKind::Money)? else {
        return Ok(Vec::new());
    };
    if min.is_some_and(|value| value < 0) || max.is_some_and(|value| value < 0) {
        return Err("订单金额范围不能为负数");
    }
    Ok([
        ("total_amount_min_cent", min),
        ("total_amount_max_cent", max),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.map(|value| (key.into(), value.to_string())))
    .collect())
}

fn selection_filters(state: &GridState) -> (String, String, String) {
    let text = |column| match state.quick_search.get(&column) {
        Some(FilterDraft::Text(value)) => value.trim().to_owned(),
        _ => String::new(),
    };
    let selected = |column| match state.quick_search.get(&column) {
        Some(FilterDraft::Enum(values)) => values.iter().cloned().collect::<Vec<_>>().join(","),
        _ => String::new(),
    };
    (text(ORDER_ID), selected(PLATFORM), selected(STATUS))
}

fn item_columns() -> Vec<Column> {
    let mut qty = Column::new(QTY, "数量", 90.0);
    qty.edit_kind = EditKind::Integer;
    let mut price = Column::new(PRICE, "单价", 110.0);
    price.edit_kind = EditKind::Money;
    vec![
        Column::new(TITLE, "货品", 240.0),
        Column::new(SKU, "SKU", 150.0),
        qty,
        price,
    ]
}

fn order_text(order: &OrderSummary, column: ColumnId) -> String {
    match column {
        ORDER_ID => order.platform_order_id.clone(),
        PLATFORM => platform_label(&order.platform).into(),
        STATUS => status_label(&order.oms_status).into(),
        TOTAL => cents_text(order.total_amount_cent),
        RECEIVER => order.receiver_name.clone().unwrap_or_default(),
        PHONE => order.receiver_phone.clone().unwrap_or_default(),
        CUSTOMER => order.customer_name.clone().unwrap_or_default(),
        SHOP => order.shop_id.clone(),
        MAIN_STATUS => order.order_status.clone(),
        REVIEW => order.review_status.clone(),
        FULFILLMENT => order.fulfillment_status.clone(),
        EXCEPTION => order.exception_status.clone(),
        FLAGS => order.exception_flags.join("、"),
        PAYMENT => order.payment_status.clone().unwrap_or_default(),
        TRACKING => order.tracking_no.clone().unwrap_or_default(),
        CREATED => display_time(&order.created_at),
        UPDATED => display_time(&order.updated_at),
        _ => String::new(),
    }
}

fn item_text(item: &OrderItem, column: ColumnId) -> String {
    match column {
        TITLE => item.title.clone().unwrap_or_default(),
        SKU => item
            .outer_sku_id
            .clone()
            .or_else(|| item.platform_sku_id.clone())
            .unwrap_or_default(),
        QTY => item.quantity.to_string(),
        PRICE => cents_text(item.price_cent),
        _ => String::new(),
    }
}

fn edit_body<'a>(
    version: i32,
    item: &'a OrderItem,
    edit: &CellEdit,
) -> Result<EditBody<'a>, String> {
    let (qty, price_cent) = match edit.column {
        QTY => {
            let qty = edit.after.parse::<i32>().map_err(|_| "数量必须是整数")?;
            if qty <= 0 || item.price_cent.checked_mul(i64::from(qty)).is_none() {
                return Err("数量必须大于零且金额不能溢出".into());
            }
            (Some(qty), None)
        }
        PRICE => {
            let price = parse_cents(&edit.after)?;
            if price.checked_mul(i64::from(item.quantity)).is_none() {
                return Err("金额超出范围".into());
            }
            (None, Some(price))
        }
        _ => return Err("该列不可编辑".into()),
    };
    Ok(EditBody {
        expected_version: version,
        item_edits: [ItemEdit {
            line_id: &item.id,
            qty,
            price_cent,
        }],
    })
}

fn parse_cents(value: &str) -> Result<i64, String> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') {
        return Err("金额必须大于等于零".into());
    }
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("金额格式无效".into());
    }
    if fraction.len() > 2 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("金额最多两位小数".into());
    }
    let whole = whole.parse::<i64>().map_err(|_| "金额超出范围")?;
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().map_err(|_| "金额格式无效")? * 10,
        _ => fraction.parse::<i64>().map_err(|_| "金额格式无效")?,
    };
    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| "金额超出范围".into())
}

fn cents_text(value: i64) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}

fn stable_id(value: &str) -> RowId {
    let mut hash = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d_u128;
    for byte in value.bytes() {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    RowId(hash)
}

fn get_detail(session: &Session, order: &str, ctx: &egui::Context) -> Result<HttpTask, HttpError> {
    session.client.start(
        HttpRequest {
            path: format!("/v1/tenants/{}/orders/{order}", session.tenant.to_uuid()),
            query: Vec::new(),
            json: None,
            bearer: Some(session.token.clone()),
        },
        ctx,
    )
}

fn decode<T: for<'de> Deserialize<'de>>(result: Result<Vec<u8>, HttpError>) -> Result<T, String> {
    result
        .map_err(|error| error.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|_| "服务器响应无效".into()))
}

fn auth_failed<T>(result: &Result<T, HttpError>) -> bool {
    matches!(result, Err(HttpError::Status(401) | HttpError::Status(403)))
}

fn editable(detail: &OrderDetail) -> bool {
    !matches!(
        detail.order.oms_status.as_str(),
        "SHIPPED" | "DONE" | "CANCELLED" | "Shipped" | "Done" | "Cancelled"
    ) && !matches!(
        detail.order.review_status.as_str(),
        "APPROVED" | "AUTO_APPROVED" | "Approved" | "AutoApproved"
    )
}

fn stable_child_id(order: &str, line: &str) -> RowId {
    stable_id(&format!("{order}\0{line}"))
}

fn valid_saved_detail(detail: &OrderDetail, selected: Option<&str>, version: i32) -> bool {
    detail.order.version > version && valid_detail(detail, selected)
}

fn valid_detail(detail: &OrderDetail, selected: Option<&str>) -> bool {
    detail.order.version > 0
        && Some(detail.order.id.as_str()) == selected
        && !detail.items.is_empty()
        && detail.items.len() <= 2_000
        && detail.items.iter().all(|item| !item.id.trim().is_empty())
        && {
            let ids: std::collections::BTreeSet<_> = detail
                .items
                .iter()
                .map(|item| stable_child_id(&detail.order.id, &item.id))
                .collect();
            ids.len() == detail.items.len()
        }
}

fn display_time(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| date.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> OrderItem {
        OrderItem {
            id: "line-1".into(),
            platform_sku_id: Some("sku-1".into()),
            outer_sku_id: None,
            title: Some("A".into()),
            quantity: 2,
            price_cent: 123,
        }
    }

    #[test]
    fn quantity_request_is_partial_and_versioned() {
        let item = item();
        let body = edit_body(
            7,
            &item,
            &CellEdit {
                row: stable_id("line-1"),
                column: QTY,
                before: "2".into(),
                after: "3".into(),
            },
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            r#"{"expected_version":7,"item_edits":[{"line_id":"line-1","qty":3}]}"#
        );
    }

    #[test]
    fn money_is_exact_and_bounded_to_cents() {
        assert_eq!(parse_cents("0"), Ok(0));
        assert_eq!(parse_cents("12.3"), Ok(1230));
        assert_eq!(parse_cents("12.34"), Ok(1234));
        assert!(parse_cents("12.345").is_err());
        assert!(parse_cents("-1").is_err());
    }

    #[test]
    fn line_identity_is_stable_and_content_sensitive() {
        assert_eq!(stable_id("line-1"), stable_id("line-1"));
        assert_ne!(stable_id("line-1"), stable_id("line-2"));
    }

    #[test]
    fn save_readback_rejects_wrong_parent_stale_version_and_duplicate_lines() {
        let mut detail = OrderDetail {
            order: OrderHead {
                id: "order-1".into(),
                version: 8,
                oms_status: "REVIEW_PENDING".into(),
                review_status: "MANUAL_REVIEW".into(),
            },
            items: vec![item()],
        };
        assert!(valid_saved_detail(&detail, Some("order-1"), 7));
        assert!(!valid_saved_detail(&detail, Some("order-2"), 7));
        assert!(!valid_saved_detail(&detail, Some("order-1"), 8));
        detail.items.push(item());
        assert!(!valid_saved_detail(&detail, Some("order-1"), 7));
        assert_ne!(
            stable_child_id("order-1", "line-1"),
            stable_child_id("order-2", "line-1")
        );
    }

    #[test]
    fn order_filter_drafts_map_to_supported_server_fields() {
        let mut state = GridState::default();
        state
            .quick_search
            .insert(ORDER_ID, FilterDraft::Text("  A-100  ".into()));
        state.quick_search.insert(
            PLATFORM,
            FilterDraft::Enum(["jd".to_string(), "pdd".to_string()].into()),
        );
        state
            .quick_search
            .insert(STATUS, FilterDraft::Enum(["NEW".to_string()].into()));
        assert_eq!(
            selection_filters(&state),
            ("A-100".into(), "jd,pdd".into(), "NEW".into())
        );
    }

    #[test]
    fn empty_order_filter_drafts_are_unrestricted() {
        assert_eq!(
            selection_filters(&GridState::default()),
            (String::new(), String::new(), String::new())
        );
    }

    #[test]
    fn order_amount_query_uses_exact_cents_and_rejects_invalid_input() {
        assert_eq!(
            amount_query(Some(&FilterDraft::Money {
                min: "12.50".into(),
                max: "".into()
            }))
            .unwrap(),
            vec![("total_amount_min_cent".into(), "1250".into())]
        );
        for (min, max) in [("-1", ""), ("13", "12.50"), ("12.501", "")] {
            assert!(
                amount_query(Some(&FilterDraft::Money {
                    min: min.into(),
                    max: max.into()
                }))
                .is_err()
            );
        }
    }
}
