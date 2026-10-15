//! Shared material commands. No token or client-authored stock effect is persisted.
use crate::remote_actions::ActionScope;
use erp_grid_core::decimal_scaled;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) const MAX_BYTES: usize = 1024 * 1024;
#[derive(Clone, Deserialize)]
pub(super) struct BatchOption {
    pub batch_id: Uuid,
    pub batch_no: Option<String>,
    pub expire_at: Option<String>,
    pub on_hand_micros: Option<String>,
    pub reserved_micros: Option<String>,
    pub available_micros: Option<String>,
    pub state: String,
}
#[derive(Deserialize)]
pub(super) struct BatchPage {
    pub work_order_id: Uuid,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub batch_enabled: bool,
    pub observed_at: String,
    pub warehouse_available_micros: Option<String>,
    pub needs_reconciliation: bool,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
    pub data: Vec<BatchOption>,
}
impl BatchPage {
    pub fn validate(&self, order: Uuid, line: &Line, offset: u64) -> Result<(), String> {
        if self.work_order_id != order
            || self.warehouse_id != line.warehouse_id
            || self.sku_id != line.sku_id
            || self.offset != offset
            || self.limit != 500
            || self.data.len() > 500
            || self.total > 1000
            || (!self.data.is_empty()
                && self.total < self.offset.saturating_add(self.data.len() as u64))
            || chrono::DateTime::parse_from_rfc3339(&self.observed_at).is_err()
        {
            return Err("批次查询归属、页码或预算不一致".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        let observed = chrono::DateTime::parse_from_rfc3339(&self.observed_at)
            .map_err(|_| "批次观察时间无效")?;
        for batch in &self.data {
            if batch.batch_id.is_nil()
                || !ids.insert(batch.batch_id)
                || !matches!(
                    batch.state.as_str(),
                    "READY" | "EMPTY" | "EXPIRED" | "RECONCILIATION_REQUIRED"
                )
                || batch.batch_no.as_ref().is_some_and(|s| s.len() > 256)
                || batch
                    .expire_at
                    .as_ref()
                    .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_err())
            {
                return Err("批次事实身份或状态无效".into());
            }
            if batch.state == "READY"
                && (self.needs_reconciliation
                    || !batch
                        .available_micros
                        .as_deref()
                        .is_some_and(|s| s.parse::<i64>().is_ok_and(|n| n > 0))
                    || batch.batch_no.is_none())
            {
                return Err("批次没有已知精确可用量".into());
            }
            if batch.state == "READY" {
                let parse = |value: &Option<String>| {
                    value
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .filter(|n| *n >= 0)
                };
                let (Some(hand), Some(reserved), Some(available), Some(whole)) = (
                    parse(&batch.on_hand_micros),
                    parse(&batch.reserved_micros),
                    parse(&batch.available_micros),
                    parse(&self.warehouse_available_micros),
                ) else {
                    return Err("批次精确量缺失".into());
                };
                if reserved > hand
                    || available > hand - reserved
                    || available > whole
                    || batch.expire_at.as_ref().is_some_and(|s| {
                        chrono::DateTime::parse_from_rfc3339(s).is_ok_and(|e| e <= observed)
                    })
                {
                    return Err("批次数量或效期与观察事实不一致".into());
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct WorkOrder {
    pub work_order_id: Uuid,
    pub work_order_no: String,
    pub product_code: String,
    pub status: String,
    pub bom_id: Option<Uuid>,
    pub order_qty_micros: String,
    pub issued_micros: String,
    pub consumed_micros: String,
}
impl WorkOrder {
    pub fn source(&self) -> Value {
        json!({"work_order_id":self.work_order_id,"work_order_no":self.work_order_no,
            "product_code":self.product_code,"bom_id":self.bom_id,"order_qty_micros":self.order_qty_micros})
    }
}
#[derive(Clone, Deserialize)]
pub(super) struct Component {
    pub component_code: String,
    pub sku_id: Option<Uuid>,
    pub sku_name: Option<String>,
    pub unit_code: String,
    pub direct_component: bool,
}
#[derive(Clone, Deserialize)]
pub(super) struct Warehouse {
    pub warehouse_id: Uuid,
    pub code: Option<String>,
    pub name: String,
}
#[derive(Deserialize)]
pub(super) struct Context {
    pub work_order: WorkOrder,
    pub materials: Vec<Component>,
    pub warehouses: Vec<Warehouse>,
    pub needs_reconciliation: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Line {
    pub line_id: Uuid,
    pub kind: String,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub original_issue_id: Option<Uuid>,
    pub unit_code: String,
    pub qty: String,
    pub reason: Option<String>,
}
impl Line {
    pub fn new(material: &Component, warehouse: &Warehouse) -> Result<Self, String> {
        Ok(Self {
            line_id: Uuid::new_v4(),
            kind: "ISSUE".into(),
            warehouse_id: warehouse.warehouse_id,
            sku_id: material.sku_id.ok_or("材料尚未关联真实货品")?,
            batch_id: None,
            original_issue_id: None,
            unit_code: material.unit_code.clone(),
            qty: "1".into(),
            reason: None,
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.line_id.is_nil()
            || self.warehouse_id.is_nil()
            || self.sku_id.is_nil()
            || !["ISSUE", "SUPPLEMENT", "RETURN", "CONSUME"].contains(&self.kind.as_str())
            || self.unit_code.trim().is_empty()
            || self.unit_code.len() > 32
            || self.batch_id.is_some_and(|id| id.is_nil())
            || self.original_issue_id.is_some_and(|id| id.is_nil())
            || self
                .reason
                .as_ref()
                .is_some_and(|r| r.len() > 4096 || r.chars().any(char::is_control))
        {
            return Err("材料身份、单位或原因无效".into());
        }
        quantity(&self.qty)?;
        if matches!(self.kind.as_str(), "RETURN" | "CONSUME") != self.original_issue_id.is_some() {
            return Err("退料/耗料须选择原领料，领料/补料不能绑定原领料".into());
        }
        if self.kind == "SUPPLEMENT" && self.reason.as_deref().is_none_or(|r| r.trim().is_empty()) {
            return Err("补料须填写原因".into());
        }
        Ok(())
    }
    pub fn saved(&self) -> Result<Value, String> {
        let mut value = serde_json::to_value(self).map_err(|_| "明细编码失败")?;
        value.as_object_mut().ok_or("明细格式错误")?.remove("qty");
        value["qty_micros"] = json!(quantity(&self.qty)?.to_string());
        Ok(value)
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Draft {
    pub document_id: Option<Uuid>,
    pub order: Option<WorkOrder>,
    pub lines: Vec<Line>,
    pub saved: Option<Value>,
    pub acknowledged_source: Option<Value>,
    pub changed: bool,
    #[serde(default)]
    pub batch_labels: std::collections::BTreeMap<Uuid, String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) enum Command {
    Save,
    Adopt,
    Confirm,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prepared {
    pub command: Command,
    pub request: String,
    pub body: Value,
    pub context: Value,
    pub review: Option<Value>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rejection {
    pub original: Prepared,
    pub message: String,
}
impl Draft {
    pub fn adopt_latest(&mut self, snapshot: &Value) -> Result<(), String> {
        let order = self.order.clone().ok_or("真实工单未就绪")?;
        let latest = Self::from_snapshot(snapshot, order)?;
        if latest.document_id != self.document_id || latest.version() < self.version() {
            return Err("最新结果未绑定同一材料单或版本倒退".into());
        }
        if latest.posted() {
            return Err("服务器已确认该单，本地输入继续保留，请先核对实际效果".into());
        }
        self.saved = latest.saved;
        self.changed = true;
        Ok(())
    }
    pub fn version(&self) -> i64 {
        self.saved
            .as_ref()
            .and_then(|s| s["version_no"].as_i64())
            .unwrap_or(0)
    }
    pub fn posted(&self) -> bool {
        self.saved.as_ref().is_some_and(|s| s["posted"] == true)
    }
    pub fn prepare(
        &self,
        command: Command,
        scope: &ActionScope,
        review: Option<Value>,
    ) -> Result<Prepared, String> {
        let request = Uuid::new_v4().to_string();
        let document = self.document_id.ok_or("先选择工单并新建材料单")?;
        let order = self.order.as_ref().ok_or("真实工单未就绪")?;
        if self.posted() {
            return Err("已确认材料单不能再次修改或确认".into());
        }
        let context = json!({"tenant_id":scope.tenant().to_uuid(),"actor_user_id":scope.actor().to_uuid(),
            "work_order_id":order.work_order_id,"document_id":document});
        let body = match command {
            Command::Save => self.save_body(&request)?,
            Command::Confirm => {
                if self.changed
                    || self.version() < 1
                    || self.acknowledged_source.as_ref()
                        != self.saved.as_ref().map(|s| &s["source"])
                {
                    return Err("先保存全部修改并明确复核来源后确认".into());
                }
                let lines = self
                    .lines
                    .iter()
                    .map(Line::saved)
                    .collect::<Result<Vec<_>, _>>()?;
                if Some(&json!(lines)) != self.saved.as_ref().map(|s| &s["lines"]) {
                    return Err("实际明细与已保存版本不一致，先保存修改".into());
                }
                json!({"request_id":request,"expected_version":self.version()})
            }
            Command::Adopt => {
                let r = review.as_ref().ok_or("先读取来源复核结果")?;
                self.validate_review(r)?;
                json!({"request_id":request,"expected_version":self.version(),"review_token":r["review_token"]})
            }
        };
        Ok(Prepared {
            command,
            request,
            body,
            context,
            review,
        })
    }
    pub fn validate_review(&self, r: &Value) -> Result<(), String> {
        let order = self.order.as_ref().ok_or("工单未就绪")?;
        if self.version() < 1
            || r["document_id"] != json!(self.document_id)
            || r["work_order_id"] != json!(order.work_order_id)
            || r["version_no"] != self.version()
            || Some(&r["saved_source"]) != self.saved.as_ref().map(|s| &s["source"])
            || !r["current_source"].is_object()
            || !r["review_token"]
                .as_str()
                .is_some_and(|t| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("来源复核未绑定当前材料单版本".into());
        }
        Ok(())
    }
    pub fn save_body(&self, request: &str) -> Result<Value, String> {
        if self.document_id.is_none_or(|id| id.is_nil())
            || self.order.as_ref().is_none_or(|o| o.work_order_id.is_nil())
            || self.lines.is_empty()
            || self.lines.len() > 100
        {
            return Err("工单、材料单或1至100行明细未就绪".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut lines = Vec::with_capacity(self.lines.len());
        for line in &self.lines {
            line.validate()?;
            if !ids.insert(line.line_id) {
                return Err("明细身份重复".into());
            }
            let mut value = serde_json::to_value(line).map_err(|_| "明细编码失败")?;
            value["qty"] = json!(display(quantity(&line.qty)?));
            value["unit_code"] = json!(line.unit_code.trim());
            value["reason"] = json!(
                line.reason
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            );
            lines.push(value);
        }
        Ok(
            json!({"request_id":request,"document_id":self.document_id,"expected_version":self.version(),"lines":lines}),
        )
    }
    pub fn from_snapshot(snapshot: &Value, order: WorkOrder) -> Result<Self, String> {
        validate_snapshot(snapshot, order.work_order_id)?;
        Ok(Self {
            document_id: Some(id(snapshot, "document_id")?),
            order: Some(order),
            lines: decode_lines(&snapshot["lines"])?,
            saved: Some(snapshot.clone()),
            acknowledged_source: Some(snapshot["source"].clone()),
            changed: false,
            batch_labels: Default::default(),
        })
    }
}
pub(super) fn quantity(text: &str) -> Result<i64, String> {
    if text.len() > 64 {
        return Err("数量长度超限".into());
    }
    decimal_scaled(text, 6)
        .map_err(str::to_owned)?
        .and_then(|n| i64::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| "数量须为最多六位小数的正数".into())
}
pub(super) fn display(micros: i64) -> String {
    format!("{}.{:06}", micros / 1000000, micros % 1000000)
}
fn id(value: &Value, key: &str) -> Result<Uuid, String> {
    value[key]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .filter(|id| !id.is_nil())
        .ok_or_else(|| format!("实际结果身份缺失：{key}"))
}
fn decode_lines(value: &Value) -> Result<Vec<Line>, String> {
    let array = value
        .as_array()
        .filter(|v| !v.is_empty() && v.len() <= 100)
        .ok_or("实际明细数量错误")?;
    let mut lines = Vec::with_capacity(array.len());
    let mut ids = std::collections::BTreeSet::new();
    for item in array {
        let mut value = item.clone();
        let micros = item["qty_micros"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|n| *n > 0)
            .ok_or("原明细数量格式错误")?;
        value
            .as_object_mut()
            .ok_or("明细格式错误")?
            .remove("qty_micros");
        value["qty"] = json!(display(micros));
        let line: Line = serde_json::from_value(value).map_err(|_| "原明细字段错误")?;
        line.validate()?;
        if !ids.insert(line.line_id) {
            return Err("原明细身份重复".into());
        }
        lines.push(line);
    }
    Ok(lines)
}
fn validate_snapshot(snapshot: &Value, order: Uuid) -> Result<(), String> {
    if id(snapshot, "work_order_id")? != order
        || snapshot["version_no"]
            .as_i64()
            .is_none_or(|v| !(1..=i64::from(i32::MAX)).contains(&v))
        || !snapshot["source"].is_object()
        || snapshot["source"]["work_order_id"] != json!(order)
    {
        return Err("原材料结果未绑定真实工单/版本/来源".into());
    }
    id(snapshot, "document_id")?;
    id(snapshot, "actor_user_id")?;
    if !snapshot["request_id"]
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control))
    {
        return Err("原结果请求身份缺失".into());
    }
    if !((snapshot["schema"] == 1 && snapshot["status"] == "DRAFT" && snapshot["posted"] == false)
        || (snapshot["schema"] == 2
            && snapshot["status"] == "POSTED"
            && snapshot["posted"] == true))
    {
        return Err("原材料结果状态不明确".into());
    }
    let time = if snapshot["schema"] == 1 {
        id(snapshot, "revision_id")?;
        "saved_at"
    } else {
        "posted_at"
    };
    if !snapshot[time]
        .as_str()
        .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
    {
        return Err("原材料结果时间缺失".into());
    }
    let lines = decode_lines(&snapshot["lines"])?;
    if snapshot["schema"] == 2 {
        validate_effects(snapshot, &lines)?;
    }
    Ok(())
}
fn validate_effects(snapshot: &Value, lines: &[Line]) -> Result<(), String> {
    let effects = snapshot["effects"]
        .as_array()
        .filter(|e| e.len() == lines.len())
        .ok_or("实际整单效果不完整")?;
    let mut moves = std::collections::BTreeSet::new();
    for (line, effect) in lines.iter().zip(effects) {
        let saved = line.saved()?;
        for key in [
            "line_id",
            "kind",
            "warehouse_id",
            "sku_id",
            "batch_id",
            "original_issue_id",
            "unit_code",
            "qty_micros",
        ] {
            if effect[key] != saved[key] {
                return Err("实际效果与原材料明细不一致".into());
            }
        }
        if !moves.insert(id(effect, "move_id")?) {
            return Err("实际材料效果重复".into());
        }
        if effect["value"]["state"] != "VALUED"
            || !effect["value"]["amount_cent"]
                .as_str()
                .is_some_and(|s| s.parse::<i64>().is_ok_and(|n| n >= 0))
            || !effect["value"]["original"].is_object()
        {
            return Err("原材料成本事实缺失".into());
        }
        for (group, keys) in [
            (
                "stock",
                [
                    "on_hand_micros",
                    "reserved_micros",
                    "frozen_micros",
                    "in_transit_micros",
                ],
            ),
            (
                "material",
                [
                    "issued_micros",
                    "returned_micros",
                    "consumed_micros",
                    "shop_micros",
                ],
            ),
        ] {
            for key in keys {
                if !effect[group][key]
                    .as_str()
                    .is_some_and(|s| s.parse::<i64>().is_ok_and(|n| n >= 0))
                {
                    return Err("实际库存或材料数量事实缺失".into());
                }
            }
        }
    }
    for key in ["operation_log_id", "audit_id", "event_id"] {
        id(snapshot, key)?;
    }
    Ok(())
}
pub(super) fn rejected(prepared: &Prepared, response: &Value) -> Result<Option<Rejection>, String> {
    if response["code"] != "material_document_rejected_before_commit" {
        return Ok(None);
    }
    if response["committed"] != false
        || response["request_id"] != prepared.request
        || response["fields"] != prepared.body
        || response["context"] != prepared.context
    {
        return Err("拒绝未绑定原请求、字段及账号/工单/材料单，请继续核对".into());
    }
    let message = response["message"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 4096)
        .ok_or("拒绝原因格式错误")?;
    Ok(Some(Rejection {
        original: prepared.clone(),
        message: message.into(),
    }))
}
pub(super) fn matched(
    draft: &Draft,
    prepared: &Prepared,
    response: &Value,
) -> Result<Draft, String> {
    let snapshot = &response["snapshot"];
    let order = draft.order.clone().ok_or("原工单丢失")?;
    validate_snapshot(snapshot, order.work_order_id)?;
    if snapshot["document_id"] != prepared.context["document_id"]
        || snapshot["actor_user_id"] != prepared.context["actor_user_id"]
        || snapshot["request_id"] != prepared.request
    {
        return Err("实际结果未绑定原请求与提交人".into());
    }
    let mut result = draft.clone();
    let expected = prepared.body["expected_version"]
        .as_i64()
        .ok_or("原版本丢失")?;
    match prepared.command {
        Command::Save => {
            let actual = decode_lines(&snapshot["lines"])?;
            let mut candidate = draft.clone();
            candidate.lines = actual;
            if candidate.save_body(&prepared.request)?["lines"] != prepared.body["lines"]
                || snapshot["version_no"] != expected + 1
                || snapshot["status"] != "DRAFT"
                || draft
                    .saved
                    .as_ref()
                    .is_some_and(|s| s["source"] != snapshot["source"])
            {
                return Err("原保存版本或明细不一致".into());
            }
            result.changed = false;
            result.lines = candidate.lines;
        }
        Command::Adopt => {
            let review = prepared.review.as_ref().ok_or("原来源复核丢失")?;
            if snapshot["version_no"] != expected + 1
                || snapshot["status"] != "DRAFT"
                || snapshot["source"] != review["current_source"]
                || snapshot["source_adoption"]["previous_source"] != review["saved_source"]
                || snapshot["source_adoption"]["review_token"] != prepared.body["review_token"]
                || Some(&snapshot["lines"]) != draft.saved.as_ref().map(|s| &s["lines"])
            {
                return Err("来源采用结果与原复核或已保存明细不一致".into());
            }
            result.acknowledged_source = Some(snapshot["source"].clone());
            // The server adopted saved lines; local unsaved inputs deliberately survive.
        }
        Command::Confirm => {
            if snapshot["version_no"] != expected
                || snapshot["status"] != "POSTED"
                || Some(&snapshot["lines"]) != draft.saved.as_ref().map(|s| &s["lines"])
                || Some(&snapshot["source"]) != draft.saved.as_ref().map(|s| &s["source"])
            {
                return Err("确认结果与原保存版本/来源不一致".into());
            }
            let effects = snapshot["effects"]
                .as_array()
                .filter(|e| e.len() == draft.lines.len())
                .ok_or("实际整单效果不完整")?;
            let mut moves = std::collections::BTreeSet::new();
            for (line, effect) in draft.lines.iter().zip(effects) {
                let saved = line.saved()?;
                for key in [
                    "line_id",
                    "kind",
                    "warehouse_id",
                    "sku_id",
                    "batch_id",
                    "original_issue_id",
                    "unit_code",
                    "qty_micros",
                ] {
                    if effect[key] != saved[key] {
                        return Err("实际材料效果未绑定原明细".into());
                    }
                }
                if !moves.insert(id(effect, "move_id")?) {
                    return Err("实际材料效果重复".into());
                }
                if !effect["stock"].is_object() || !effect["value"].is_object() {
                    return Err("实际库存/成本事实缺失".into());
                }
            }
            for key in ["operation_log_id", "audit_id", "event_id"] {
                id(snapshot, key)?;
            }
            result.changed = false;
        }
    }
    result.saved = Some(snapshot.clone());
    Ok(result)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Checkpoint {
    pub schema: u16,
    pub scope: ActionScope,
    pub draft: Draft,
    pub pending: Option<Prepared>,
    pub rejected: Option<Rejection>,
}
impl Checkpoint {
    pub fn decode(bytes: &[u8], scope: &ActionScope) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_BYTES {
            return Err("材料恢复记录超过1MiB".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "材料恢复记录格式错误")?;
        if value.schema != 1 || &value.scope != scope || value.draft.lines.len() > 100 {
            return Err("材料恢复归属/版本错误".into());
        }
        if let Some(p) = &value.pending {
            let expected = value
                .draft
                .prepare(p.command.clone(), scope, p.review.clone())?;
            let mut body = expected.body;
            body["request_id"] = json!(p.request);
            if !Uuid::parse_str(&p.request).is_ok_and(|id| !id.is_nil())
                || p.body != body
                || p.context != expected.context
            {
                return Err("原材料请求与冻结草稿不一致".into());
            }
        }
        if let Some(saved) = &value.draft.saved {
            validate_snapshot(
                saved,
                value
                    .draft
                    .order
                    .as_ref()
                    .ok_or("恢复工单缺失")?
                    .work_order_id,
            )?;
        }
        Ok(value)
    }
}
