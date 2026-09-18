//! Order-review bottom modules migrated from the legacy detail tabs.
//!
//! The module deliberately consumes the two existing read contracts only:
//! order detail and the bounded shipment list.  It does not manufacture shipment, fee or
//! log rows when the service has not returned them.

use super::super::Session;
use crate::http::{HttpRequest, HttpTask};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId};
use erp_ui_egui::{DataGrid, GridState};
use serde_json::Value;

const SOURCE: ColumnId = ColumnId(1);
const TYPE: ColumnId = ColumnId(2);
const NUMBER: ColumnId = ColumnId(3);
const STATUS: ColumnId = ColumnId(4);
const AMOUNT: ColumnId = ColumnId(5);
const TIME: ColumnId = ColumnId(6);
const SHIPMENT_NO: ColumnId = ColumnId(11);
const SHIPMENT_STATUS: ColumnId = ColumnId(12);
const SHIPPED_AT: ColumnId = ColumnId(13);
const WAREHOUSE: ColumnId = ColumnId(14);
const CARRIER: ColumnId = ColumnId(15);
const TRACKING: ColumnId = ColumnId(16);
const FEE_NAME: ColumnId = ColumnId(21);
const FEE_AMOUNT: ColumnId = ColumnId(22);
const REMARK: ColumnId = ColumnId(23);
const LOG_TIME: ColumnId = ColumnId(31);
const ACTION: ColumnId = ColumnId(32);
const OPERATOR: ColumnId = ColumnId(33);
const DETAIL: ColumnId = ColumnId(34);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Goods,
    Platform,
    Shipments,
    Fees,
    Logs,
}

struct Pending {
    generation: u64,
    detail: Option<HttpTask>,
    chain: Option<HttpTask>,
}

pub(super) struct DetailModules {
    platform_rows: Vec<Value>,
    fee_rows: Vec<Value>,
    log_rows: Vec<Value>,
    shipment_rows: Vec<Value>,
    order_id: Option<String>,
    generation: u64,
    pending: Option<Pending>,
    detail: Option<Value>,
    chain: Option<Value>,
    error: Option<String>,
    chain_error: Option<String>,
    retry_requested: bool,
    tab: Tab,
    shipment_offset: usize,
    shipment_total: usize,
    platform_state: GridState,
    shipment_state: GridState,
    fee_state: GridState,
    log_state: GridState,
    platform_columns: Vec<Column>,
    shipment_columns: Vec<Column>,
    fee_columns: Vec<Column>,
    log_columns: Vec<Column>,
}

impl Default for DetailModules {
    fn default() -> Self {
        Self {
            platform_rows: Vec::new(),
            fee_rows: Vec::new(),
            log_rows: Vec::new(),
            shipment_rows: Vec::new(),
            order_id: None,
            generation: 0,
            pending: None,
            detail: None,
            chain: None,
            error: None,
            chain_error: None,
            retry_requested: false,
            tab: Tab::Goods,
            shipment_offset: 0,
            shipment_total: 0,
            platform_state: read_only_state(),
            shipment_state: read_only_state(),
            fee_state: read_only_state(),
            log_state: read_only_state(),
            platform_columns: vec![
                Column::new(SOURCE, "订单来源", 120.0),
                Column::new(TYPE, "订单类型", 120.0),
                Column::new(NUMBER, "订单号", 220.0),
                Column::new(STATUS, "订单状态", 140.0),
                Column::new(AMOUNT, "金额", 120.0),
                Column::new(TIME, "时间", 180.0),
            ],
            shipment_columns: vec![
                Column::new(SHIPMENT_NO, "发货单号", 180.0),
                Column::new(SHIPMENT_STATUS, "发货状态", 120.0),
                Column::new(SHIPPED_AT, "发货时间", 180.0),
                Column::new(WAREHOUSE, "仓库", 120.0),
                Column::new(CARRIER, "物流公司", 140.0),
                Column::new(TRACKING, "物流单号", 180.0),
            ],
            fee_columns: vec![
                Column::new(FEE_NAME, "费用项", 180.0),
                Column::new(FEE_AMOUNT, "金额", 120.0),
                Column::new(REMARK, "备注", 220.0),
            ],
            log_columns: vec![
                Column::new(LOG_TIME, "时间", 180.0),
                Column::new(ACTION, "操作", 140.0),
                Column::new(OPERATOR, "操作人", 120.0),
                Column::new(DETAIL, "详情", 360.0),
            ],
        }
    }
}

impl DetailModules {
    /// Starts the shipment request; Orders owns the detail request.
    pub(super) fn load(&mut self, session: &Session, ctx: &egui::Context, order_id: &str) {
        if self.order_id.as_deref() != Some(order_id) {
            self.clear();
            self.shipment_offset = 0;
        }
        self.generation = self.generation.wrapping_add(1);
        self.shipment_state.bind_view(self.generation);
        let generation = self.generation;
        self.pending = None;
        self.chain = None;
        self.error = None;
        self.chain_error = None;
        self.retry_requested = false;
        self.order_id = Some(order_id.to_owned());
        let chain = session.client.start(
            HttpRequest {
                path: format!("/v1/tenants/{}/oms/shipments", session.tenant.to_uuid()),
                query: vec![
                    ("order_id".into(), order_id.into()),
                    ("offset".into(), self.shipment_offset.to_string()),
                    ("limit".into(), "100".into()),
                ],
                json: None,
                bearer: Some(session.token.clone()),
            },
            ctx,
        );
        match chain {
            Ok(chain) => {
                self.pending = Some(Pending {
                    generation,
                    detail: None,
                    chain: Some(chain),
                });
            }
            Err(error) => self.chain_error = Some(error.to_string()),
        }
    }

    pub(super) fn accept_detail(&mut self, bytes: &[u8]) {
        match serde_json::from_slice::<Value>(bytes) {
            Ok(value)
                if value.pointer("/order/id").and_then(Value::as_str)
                    == self.order_id.as_deref() =>
            {
                self.detail = Some(value);
                self.platform_rows = self.platform_rows();
                self.fee_rows = self.fee_rows();
                self.log_rows = array_at(self.detail.as_ref(), "audit_logs");
                self.generation = self.generation.wrapping_add(1);
                self.platform_state.bind_view(self.generation);
                self.fee_state.bind_view(self.generation);
                self.log_state.bind_view(self.generation);
                self.error = None;
            }
            _ => self.error = Some("订单详情响应无效或身份不符".into()),
        }
    }

    pub(super) fn retry(&mut self, session: &Session, ctx: &egui::Context) {
        if let Some(order_id) = self.order_id.clone() {
            self.load(session, ctx, &order_id);
        }
    }

    pub(super) fn take_retry_requested(&mut self) -> bool {
        let requested = self.retry_requested;
        self.retry_requested = false;
        requested
    }

    /// Polls the two requests and drops late results from a previous order.
    pub(super) fn tick(&mut self) -> bool {
        let Some(pending) = self.pending.as_mut() else {
            return false;
        };
        let detail = pending.detail.as_mut().and_then(HttpTask::poll);
        let chain = pending.chain.as_mut().and_then(HttpTask::poll);
        if detail.is_none() && chain.is_none() {
            return false;
        }
        let mut authentication_failed = false;
        let done = pending.detail.is_none() || detail.is_some();
        let chain_done = pending.chain.is_none() || chain.is_some();
        if let Some(result) = detail {
            match result {
                Ok(bytes) => match serde_json::from_slice(&bytes) {
                    Ok(value) => self.detail = Some(value),
                    Err(_) => self.error = Some("订单详情响应无效".into()),
                },
                Err(error) => {
                    authentication_failed |= matches!(&error, crate::http::HttpError::Status(401));
                    self.chain_error = Some(error.to_string());
                }
            }
            pending.detail = None;
        }
        if let Some(result) = chain {
            match result {
                Ok(bytes) => match serde_json::from_slice(&bytes) {
                    Ok(value) => {
                        let value: Value = value;
                        self.shipment_total =
                            value.get("total").and_then(Value::as_u64).unwrap_or(0) as usize;
                        self.shipment_rows = array_at(Some(&value), "items");
                        self.chain = Some(value);
                    }
                    Err(_) => self.chain_error = Some("发货单响应无效".into()),
                },
                Err(error) => {
                    authentication_failed |= matches!(&error, crate::http::HttpError::Status(401));
                    self.chain_error = Some(error.to_string());
                }
            }
            pending.chain = None;
        }
        if done && chain_done && pending.generation == self.generation {
            self.pending = None;
        }
        authentication_failed
    }

    pub(super) fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.platform_rows.clear();
        self.fee_rows.clear();
        self.log_rows.clear();
        self.shipment_rows.clear();
        self.order_id = None;
        self.detail = None;
        self.chain = None;
        self.error = None;
        self.chain_error = None;
        self.retry_requested = false;
        self.platform_state.bind_view(self.generation);
        self.shipment_state.bind_view(self.generation);
        self.fee_state.bind_view(self.generation);
        self.log_state.bind_view(self.generation);
    }

    pub(super) fn selected_goods(&self) -> bool {
        self.tab == Tab::Goods
    }

    /// Returns true when the caller should display its existing goods grid.
    /// The four migrated detail tabs own all other grid rendering here.
    pub(super) fn ui(&mut self, ui: &mut egui::Ui, can_switch: bool) -> bool {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    can_switch,
                    egui::Button::selectable(self.tab == Tab::Goods, "货品"),
                )
                .clicked()
            {
                self.tab = Tab::Goods;
            }
            if ui
                .add_enabled(
                    can_switch,
                    egui::Button::selectable(self.tab == Tab::Platform, "平台订单"),
                )
                .clicked()
            {
                self.tab = Tab::Platform;
            }
            if ui
                .add_enabled(
                    can_switch,
                    egui::Button::selectable(self.tab == Tab::Shipments, "发货单"),
                )
                .clicked()
            {
                self.tab = Tab::Shipments;
            }
            if ui
                .add_enabled(
                    can_switch,
                    egui::Button::selectable(self.tab == Tab::Fees, "订单费用"),
                )
                .clicked()
            {
                self.tab = Tab::Fees;
            }
            if ui
                .add_enabled(
                    can_switch,
                    egui::Button::selectable(self.tab == Tab::Logs, "日志"),
                )
                .clicked()
            {
                self.tab = Tab::Logs;
            }
        });
        if self.selected_goods() {
            return true;
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
            return false;
        }
        if self.pending.is_some() && self.tab == Tab::Shipments {
            ui.spinner();
            return false;
        }
        match self.tab {
            Tab::Goods => {}
            Tab::Platform => self.platform_ui(ui),
            Tab::Shipments => self.shipments_ui(ui),
            Tab::Fees => self.fees_ui(ui),
            Tab::Logs => self.logs_ui(ui),
        }
        false
    }

    fn platform_ui(&mut self, ui: &mut egui::Ui) {
        let rows = &self.platform_rows;
        show_grid(
            ui,
            "order-platform-orders",
            &mut self.platform_columns,
            &mut self.platform_state,
            rows,
            platform_cell,
        );
    }

    fn shipments_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = &self.chain_error {
            ui.colored_label(egui::Color32::RED, error);
            if ui.button("重试发货单").clicked() {
                self.retry_requested = true;
            }
            return;
        }
        ui.horizontal(|ui| {
            ui.label(format!("{} 条", self.shipment_total));
            if ui
                .add_enabled(self.shipment_offset > 0, egui::Button::new("上一页"))
                .clicked()
            {
                self.shipment_offset = self.shipment_offset.saturating_sub(100);
                self.retry_requested = true;
            }
            if ui
                .add_enabled(
                    self.shipment_offset + 100 < self.shipment_total,
                    egui::Button::new("下一页"),
                )
                .clicked()
            {
                self.shipment_offset += 100;
                self.retry_requested = true;
            }
        });
        let rows = &self.shipment_rows;
        show_grid(
            ui,
            "order-shipments",
            &mut self.shipment_columns,
            &mut self.shipment_state,
            rows,
            shipment_cell,
        );
    }

    fn fees_ui(&mut self, ui: &mut egui::Ui) {
        let rows = &self.fee_rows;
        show_grid(
            ui,
            "order-fees",
            &mut self.fee_columns,
            &mut self.fee_state,
            rows,
            fee_cell,
        );
    }

    fn logs_ui(&mut self, ui: &mut egui::Ui) {
        let rows = &self.log_rows;
        show_grid(
            ui,
            "order-logs",
            &mut self.log_columns,
            &mut self.log_state,
            rows,
            log_cell,
        );
    }

    fn platform_rows(&self) -> Vec<Value> {
        self.detail
            .as_ref()
            .and_then(|v| v.get("order"))
            .cloned()
            .into_iter()
            .collect()
    }

    fn fee_rows(&self) -> Vec<Value> {
        let Some(order) = self.detail.as_ref().and_then(|v| v.get("order")) else {
            return Vec::new();
        };
        let fields = [
            ("platform_commission_cent", "平台佣金"),
            ("estimated_postage_cent", "预估运费"),
            ("postage_amount_cent", "实际运费"),
            ("goods_amount_cent", "商品金额"),
            ("discount_amount_cent", "优惠金额"),
            ("total_amount_cent", "订单合计"),
        ];
        fields
            .iter()
            .filter_map(|(key, name)| {
                order
                    .get(*key)
                    .filter(|amount| amount.is_i64())
                    .map(|amount| serde_json::json!({"id": key, "fee_name": name, "amount_cent": amount}))
            })
            .collect()
    }
}

fn read_only_state() -> GridState {
    let mut state = GridState::default();
    state.read_only = true;
    state.theme = Some(crate::erp_theme::grid());
    state.metrics = erp_ui_egui::GridMetrics::legacy();
    state.sorting_enabled = false;
    state.filtering_enabled = false;
    state
}

fn array_at(value: Option<&Value>, key: &str) -> Vec<Value> {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn text(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

fn amount(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_i64))
        .map(super::cents_text)
        .unwrap_or_default()
}

fn show_grid(
    ui: &mut egui::Ui,
    id: &str,
    columns: &mut Vec<Column>,
    state: &mut GridState,
    rows: &[Value],
    cell: fn(&Value, ColumnId) -> String,
) {
    if rows.is_empty() {
        ui.label("暂无数据");
        return;
    }
    DataGrid {
        id: ui.id().with(id),
        columns,
        rows: rows.len(),
        state,
        row_id: Some(&|index| {
            let identity = text(&rows[index], &["audit_id", "shipment_id", "id"]);
            if identity.is_empty() {
                RowId(index as u128 + 1)
            } else {
                super::stable_id(&identity)
            }
        }),
    }
    .show(ui, |index, column| cell(&rows[index], column));
}

fn platform_cell(row: &Value, column: ColumnId) -> String {
    match column {
        SOURCE => text(row, &["order_source", "sales_channel", "platform_source"]),
        TYPE => text(row, &["order_type", "order_type_name"]),
        NUMBER => text(row, &["platform_order_id", "order_no", "id"]),
        STATUS => text(
            row,
            &[
                "platform_status",
                "oms_status",
                "order_status",
                "review_status",
            ],
        ),
        AMOUNT => amount(row, &["total_amount_cent", "receivable_amount_cent"]),
        TIME => text(row, &["order_created_at", "created_at"]),
        _ => String::new(),
    }
}

fn shipment_cell(row: &Value, column: ColumnId) -> String {
    match column {
        SHIPMENT_NO => text(row, &["shipment_no", "shipment_id"]),
        SHIPMENT_STATUS => text(row, &["status", "shipment_status"]),
        SHIPPED_AT => text(row, &["shipped_at"]),
        WAREHOUSE => text(row, &["warehouse_id"]),
        CARRIER => text(row, &["carrier_code", "logistics_company"]),
        TRACKING => text(row, &["tracking_no"]),
        _ => String::new(),
    }
}

fn fee_cell(row: &Value, column: ColumnId) -> String {
    match column {
        FEE_NAME => text(row, &["fee_name"]),
        FEE_AMOUNT => amount(row, &["amount_cent"]),
        REMARK => text(row, &["remark"]),
        _ => String::new(),
    }
}

fn log_cell(row: &Value, column: ColumnId) -> String {
    match column {
        LOG_TIME => text(row, &["created_at"]),
        ACTION => text(row, &["action"]),
        OPERATOR => text(row, &["operator", "user_id"]),
        DETAIL => {
            let before = row.get("before_json").filter(|v| !v.is_null());
            let after = row.get("after_json").filter(|v| !v.is_null());
            match (before, after) {
                (Some(before), Some(after)) => format!("{before} → {after}"),
                (_, Some(value)) | (Some(value), _) => value.to_string(),
                _ => text(row, &["detail", "message"]),
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn api_detail_uses_minor_currency_and_actual_audit_payload() {
        let mut modules = DetailModules {
            order_id: Some("order-1".into()),
            ..Default::default()
        };
        modules.accept_detail(br#"{"order":{"id":"order-1","platform_order_id":"QA-001","total_amount_cent":1250,"postage_amount_cent":null},"audit_logs":[{"audit_id":"log-1","user_id":"operator-1","after_json":{"warehouse":"A"}}]}"#);
        assert_eq!(platform_cell(&modules.platform_rows[0], NUMBER), "QA-001");
        assert_eq!(platform_cell(&modules.platform_rows[0], AMOUNT), "12.50");
        assert_eq!(modules.fee_rows.len(), 1);
        assert_eq!(fee_cell(&modules.fee_rows[0], FEE_AMOUNT), "12.50");
        assert!(log_cell(&modules.log_rows[0], DETAIL).contains("warehouse"));
        assert_eq!(log_cell(&modules.log_rows[0], OPERATOR), "operator-1");
        modules.clear();
        assert!(modules.platform_rows.is_empty());
        modules.accept_detail(br#"{"order":{"id":"other"}}"#);
        assert!(modules.platform_rows.is_empty());
        assert!(modules.error.is_some());
    }
}
