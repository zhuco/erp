use crate::remote_actions::ActionScope;
use erp_grid_core::{ColumnId, RowId, decimal_scaled, money_minor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub(super) const MAX_BYTES: usize = 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Line {
    pub number: i32,
    pub sku: String,
    pub qty: String,
    pub price: String,
    pub discount: String,
    pub tax: String,
    pub date: String,
    pub address: String,
    pub source: String,
    pub warehouse: Option<String>,
}
impl Line {
    pub fn new(number: i32) -> Self {
        Self {
            number,
            sku: String::new(),
            qty: "1".into(),
            price: "0.00".into(),
            discount: "0.00".into(),
            tax: "0.00".into(),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            address: String::new(),
            source: "RESALE".into(),
            warehouse: None,
        }
    }
    pub fn text(&self, column: ColumnId) -> String {
        match column.0 {
            1 => self.sku.clone(),
            2 => self.qty.clone(),
            3 => self.price.clone(),
            4 => self.discount.clone(),
            5 => self.tax.clone(),
            6 => self.date.clone(),
            7 => self.address.clone(),
            8 => if self.source == "MAKE" {
                "自产"
            } else {
                "转售"
            }
            .into(),
            _ => String::new(),
        }
    }
    pub fn set(&mut self, column: ColumnId, value: &str) -> Result<(), String> {
        if value.len() > 512 {
            return Err("字段长度超限".into());
        }
        match column.0 {
            1 => self.sku = value.into(),
            2 => {
                quantity(value)?;
                self.qty = value.into();
            }
            3 => {
                cents(value)?;
                self.price = value.into();
            }
            4 => {
                cents(value)?;
                self.discount = value.into();
            }
            5 => {
                cents(value)?;
                self.tax = value.into();
            }
            6 => {
                chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| "交期格式错误")?;
                self.date = value.into();
            }
            7 => self.address = value.into(),
            8 => {
                self.source = match value {
                    "自产" => "MAKE",
                    "转售" => "RESALE",
                    _ => return Err("来源类型错误".into()),
                }
                .into()
            }
            _ => return Err("字段不可编辑".into()),
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Draft {
    pub order: RowId,
    pub number: String,
    pub customer: String,
    pub currency: String,
    pub version: Option<i32>,
    pub status: Option<String>,
    pub lines: Vec<Line>,
    pub changed: bool,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            order: RowId(uuid::Uuid::new_v4().as_u128()),
            number: String::new(),
            customer: String::new(),
            currency: "CNY".into(),
            version: None,
            status: None,
            lines: vec![Line::new(1)],
            changed: false,
        }
    }
}
impl Draft {
    pub fn row_id(&self, line: &Line) -> RowId {
        RowId(self.order.0 ^ u128::from(line.number.unsigned_abs()))
    }
    pub fn total(&self) -> Result<i64, String> {
        if self.lines.is_empty() || self.lines.len() > 1000 {
            return Err("明细数量错误".into());
        }
        let mut total = 0i64;
        for line in &self.lines {
            let value = i128::from(quantity(&line.qty)?) * i128::from(cents(&line.price)?)
                / 1_000_000
                - i128::from(cents(&line.discount)?)
                + i128::from(cents(&line.tax)?);
            let value = i64::try_from(value).map_err(|_| "明细金额超限")?;
            if value < 0 {
                return Err("折扣超过明细金额".into());
            }
            total = total.checked_add(value).ok_or("合计金额超限")?;
        }
        Ok(total)
    }
    pub fn payload(&self, request: &str) -> Result<Value, String> {
        if self.number.trim().is_empty()
            || self.number.len() > 128
            || self.customer.trim().is_empty()
            || self.customer.len() > 128
            || self.lines.is_empty()
            || self.lines.len() > 1000
        {
            return Err("单据号、客户或明细未填写完整".into());
        }
        if !["CNY", "USD", "EUR", "HKD"].contains(&self.currency.as_str()) {
            return Err("币种不受支持".into());
        }
        self.total()?;
        let mut numbers = std::collections::BTreeSet::new();
        let mut items = Vec::with_capacity(self.lines.len());
        for line in &self.lines {
            if line.number <= 0
                || !numbers.insert(line.number)
                || line.sku.trim().is_empty()
                || line.sku.len() > 128
                || line.address.trim().is_empty()
                || line.address.len() > 512
            {
                return Err("明细身份、货品或地址无效".into());
            }
            let qty = display(quantity(&line.qty)?, 6);
            chrono::NaiveDate::parse_from_str(&line.date, "%Y-%m-%d")
                .map_err(|_| "交期格式错误")?;
            let warehouse = line
                .warehouse
                .as_deref()
                .map(|id| {
                    uuid::Uuid::parse_str(id)
                        .map(|id| id.to_string())
                        .map_err(|_| "仓库身份错误")
                })
                .transpose()?;
            items.push(json!({"line_no":line.number,"sku_code":line.sku.trim(),"qty":qty,"price_cent":cents(&line.price)?,"discount_cent":cents(&line.discount)?,"tax_cent":cents(&line.tax)?,"due_date":line.date,"address":line.address.trim(),"source_kind":line.source,"warehouse_id":warehouse}));
        }
        Ok(
            json!({"request_id":request,"so_id":self.order.to_uuid(),"document_no":self.number.trim(),"customer_code":self.customer.trim(),"currency":self.currency,"action":if self.version.is_some(){"REVISE"}else{"SUBMIT"},"expected_version":self.version,"items":items}),
        )
    }
    pub fn decode(document: &Value) -> Result<Self, String> {
        let h = &document["header"];
        let text = |key: &str| {
            h[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("单头字段缺失: {key}"))
        };
        let order = RowId::from_uuid(&text("so_id")?)
            .filter(|id| id.0 != 0)
            .ok_or("单据身份无效")?;
        let version = h["version_no"]
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or("版本无效")?;
        let items = document["items"].as_array().ok_or("明细缺失")?;
        if items.is_empty() || items.len() > 1000 {
            return Err("明细数量超限".into());
        }
        let mut lines = Vec::with_capacity(items.len());
        for item in items {
            let s = |key: &str| {
                item[key]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("明细字段缺失: {key}"))
            };
            let money = |key: &str| {
                item[key]
                    .as_i64()
                    .map(|n| display(n, 2))
                    .ok_or_else(|| format!("金额字段缺失: {key}"))
            };
            let micros = s("qty_micros")?
                .parse::<i64>()
                .map_err(|_| "数量格式错误")?;
            let number = item["line_no"]
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .filter(|n| *n > 0)
                .ok_or("行身份错误")?;
            let source = s("source_kind")?;
            if !["RESALE", "MAKE"].contains(&source.as_str()) {
                return Err("未知来源类型".into());
            }
            lines.push(Line {
                number,
                sku: s("item_code")?,
                qty: display(micros, 6),
                price: money("price_cent")?,
                discount: money("discount_cent")?,
                tax: money("tax_cent")?,
                date: s("due_date")?,
                address: s("address")?,
                source,
                warehouse: item["warehouse_id"].as_str().map(str::to_owned),
            });
        }
        let status = text("status")?;
        if !["SUBMITTED", "FULFILLING", "FULFILLED", "CANCELLED"].contains(&status.as_str()) {
            return Err("未知单据状态".into());
        }
        let draft = Self {
            order,
            number: text("document_no")?,
            customer: text("customer_code")?,
            currency: text("currency")?,
            version: Some(version),
            status: Some(status),
            lines,
            changed: false,
        };
        draft.payload("validate")?;
        if h["total_cent"].as_i64() != Some(draft.total()?) {
            return Err("服务端合计与明细不一致".into());
        }
        Ok(draft)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prepared {
    pub request: String,
    pub body: Value,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rejection {
    pub original: Prepared,
    pub message: String,
}
pub(super) fn rejected(prepared: &Prepared, response: &Value) -> Result<Option<Rejection>, String> {
    if response["code"] != "trade_document_rejected_before_commit" {
        return Ok(None);
    }
    if response["committed"] != false
        || response["request_id"] != prepared.request
        || response["fields"] != prepared.body
    {
        return Err("拒绝结果未绑定原请求，需继续核对".into());
    }
    let message = response["message"]
        .as_str()
        .filter(|message| !message.is_empty() && message.len() <= 4096)
        .ok_or("拒绝结果原因无效")?;
    Ok(Some(Rejection {
        original: prepared.clone(),
        message: message.into(),
    }))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Checkpoint {
    pub schema: u16,
    pub scope: ActionScope,
    pub draft: Draft,
    pub pending: Option<Prepared>,
    #[serde(default)]
    pub rejected: Option<Rejection>,
}
impl Checkpoint {
    pub fn decode(bytes: &[u8], scope: &ActionScope) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_BYTES {
            return Err("销售草稿大小错误".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "销售草稿格式错误")?;
        if value.schema != 1
            || &value.scope != scope
            || value.draft.lines.len() > 1000
            || value.draft.order.0 == 0
        {
            return Err("销售草稿归属或版本错误".into());
        }
        if let Some(pending) = &value.pending
            && (!uuid::Uuid::parse_str(&pending.request).is_ok_and(|id| !id.is_nil())
                || pending.body != value.draft.payload(&pending.request)?)
        {
            return Err("原销售请求与草稿不一致".into());
        }
        if value.rejected.as_ref().is_some_and(|rejection| {
            rejection.message.len() > 4096
                || !uuid::Uuid::parse_str(&rejection.original.request).is_ok_and(|id| !id.is_nil())
        }) {
            return Err("历史拒绝记录身份无效".into());
        }
        Ok(value)
    }
}
pub(super) fn matches(prepared: &Prepared, document: &Value) -> Result<Draft, String> {
    let draft = Draft::decode(document)?;
    let body = draft.payload(&prepared.request)?;
    for key in ["so_id", "document_no", "customer_code", "currency", "items"] {
        if body[key] != prepared.body[key] {
            return Err("原销售结果与提交字段不一致，请核对".into());
        }
    }
    if document["header"]["request_id"] != prepared.request {
        return Err("原销售结果身份不一致".into());
    }
    let expected = prepared.body["expected_version"]
        .as_i64()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("版本超限")?;
    if draft.version.map(i64::from) != Some(expected) {
        return Err("原销售结果版本不一致".into());
    }
    Ok(draft)
}
pub(super) fn quantity(text: &str) -> Result<i64, String> {
    decimal_scaled(text, 6)
        .map_err(str::to_owned)?
        .and_then(|n| i64::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| "数量必须为最多六位小数的正数".into())
}
fn cents(text: &str) -> Result<i64, String> {
    money_minor(text)
        .map_err(str::to_owned)?
        .filter(|n| *n >= 0)
        .ok_or_else(|| "金额必须为非负数且最多两位小数".into())
}
fn display(value: i64, scale: u8) -> String {
    let unit = 10i128.pow(u32::from(scale));
    let value = i128::from(value);
    let sign = if value < 0 { "-" } else { "" };
    let value = value.abs();
    let mut result = format!(
        "{sign}{}.{:0width$}",
        value / unit,
        value % unit,
        width = usize::from(scale)
    );
    if scale == 6 {
        while result.ends_with('0') {
            result.pop();
        }
        if result.ends_with('.') {
            result.pop();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trade_sales_rejection_requires_exact_original_fields_and_no_commit() -> Result<(), String> {
        let mut draft = Draft {
            number: "SO-1".into(),
            customer: "C-1".into(),
            ..Default::default()
        };
        draft.lines[0].sku = "S-1".into();
        draft.lines[0].address = "Dock".into();
        let request = uuid::Uuid::new_v4().to_string();
        let prepared = Prepared {
            body: draft.payload(&request)?,
            request,
        };
        let mut reply = json!({"code":"trade_document_rejected_before_commit","committed":false,"request_id":prepared.request,"fields":prepared.body,"message":"customer unavailable"});
        assert!(rejected(&prepared, &reply)?.is_some());
        reply["committed"] = json!(true);
        assert!(rejected(&prepared, &reply).is_err());
        reply["committed"] = json!(false);
        reply["fields"]["items"][0]["qty"] = json!("2");
        assert!(rejected(&prepared, &reply).is_err());
        assert!(rejected(&prepared, &json!({"status":"unknown"}))?.is_none());
        Ok(())
    }
    #[test]
    fn trade_sales_payload_preserves_fraction_and_rejects_subcent_money() -> Result<(), String> {
        let mut draft = Draft {
            number: "SO-1".into(),
            customer: "C-1".into(),
            ..Default::default()
        };
        draft.lines[0].sku = "S-1".into();
        draft.lines[0].address = "Dock".into();
        draft.lines[0].qty = "1.25".into();
        let body = draft.payload("original")?;
        assert_eq!(body["items"][0]["qty"], "1.25");
        draft.lines[0].price = "0.001".into();
        assert!(draft.payload("original").is_err());
        Ok(())
    }
    #[test]
    fn trade_sales_original_result_requires_matching_identity_and_version() -> Result<(), String> {
        let mut draft = Draft {
            number: "SO-1".into(),
            customer: "C-1".into(),
            ..Default::default()
        };
        draft.lines[0].sku = "S-1".into();
        draft.lines[0].address = "Dock".into();
        draft.lines[0].qty = "1.2500".into();
        let request = uuid::Uuid::new_v4().to_string();
        let body = draft.payload(&request)?;
        assert_eq!(body["items"][0]["qty"], "1.25");
        let prepared = Prepared {
            request: request.clone(),
            body: body.clone(),
        };
        let mut result = json!({"header":{"so_id":draft.order.to_uuid(),"document_no":"SO-1","customer_code":"C-1","currency":"CNY","request_id":request,"version_no":1,"status":"SUBMITTED","total_cent":0},"items":[{"line_no":1,"item_code":"S-1","qty_micros":"1250000","price_cent":0,"discount_cent":0,"tax_cent":0,"due_date":draft.lines[0].date,"address":"Dock","source_kind":"RESALE","warehouse_id":null}]});
        assert!(matches(&prepared, &result).is_ok());
        result["header"]["total_cent"] = json!(1);
        assert!(matches(&prepared, &result).is_err());
        result["header"]["total_cent"] = json!(0);
        result["header"]["version_no"] = json!(2);
        assert!(matches(&prepared, &result).is_err());
        result["header"]["version_no"] = json!(1);
        result["items"][0]["qty_micros"] = json!("2000000");
        assert!(matches(&prepared, &result).is_err());
        Ok(())
    }
    #[test]
    fn trade_sales_checkpoint_rejects_another_actor_and_changed_pending_body() -> Result<(), String>
    {
        let client =
            crate::http::HttpClient::new("https://trade.qa.invalid").map_err(|e| e.to_string())?;
        let scope = ActionScope::new(&client, RowId(1), RowId(2))?;
        let other = ActionScope::new(&client, RowId(1), RowId(3))?;
        let mut draft = Draft {
            number: "SO-1".into(),
            customer: "C-1".into(),
            ..Default::default()
        };
        draft.lines[0].sku = "S-1".into();
        draft.lines[0].address = "Dock".into();
        let request = uuid::Uuid::new_v4().to_string();
        let body = draft.payload(&request)?;
        let mut checkpoint = Checkpoint {
            schema: 1,
            scope: scope.clone(),
            draft,
            pending: Some(Prepared { request, body }),
            rejected: None,
        };
        let bytes = serde_json::to_vec(&checkpoint).map_err(|_| "encode")?;
        assert!(Checkpoint::decode(&bytes, &scope).is_ok());
        assert!(Checkpoint::decode(&bytes, &other).is_err());
        if let Some(pending) = &mut checkpoint.pending {
            pending.body["items"][0]["price_cent"] = json!(99);
        }
        let bytes = serde_json::to_vec(&checkpoint).map_err(|_| "encode")?;
        assert!(Checkpoint::decode(&bytes, &scope).is_err());
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn trade_sales_sqlite_checkpoint_reopens_original_fields_and_isolates_actor()
    -> Result<(), String> {
        use crate::action_store::order_recovery::OrderRecoveryStore;
        let root = tempfile::tempdir().map_err(|_| "test directory")?;
        let client = crate::http::HttpClient::new("https://owned-trade.qa.invalid")
            .map_err(|e| e.to_string())?;
        let scope = ActionScope::new(&client, RowId(1), RowId(2))?;
        let other = ActionScope::new(&client, RowId(1), RowId(3))?;
        let loaded = OrderRecoveryStore::open_trade_test(scope.clone(), root.path()).await?;
        let mut draft = Draft {
            number: "SO-STORAGE".into(),
            customer: "C-1".into(),
            ..Default::default()
        };
        draft.lines[0].sku = "S-1".into();
        draft.lines[0].address = "Dock".into();
        draft.lines[0].qty = "1.2500".into();
        draft.changed = true;
        let request = uuid::Uuid::new_v4().to_string();
        let prepared = Prepared {
            body: draft.payload(&request)?,
            request,
        };
        let checkpoint = Checkpoint {
            schema: 1,
            scope: scope.clone(),
            draft,
            pending: Some(prepared.clone()),
            rejected: None,
        };
        let bytes = serde_json::to_vec(&checkpoint).map_err(|_| "checkpoint encode")?;
        let ctx = eframe::egui::Context::default();
        let mut write = loaded.store.commit(loaded.revision, Some(&bytes), &ctx)?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let revision = loop {
            if let Some(result) = write.poll() {
                break result?;
            }
            if tokio::time::Instant::now() > deadline {
                return Err("storage acknowledgement timeout".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        };
        assert_eq!(revision, 1);
        let reopened = OrderRecoveryStore::open_trade_test(scope.clone(), root.path()).await?;
        let restored = Checkpoint::decode(
            reopened
                .bytes
                .as_deref()
                .ok_or("original checkpoint missing")?,
            &scope,
        )?;
        let pending = restored.pending.ok_or("original request missing")?;
        assert_eq!(pending.request, prepared.request);
        assert_eq!(pending.body, prepared.body);
        let isolated = OrderRecoveryStore::open_trade_test(other, root.path()).await?;
        assert!(isolated.bytes.is_none());
        loaded.store.close_test().await;
        reopened.store.close_test().await;
        isolated.store.close_test().await;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn trade_sales_sqlite_second_window_conflict_keeps_first_original_checkpoint()
    -> Result<(), String> {
        use crate::action_store::order_recovery::OrderRecoveryStore;
        let root = tempfile::tempdir().map_err(|_| "test directory")?;
        let client = crate::http::HttpClient::new("https://owned-trade.qa.invalid")
            .map_err(|e| e.to_string())?;
        let scope = ActionScope::new(&client, RowId(1), RowId(2))?;
        let ctx = eframe::egui::Context::default();
        let first = OrderRecoveryStore::open_trade_test(scope.clone(), root.path()).await?;
        let second = OrderRecoveryStore::open_trade_test(scope.clone(), root.path()).await?;
        let a = br#"{"original_request":"window-A"}"#;
        let b = br#"{"original_request":"window-B"}"#;
        let mut write = first.store.commit(first.revision, Some(a), &ctx)?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(result) = write.poll() {
                assert_eq!(result?, 1);
                break;
            }
            if tokio::time::Instant::now() > deadline {
                return Err("first acknowledgement timeout".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        let mut conflict = second.store.commit(second.revision, Some(b), &ctx)?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(result) = conflict.poll() {
                assert!(result.is_err());
                break;
            }
            if tokio::time::Instant::now() > deadline {
                return Err("conflict acknowledgement timeout".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        let read = OrderRecoveryStore::open_trade_test(scope, root.path()).await?;
        assert_eq!(read.bytes.as_deref(), Some(a.as_slice()));
        first.store.close_test().await;
        second.store.close_test().await;
        read.store.close_test().await;
        Ok(())
    }
}
