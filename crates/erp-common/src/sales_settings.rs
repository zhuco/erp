use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SALES_AUTO_DOWNLOAD_ORDERS: &str = "sales.auto_download_orders";
pub const SALES_AUTO_TO_REVIEW: &str = "sales.auto_to_review";
pub const SALES_DOWNLOAD_INTERVAL_MINUTES: &str = "sales.download_interval_minutes";
pub const SALES_AUTO_MERGE_NON_SHIPPING_TIME: &str = "sales.auto_merge_non_shipping_time";
pub const SALES_NON_SHIPPING_TIME_RANGE: &str = "sales.non_shipping_time_range";
pub const SALES_ENABLE_STOCK_SYNC: &str = "sales.enable_stock_sync";
pub const SALES_ENABLE_AFTER_SALES_DOWNLOAD: &str = "sales.enable_after_sales_download";
pub const SALES_ENABLE_LOGISTICS_UPLOAD: &str = "sales.enable_logistics_upload";

pub const SALES_CONFIG_KEYS: &[&str] = &[
    SALES_AUTO_DOWNLOAD_ORDERS,
    SALES_AUTO_TO_REVIEW,
    SALES_DOWNLOAD_INTERVAL_MINUTES,
    SALES_AUTO_MERGE_NON_SHIPPING_TIME,
    SALES_NON_SHIPPING_TIME_RANGE,
    SALES_ENABLE_STOCK_SYNC,
    SALES_ENABLE_AFTER_SALES_DOWNLOAD,
    SALES_ENABLE_LOGISTICS_UPLOAD,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesShopConfig {
    pub auto_download_orders: bool,
    pub auto_to_review: bool,
    pub download_interval_minutes: i64,
    pub auto_merge_non_shipping_time: bool,
    pub non_shipping_time_start: Option<String>,
    pub non_shipping_time_end: Option<String>,
    pub enable_stock_sync: bool,
    pub enable_after_sales_download: bool,
    pub enable_logistics_upload: bool,
}

impl Default for SalesShopConfig {
    fn default() -> Self {
        Self {
            auto_download_orders: true,
            auto_to_review: true,
            download_interval_minutes: 5,
            auto_merge_non_shipping_time: false,
            non_shipping_time_start: Some("00:00".to_string()),
            non_shipping_time_end: Some("08:00".to_string()),
            enable_stock_sync: true,
            enable_after_sales_download: true,
            enable_logistics_upload: true,
        }
    }
}

impl SalesShopConfig {
    pub fn apply_setting(&mut self, key: &str, value: &Value) {
        match key {
            SALES_AUTO_DOWNLOAD_ORDERS => {
                if let Some(v) = value.as_bool() {
                    self.auto_download_orders = v;
                }
            }
            SALES_AUTO_TO_REVIEW => {
                if let Some(v) = value.as_bool() {
                    self.auto_to_review = v;
                }
            }
            SALES_DOWNLOAD_INTERVAL_MINUTES => {
                if let Some(v) = value.as_i64() {
                    self.download_interval_minutes = v;
                }
            }
            SALES_AUTO_MERGE_NON_SHIPPING_TIME => {
                if let Some(v) = value.as_bool() {
                    self.auto_merge_non_shipping_time = v;
                }
            }
            SALES_NON_SHIPPING_TIME_RANGE => {
                if let Some(map) = value.as_object() {
                    if let Some(start) = map.get("start").and_then(|v| v.as_str()) {
                        self.non_shipping_time_start = Some(start.to_string());
                    }
                    if let Some(end) = map.get("end").and_then(|v| v.as_str()) {
                        self.non_shipping_time_end = Some(end.to_string());
                    }
                }
            }
            SALES_ENABLE_STOCK_SYNC => {
                if let Some(v) = value.as_bool() {
                    self.enable_stock_sync = v;
                }
            }
            SALES_ENABLE_AFTER_SALES_DOWNLOAD => {
                if let Some(v) = value.as_bool() {
                    self.enable_after_sales_download = v;
                }
            }
            SALES_ENABLE_LOGISTICS_UPLOAD => {
                if let Some(v) = value.as_bool() {
                    self.enable_logistics_upload = v;
                }
            }
            _ => {}
        }
    }

    pub fn to_settings(&self) -> Vec<(String, Value)> {
        vec![
            (
                SALES_AUTO_DOWNLOAD_ORDERS.to_string(),
                Value::Bool(self.auto_download_orders),
            ),
            (
                SALES_AUTO_TO_REVIEW.to_string(),
                Value::Bool(self.auto_to_review),
            ),
            (
                SALES_DOWNLOAD_INTERVAL_MINUTES.to_string(),
                Value::Number(self.download_interval_minutes.into()),
            ),
            (
                SALES_AUTO_MERGE_NON_SHIPPING_TIME.to_string(),
                Value::Bool(self.auto_merge_non_shipping_time),
            ),
            (
                SALES_NON_SHIPPING_TIME_RANGE.to_string(),
                serde_json::json!({
                    "start": self.non_shipping_time_start,
                    "end": self.non_shipping_time_end,
                }),
            ),
            (
                SALES_ENABLE_STOCK_SYNC.to_string(),
                Value::Bool(self.enable_stock_sync),
            ),
            (
                SALES_ENABLE_AFTER_SALES_DOWNLOAD.to_string(),
                Value::Bool(self.enable_after_sales_download),
            ),
            (
                SALES_ENABLE_LOGISTICS_UPLOAD.to_string(),
                Value::Bool(self.enable_logistics_upload),
            ),
        ]
    }
}
