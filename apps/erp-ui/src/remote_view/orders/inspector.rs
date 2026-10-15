//! Compact order detail inspector for the orders workbench.
//!
//! The parent owns row selection and the main order list.  This module owns
//! detail reads and the three detail actions that the legacy panel exposed:
//! warehouse, courier and order tags.  A write is never retried implicitly;
//! an unusable response is kept as an unknown result and blocks leaving the
//! page until the parent reads the order again.

use super::super::Session;
use crate::action_store::{
    StorageTask,
    order_recovery::{LoadedOrderRecovery, OrderRecoveryStore},
};
use crate::http::{HttpError, HttpRequest, HttpTask};
use crate::remote_actions::ActionScope;
use eframe::egui;
use erp_grid_core::RowId;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct InspectorOrder {
    #[serde(default)]
    pub(super) id: String,
    #[serde(default)]
    pub(super) platform_order_id: String,
    #[serde(default)]
    pub(super) platform: String,
    #[serde(default)]
    pub(super) shop_id: String,
    #[serde(default)]
    pub(super) receiver_name: Option<String>,
    #[serde(default)]
    pub(super) receiver_phone: Option<String>,
    #[serde(default)]
    pub(super) total_amount_cent: i64,
    #[serde(default)]
    pub(super) logistics_company: Option<String>,
    #[serde(default)]
    pub(super) ship_warehouse_id: Option<String>,
    #[serde(default)]
    pub(super) ship_warehouse_name: Option<String>,
    #[serde(default)]
    pub(super) exception_flags: Vec<String>,
    #[serde(default)]
    pub(super) freeze_reason: Option<String>,
    #[serde(default)]
    pub(super) oms_status: String,
    #[serde(default)]
    pub(super) review_status: String,
    #[serde(default)]
    pub(super) version: i32,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct InspectorDetail {
    pub(super) order: InspectorOrder,
    #[serde(default)]
    pub(super) tags: Vec<OrderTag>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct OrderTag {
    #[serde(default, alias = "name")]
    pub(super) tag: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct WarehousePage {
    #[serde(default)]
    items: Vec<WarehouseOption>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct WarehouseOption {
    #[serde(default, alias = "id")]
    warehouse_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    code: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CourierPage {
    #[serde(default)]
    items: Vec<CourierOption>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CourierOption {
    #[serde(default)]
    name: String,
    #[serde(default)]
    code: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TagDefinitionPage {
    #[serde(default)]
    items: Vec<TagDefinition>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TagDefinition {
    #[serde(default)]
    name: String,
}

#[derive(Clone, Debug, Serialize)]
struct OrderEdit {
    expected_version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    ship_warehouse_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logistics_company: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct TagsEdit {
    tags: Vec<String>,
}

enum PendingWriteKind {
    Order,
    Tags,
}

struct PendingWrite {
    task: HttpTask,
    kind: PendingWriteKind,
    order_id: String,
    expected_version: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum UnknownTarget {
    Order {
        order_id: String,
        expected_version: i32,
        warehouse_id: Option<String>,
        logistics_company: Option<String>,
    },
    Tags {
        order_id: String,
        tags: Vec<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RecoveryEnvelope {
    target: UnknownTarget,
    method: String,
    path: String,
    body: Vec<u8>,
}

struct PreparedWrite {
    target: UnknownTarget,
    method: String,
    path: String,
    body: Vec<u8>,
}

#[derive(Default)]
pub(super) struct OrderInspector {
    tag_input: String,
    selected: Option<String>,
    detail: Option<InspectorDetail>,
    detail_task: Option<HttpTask>,
    warehouse_task: Option<HttpTask>,
    courier_task: Option<HttpTask>,
    tags_task: Option<HttpTask>,
    warehouses: Vec<WarehouseOption>,
    couriers: Vec<CourierOption>,
    tag_definitions: Vec<TagDefinition>,
    pending: Option<PendingWrite>,
    unknown: bool,
    notice: Option<String>,
    failure: Option<String>,
    option_failure: Option<String>,
    unknown_target: Option<UnknownTarget>,
    changed: bool,
    scope: Option<u64>,
    recovery_identity: Option<(String, RowId, RowId)>,
    recovery_store: Option<OrderRecoveryStore>,
    recovery_opening: Option<StorageTask<LoadedOrderRecovery>>,
    recovery_saving: Option<StorageTask<u64>>,
    recovery_revision: u64,
    prepared: Option<PreparedWrite>,
    clear_after: bool,
    prepared_committed: bool,
    storage_failed: bool,
    identity_verified: bool,
}

impl OrderInspector {
    pub(super) fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub(super) fn dirty(&self) -> bool {
        self.pending.is_some()
            || self.unknown
            || self.unknown_target.is_some()
            || self.prepared.is_some()
            || self.recovery_opening.is_some()
            || self.recovery_saving.is_some()
            || self.clear_after
            || self.storage_failed
    }

    pub(super) fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    pub(super) fn leave_blocked(&self) -> Option<&'static str> {
        if self.pending.is_some()
            || self.prepared.is_some()
            || self.recovery_saving.is_some()
            || self.recovery_opening.is_some()
            || self.clear_after
        {
            Some("订单详情保存结果尚未确认")
        } else if self.unknown || self.storage_failed {
            Some("订单详情保存结果未知，必须读取最新订单核对")
        } else {
            None
        }
    }

    pub(super) fn discard_for_leave(&mut self) -> Result<(), &'static str> {
        if let Some(reason) = self.leave_blocked() {
            return Err(reason);
        }
        self.detail_task = None;
        self.failure = None;
        Ok(())
    }

    pub(super) fn reset_reads(&mut self, session: &Session) {
        self.scope = Some(session_scope(session));
        self.detail_task = None;
        self.warehouse_task = None;
        self.courier_task = None;
        self.tags_task = None;
        self.detail = None;
        self.selected = None;
        self.failure = None;
        self.option_failure = None;
        self.warehouses.clear();
        self.couriers.clear();
        self.tag_definitions.clear();
    }

    pub(super) fn sync_recovery_identity(&mut self, identity: Option<(&str, RowId, RowId)>) {
        let next = identity.map(|(server, tenant, actor)| (server.to_owned(), tenant, actor));
        self.identity_verified = next.is_some() && self.recovery_identity == next;
        if self.recovery_identity == next {
            return;
        }
        if self.dirty() {
            self.notice = Some("账号身份变化，订单详情写入保持隔离且禁止派发".into());
            return;
        }
        self.recovery_identity = next;
        self.identity_verified = self.recovery_identity.is_some();
        self.scope = None;
        self.recovery_store = None;
        self.recovery_opening = None;
        self.recovery_saving = None;
        self.recovery_revision = 0;
    }

    /// Synchronises the inspector with the row selected by the parent.  A
    /// dirty inspector intentionally keeps its original order identity.
    pub(super) fn sync_selection(
        &mut self,
        order_id: Option<&str>,
        session: &Session,
        ctx: &egui::Context,
    ) {
        if self.scope != Some(session_scope(session)) {
            if self.dirty() {
                return;
            }
            self.reset_reads(session);
        }
        let next = order_id.map(str::trim).filter(|id| !id.is_empty());
        if self.selected.as_deref() == next {
            return;
        }
        if self.dirty() {
            return;
        }
        self.tag_input.clear();
        self.selected = next.map(str::to_owned);
        self.detail = None;
        self.failure = None;
        self.notice = None;
        self.detail_task = None;
        let Some(id) = self.selected.clone() else {
            return;
        };
        self.detail_task = request(
            session,
            format!("/v1/tenants/{}/orders/{id}", session.tenant.to_uuid()),
            Vec::new(),
            None,
            ctx,
        )
        .ok();
        self.ensure_options(session, ctx);
    }

    fn ensure_options(&mut self, session: &Session, ctx: &egui::Context) {
        if self.warehouse_task.is_none() && self.warehouses.is_empty() {
            self.warehouse_task = request(
                session,
                format!("/v1/tenants/{}/warehouses", session.tenant.to_uuid()),
                vec![
                    ("status".into(), "active".into()),
                    ("offset".into(), "0".into()),
                    ("limit".into(), "200".into()),
                ],
                None,
                ctx,
            )
            .ok();
        }
        if self.courier_task.is_none() && self.couriers.is_empty() {
            self.courier_task = request(
                session,
                format!("/v1/tenants/{}/couriers", session.tenant.to_uuid()),
                vec![
                    ("offset".into(), "0".into()),
                    ("limit".into(), "200".into()),
                ],
                None,
                ctx,
            )
            .ok();
        }
        if self.tags_task.is_none() && self.tag_definitions.is_empty() {
            self.tags_task = request(
                session,
                format!(
                    "/v1/tenants/{}/order-tag-definitions",
                    session.tenant.to_uuid()
                ),
                vec![
                    ("status".into(), "ACTIVE".into()),
                    ("offset".into(), "0".into()),
                    ("limit".into(), "200".into()),
                ],
                None,
                ctx,
            )
            .ok();
        }
    }

    pub(super) fn tick(&mut self, ctx: &egui::Context, session: &Session) -> bool {
        if !self.identity_verified
            || !self
                .recovery_identity
                .as_ref()
                .is_some_and(|(server, tenant, _)| {
                    server == session.client.base_url() && *tenant == session.tenant
                })
        {
            return false;
        }
        self.scope = Some(session_scope(session));
        let mut auth_failed = false;
        self.tick_recovery(ctx, session);
        self.dispatch_prepared(ctx, session);
        if let Some(result) = self.detail_task.as_mut().and_then(HttpTask::poll) {
            self.detail_task = None;
            auth_failed |= is_auth(&result);
            match decode::<InspectorDetail>(result) {
                Ok(detail) if self.selected.as_deref() == Some(detail.order.id.as_str()) => {
                    self.detail = Some(detail);
                    self.failure = None;
                    self.resolve_unknown_after_read(ctx);
                }
                Ok(_) => self.failure = Some("订单详情与当前选择不一致".into()),
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(result) = self.warehouse_task.as_mut().and_then(HttpTask::poll) {
            self.warehouse_task = None;
            auth_failed |= is_auth(&result);
            match decode::<WarehousePage>(result) {
                Ok(page) => self.warehouses = page.items,
                Err(error) => self.option_failure = Some(format!("仓库列表：{error}")),
            }
        }
        if let Some(result) = self.courier_task.as_mut().and_then(HttpTask::poll) {
            self.courier_task = None;
            auth_failed |= is_auth(&result);
            match decode::<CourierPage>(result) {
                Ok(page) => self.couriers = page.items,
                Err(error) => self.option_failure = Some(format!("物流列表：{error}")),
            }
        }
        if let Some(result) = self.tags_task.as_mut().and_then(HttpTask::poll) {
            self.tags_task = None;
            auth_failed |= is_auth(&result);
            match decode::<TagDefinitionPage>(result) {
                Ok(page) => self.tag_definitions = page.items,
                Err(error) => self.option_failure = Some(format!("标记列表：{error}")),
            }
        }
        if let Some(result) = self
            .pending
            .as_mut()
            .and_then(|pending| pending.task.poll())
        {
            auth_failed |= is_auth(&result);
            let Some(pending) = self.pending.take() else {
                return auth_failed;
            };
            match result {
                Ok(body) if matches!(&pending.kind, PendingWriteKind::Tags) => {
                    match decode::<Vec<OrderTag>>(Ok(body)) {
                        Ok(tags) if self.unknown_target.as_ref().is_some_and(|target| matches!(target, UnknownTarget::Tags { order_id, tags: expected } if self.selected.as_ref() == Some(order_id) && normalized_tags(&tags) == *expected)) => {
                            if let Some(detail) = &mut self.detail {
                                detail.tags = tags;
                                self.unknown = false;
                                self.unknown_target = None;
                                self.changed = true;
                                self.clear_checkpoint(ctx);
                                self.notice = Some("订单标记已保存".into());
                            } else {
                                self.unknown = true;
                                self.notice =
                                    Some("标记保存回执无法关联订单，必须读取最新订单".into());
                            }
                        }
                        Ok(_) | Err(_) => {
                            self.unknown = true;
                            self.notice = Some("标记保存回执无法核对，禁止重发".into());
                        }
                    }
                }
                Ok(body) => match decode::<InspectorDetail>(Ok(body)) {
                    Ok(detail)
                        if detail.order.id == pending.order_id
                            && detail.order.version > pending.expected_version
                            && self.unknown_target.as_ref().is_some_and(|target| {
                                unknown_order_matches(target, &detail.order)
                            }) =>
                    {
                        self.detail = Some(detail);
                        self.unknown = false;
                        self.unknown_target = None;
                        self.changed = true;
                        self.clear_checkpoint(ctx);
                        self.notice = Some("订单详情已保存".into());
                    }
                    _ => {
                        self.unknown = true;
                        self.notice = Some("订单保存回执无法核对，必须读取最新订单".into());
                    }
                },
                Err(error) => {
                    if matches!(error, HttpError::Status(409)) {
                        self.unknown = false;
                        self.unknown_target = None;
                        self.clear_checkpoint(ctx);
                        self.notice = Some("订单版本冲突，已拒绝保存；请读取最新订单".into());
                    } else {
                        if matches!(
                            error,
                            HttpError::Transport | HttpError::TooLarge | HttpError::Cancelled
                        ) || matches!(error, HttpError::Status(code) if code >= 500)
                        {
                            self.unknown = true;
                        }
                        if matches!(error, HttpError::Status(code) if (400..500).contains(&code)) {
                            self.unknown_target = None;
                            self.clear_checkpoint(ctx);
                        }
                        self.notice = Some(error.to_string());
                    }
                }
            }
        }
        if self.detail_task.is_some() || self.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        auth_failed
    }

    fn tick_recovery(&mut self, ctx: &egui::Context, session: &Session) {
        let Some((server, tenant, actor)) = self.recovery_identity.clone() else {
            return;
        };
        if server != session.client.base_url() || tenant != session.tenant {
            return;
        }
        if self.recovery_store.is_none() && self.recovery_opening.is_none() && !self.storage_failed
        {
            match ActionScope::new(&session.client, tenant, actor)
                .and_then(|scope| OrderRecoveryStore::open_inspector(scope, ctx))
            {
                Ok(task) => self.recovery_opening = Some(task),
                Err(error) => {
                    self.storage_failed = true;
                    self.notice = Some(error);
                }
            }
        }
        if let Some(result) = self.recovery_opening.as_mut().and_then(StorageTask::poll) {
            self.recovery_opening = None;
            match result {
                Ok(loaded) => {
                    self.recovery_revision = loaded.revision;
                    self.recovery_store = Some(loaded.store);
                    if let Some(bytes) = loaded.bytes {
                        match serde_json::from_slice::<RecoveryEnvelope>(&bytes) {
                            Ok(envelope) => {
                                let order = match &envelope.target {
                                    UnknownTarget::Order { order_id, .. }
                                    | UnknownTarget::Tags { order_id, .. } => order_id.clone(),
                                };
                                self.selected = Some(order);
                                self.unknown_target = Some(envelope.target);
                                self.unknown = true;
                                self.reload_detail(session, ctx);
                                self.notice =
                                    Some("已恢复订单详情写入记录，正在核对最新订单".into());
                            }
                            Err(_) => {
                                self.storage_failed = true;
                                self.notice = Some("订单详情恢复记录损坏，已保留并禁止写入".into());
                            }
                        }
                    }
                }
                Err(error) => {
                    self.storage_failed = true;
                    self.notice = Some(error);
                }
            }
        }
        if let Some(result) = self.recovery_saving.as_mut().and_then(StorageTask::poll) {
            self.recovery_saving = None;
            match result {
                Ok(revision) => {
                    self.recovery_revision = revision;
                    if self.clear_after {
                        self.clear_after = false;
                    } else {
                        self.prepared_committed = true;
                    }
                }
                Err(error) => {
                    self.storage_failed = true;
                    self.notice = Some(format!("恢复记录写入失败，禁止派发：{error}"));
                }
            }
        }
        if !self.storage_failed
            && self.recovery_saving.is_none()
            && self.prepared.is_some()
            && !self.clear_after
            && !self.prepared_committed
        {
            self.persist_prepared(ctx);
        }
        if self.recovery_opening.is_some() || self.recovery_saving.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn dispatch_prepared(&mut self, ctx: &egui::Context, session: &Session) {
        if self.pending.is_some()
            || self.recovery_saving.is_some()
            || !self.prepared_committed
            || self.storage_failed
            || !self.identity_verified
        {
            return;
        }
        let Some(prepared) = self.prepared.take() else {
            return;
        };
        self.prepared_committed = false;
        let request = HttpRequest {
            path: prepared.path.clone(),
            query: Vec::new(),
            json: Some(prepared.body.clone()),
            bearer: Some(session.token.clone()),
            idempotency_key: None,
        };
        let task = if prepared.method == "PUT" {
            session.client.put(request, ctx)
        } else {
            session.client.start(request, ctx)
        };
        match task {
            Ok(task) => {
                let (order_id, expected_version) = match &prepared.target {
                    UnknownTarget::Order {
                        order_id,
                        expected_version,
                        ..
                    } => (order_id.clone(), *expected_version),
                    UnknownTarget::Tags { order_id, .. } => (order_id.clone(), 0),
                };
                self.pending = Some(PendingWrite {
                    task,
                    kind: if prepared.method == "PUT" {
                        PendingWriteKind::Order
                    } else {
                        PendingWriteKind::Tags
                    },
                    order_id,
                    expected_version,
                });
            }
            Err(error) => {
                self.unknown = true;
                self.notice = Some(format!("请求未派发：{error}，请读取最新核对"));
            }
        }
    }

    pub(super) fn ui(&mut self, ui: &mut egui::Ui, session: &Session) {
        let Some(detail) = self.detail.clone() else {
            if let Some(error) = &self.failure {
                ui.colored_label(egui::Color32::RED, error);
            } else if self.selected.is_some() {
                ui.spinner();
            }
            if let Some(error) = self.option_failure.clone() {
                ui.colored_label(egui::Color32::RED, error);
                if ui.button("重试选项").clicked() {
                    self.option_failure = None;
                    self.warehouses.clear();
                    self.couriers.clear();
                    self.tag_definitions.clear();
                    self.ensure_options(session, ui.ctx());
                }
            }
            return;
        };
        let order = detail.order;
        ui.horizontal(|ui| {
            ui.label("订单详情");
            ui.label(order.platform_order_id.clone());
        });
        ui.separator();
        field(ui, "平台", super::platform_label(&order.platform));
        field(ui, "店铺", &order.shop_id);
        field_opt(ui, "收件人", order.receiver_name.as_deref());
        field_opt(ui, "电话", order.receiver_phone.as_deref());
        field(
            ui,
            "金额",
            &format!(
                "¥{}.{:02}",
                order.total_amount_cent / 100,
                order.total_amount_cent.abs() % 100
            ),
        );
        field(ui, "状态", super::status_label(&order.oms_status));
        field(ui, "审核", &order.review_status);
        if !order.exception_flags.is_empty() {
            field(ui, "异常", &order.exception_flags.join(", "));
        }
        if let Some(reason) = order
            .freeze_reason
            .as_deref()
            .filter(|v| !v.trim().is_empty())
        {
            field(ui, "冻结", reason);
        }
        ui.separator();
        let enabled = !self.dirty() && self.identity_verified && self.recovery_store.is_some();
        if let Some(error) = self.option_failure.clone() {
            ui.colored_label(egui::Color32::RED, error);
            if ui.button("重试选项").clicked() {
                self.option_failure = None;
                self.warehouses.clear();
                self.couriers.clear();
                self.tag_definitions.clear();
                self.ensure_options(session, ui.ctx());
            }
        }
        if (self.unknown_target.is_some() || self.notice.is_some())
            && self.pending.is_none()
            && self.prepared.is_none()
        {
            ui.colored_label(
                egui::Color32::YELLOW,
                "保存结果可能已写入，请读取最新订单核对",
            );
            if ui.button("读取最新").clicked() {
                self.reload_detail(session, ui.ctx());
            }
        }
        ui.label("发货仓库");
        let mut warehouse = order.ship_warehouse_id.clone().unwrap_or_default();
        let warehouse_options = self.warehouses.clone();
        egui::ComboBox::from_id_salt("order-inspector-warehouse")
            .selected_text(
                order
                    .ship_warehouse_name
                    .clone()
                    .or_else(|| option_name(&self.warehouses, &warehouse))
                    .unwrap_or_else(|| "未分配".into()),
            )
            .show_ui(ui, |ui| {
                for item in &warehouse_options {
                    let id = item.warehouse_id.trim();
                    if id.is_empty() {
                        continue;
                    }
                    if ui
                        .selectable_value(
                            &mut warehouse,
                            id.to_owned(),
                            display_name(&item.name, &item.code, id),
                        )
                        .changed()
                        && enabled
                    {
                        self.update_order(
                            session,
                            ui.ctx(),
                            OrderEdit {
                                expected_version: order.version,
                                ship_warehouse_id: Some(warehouse.clone()),
                                logistics_company: None,
                            },
                        );
                    }
                }
            });
        ui.label("物流公司");
        let mut courier = order.logistics_company.clone().unwrap_or_default();
        let courier_options = self.couriers.clone();
        egui::ComboBox::from_id_salt("order-inspector-courier")
            .selected_text(if courier.is_empty() {
                "未选择".into()
            } else {
                courier.clone()
            })
            .show_ui(ui, |ui| {
                for item in &courier_options {
                    let value = if item.name.trim().is_empty() {
                        item.code.clone()
                    } else {
                        item.name.clone()
                    };
                    if value.is_empty() {
                        continue;
                    }
                    if ui
                        .selectable_value(&mut courier, value.clone(), value.clone())
                        .changed()
                        && enabled
                    {
                        self.update_order(
                            session,
                            ui.ctx(),
                            OrderEdit {
                                expected_version: order.version,
                                ship_warehouse_id: None,
                                logistics_company: Some(courier.clone()),
                            },
                        );
                    }
                }
            });
        let current_tags = detail
            .tags
            .iter()
            .map(|tag| tag.tag.trim())
            .filter(|tag| !tag.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        ui.horizontal_wrapped(|ui| {
            ui.label("标记");
            for tag in &current_tags {
                if ui
                    .add_enabled(enabled, egui::Button::new(format!("{tag} ×")))
                    .on_hover_text("移除标记")
                    .clicked()
                {
                    let tags = current_tags
                        .iter()
                        .filter(|item| *item != tag)
                        .cloned()
                        .collect();
                    self.update_tags(session, ui.ctx(), tags);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut self.tag_input)
                    .hint_text("订单标记")
                    .char_limit(64)
                    .desired_width(130.0),
            );
            if ui
                .add_enabled(
                    enabled && !self.tag_input.trim().is_empty(),
                    egui::Button::new("添加"),
                )
                .clicked()
            {
                let mut tags = current_tags.clone();
                let tag = self.tag_input.trim().to_owned();
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
                self.update_tags(session, ui.ctx(), tags);
                self.tag_input.clear();
            }
        });
        let tag_definitions = self.tag_definitions.clone();
        for definition in &tag_definitions {
            let tag = definition.name.trim();
            if tag.is_empty()
                || current_tags
                    .iter()
                    .any(|item| item.eq_ignore_ascii_case(tag))
            {
                continue;
            }
            if ui
                .add_enabled(enabled, egui::Button::new(format!("+{tag}")))
                .clicked()
            {
                let mut tags = current_tags.clone();
                tags.push(tag.to_owned());
                self.update_tags(session, ui.ctx(), tags);
            }
        }
        if self.storage_failed && ui.button("重新打开恢复记录").clicked() {
            self.storage_failed = false;
            self.recovery_store = None;
            self.prepared = None;
            self.prepared_committed = false;
            self.clear_after = false;
        }
        if let Some(notice) = &self.notice {
            ui.label(notice);
        }
    }

    fn update_order(&mut self, session: &Session, ctx: &egui::Context, body: OrderEdit) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let expected_version = body.expected_version;
        let json = match serde_json::to_vec(&body) {
            Ok(json) => json,
            Err(error) => {
                self.notice = Some(error.to_string());
                return;
            }
        };
        let target = UnknownTarget::Order {
            order_id: id.clone(),
            expected_version,
            warehouse_id: body.ship_warehouse_id.clone(),
            logistics_company: body.logistics_company.clone(),
        };
        self.stage_write(
            session,
            ctx,
            target,
            "PUT",
            format!("/v1/tenants/{}/orders/{id}", session.tenant.to_uuid()),
            json,
        );
    }

    fn update_tags(&mut self, session: &Session, ctx: &egui::Context, tags: Vec<String>) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let json = match serde_json::to_vec(&TagsEdit { tags: tags.clone() }) {
            Ok(json) => json,
            Err(error) => {
                self.notice = Some(error.to_string());
                return;
            }
        };
        let target = UnknownTarget::Tags {
            order_id: id.clone(),
            tags: tags
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
        };
        self.stage_write(
            session,
            ctx,
            target,
            "POST",
            format!("/v1/tenants/{}/orders/{id}/tags", session.tenant.to_uuid()),
            json,
        );
    }

    fn stage_write(
        &mut self,
        _session: &Session,
        ctx: &egui::Context,
        target: UnknownTarget,
        method: &str,
        path: String,
        body: Vec<u8>,
    ) {
        let envelope = RecoveryEnvelope {
            target: target.clone(),
            method: method.into(),
            path: path.clone(),
            body: body.clone(),
        };
        if serde_json::to_vec(&envelope).is_err() {
            self.notice = Some("订单恢复记录编码失败".into());
            return;
        }
        self.prepared_committed = false;
        self.unknown_target = Some(target.clone());
        self.prepared = Some(PreparedWrite {
            target,
            method: method.into(),
            path,
            body,
        });
        if self.recovery_store.is_some() {
            self.persist_prepared(ctx);
        } else {
            self.notice = Some("正在打开订单恢复存储，尚未派发".into());
        }
    }

    fn persist_prepared(&mut self, ctx: &egui::Context) {
        let Some(prepared) = &self.prepared else {
            return;
        };
        let envelope = RecoveryEnvelope {
            target: prepared.target.clone(),
            method: prepared.method.clone(),
            path: prepared.path.clone(),
            body: prepared.body.clone(),
        };
        let Ok(bytes) = serde_json::to_vec(&envelope) else {
            return;
        };
        let Some(store) = &self.recovery_store else {
            return;
        };
        match store.commit(self.recovery_revision, Some(&bytes), ctx) {
            Ok(task) => self.recovery_saving = Some(task),
            Err(error) => {
                self.storage_failed = true;
                self.notice = Some(error);
            }
        }
    }

    fn clear_checkpoint(&mut self, ctx: &egui::Context) {
        self.prepared = None;
        self.prepared_committed = false;
        self.clear_after = true;
        let Some(store) = &self.recovery_store else {
            self.clear_after = false;
            return;
        };
        match store.commit(self.recovery_revision, None, ctx) {
            Ok(task) => {
                self.recovery_saving = Some(task);
                self.clear_after = true;
            }
            Err(error) => {
                self.storage_failed = true;
                self.notice = Some(format!("保存已确认，但清理恢复记录失败：{error}"));
            }
        }
    }

    fn reload_detail(&mut self, session: &Session, ctx: &egui::Context) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        self.detail_task = request(
            session,
            format!("/v1/tenants/{}/orders/{id}", session.tenant.to_uuid()),
            Vec::new(),
            None,
            ctx,
        )
        .ok();
    }

    fn resolve_unknown_after_read(&mut self, ctx: &egui::Context) {
        let Some(target) = self.unknown_target.clone() else {
            return;
        };
        let Some(detail) = self.detail.as_ref() else {
            return;
        };
        let matches = match target {
            UnknownTarget::Order { .. } => unknown_order_matches(&target, &detail.order),
            UnknownTarget::Tags { order_id, tags } => {
                detail.order.id == order_id && normalized_tags(&detail.tags) == tags
            }
        };
        if matches {
            self.unknown = false;
            self.unknown_target = None;
            self.changed = true;
            self.clear_checkpoint(ctx);
            self.notice = Some("已通过最新订单确认保存结果".into());
        } else {
            self.notice = Some("最新订单未确认原保存目标，禁止重发".into());
        }
    }
}

fn request(
    session: &Session,
    path: String,
    query: Vec<(String, String)>,
    json: Option<Vec<u8>>,
    ctx: &egui::Context,
) -> Result<HttpTask, HttpError> {
    let request = HttpRequest {
        path,
        query,
        json,
        bearer: Some(session.token.clone()),
        idempotency_key: None,
    };
    session.client.start(request, ctx)
}

fn decode<T: for<'de> Deserialize<'de>>(result: Result<Vec<u8>, HttpError>) -> Result<T, String> {
    result
        .map_err(|error| error.to_string())
        .and_then(|body| serde_json::from_slice(&body).map_err(|_| "服务器响应无效".into()))
}

fn is_auth<T>(result: &Result<T, HttpError>) -> bool {
    matches!(result, Err(HttpError::Status(401)))
}
fn field(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(if value.is_empty() { "-" } else { value });
    });
}
fn field_opt(ui: &mut egui::Ui, label: &str, value: Option<&str>) {
    field(ui, label, value.unwrap_or(""));
}
fn display_name(name: &str, code: &str, fallback: &str) -> String {
    if !name.trim().is_empty() {
        name.into()
    } else if !code.trim().is_empty() {
        code.into()
    } else {
        fallback.into()
    }
}
fn option_name(items: &[WarehouseOption], id: &str) -> Option<String> {
    items
        .iter()
        .find(|item| item.warehouse_id == id)
        .map(|item| display_name(&item.name, &item.code, id))
}

fn normalized_tags(tags: &[OrderTag]) -> Vec<String> {
    tags.iter()
        .map(|tag| tag.tag.trim().to_owned())
        .filter(|tag| !tag.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn session_scope(session: &Session) -> u64 {
    let mut hasher = DefaultHasher::new();
    session.client.base_url().hash(&mut hasher);
    session.tenant.hash(&mut hasher);
    session.token.hash(&mut hasher);
    hasher.finish()
}

fn unknown_order_matches(target: &UnknownTarget, order: &InspectorOrder) -> bool {
    let UnknownTarget::Order {
        order_id,
        expected_version,
        warehouse_id,
        logistics_company,
    } = target
    else {
        return false;
    };
    order.id == *order_id
        && order.version > *expected_version
        && warehouse_id
            .as_deref()
            .is_none_or(|value| order.ship_warehouse_id.as_deref() == Some(value))
        && logistics_company
            .as_deref()
            .is_none_or(|value| order.logistics_company.as_deref() == Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_normalization_discards_empty_values_without_reordering() {
        let tags = vec![
            OrderTag {
                tag: "  VIP ".into(),
            },
            OrderTag { tag: "".into() },
            OrderTag {
                tag: "异常".into()
            },
        ];
        assert_eq!(
            normalized_tags(&tags),
            vec!["VIP".to_owned(), "异常".to_owned()]
        );
    }

    #[test]
    fn unknown_order_target_requires_newer_version_and_requested_fields() {
        let target = UnknownTarget::Order {
            order_id: "o-1".into(),
            expected_version: 4,
            warehouse_id: Some("w-1".into()),
            logistics_company: Some("顺丰".into()),
        };
        let order = InspectorOrder {
            id: "o-1".into(),
            version: 5,
            ship_warehouse_id: Some("w-1".into()),
            logistics_company: Some("顺丰".into()),
            ..Default::default()
        };
        assert!(unknown_order_matches(&target, &order));
        assert!(!unknown_order_matches(
            &target,
            &InspectorOrder {
                version: 4,
                ..order.clone()
            }
        ));
        assert!(!unknown_order_matches(
            &target,
            &InspectorOrder {
                logistics_company: Some("圆通".into()),
                ..order
            }
        ));
    }
}
