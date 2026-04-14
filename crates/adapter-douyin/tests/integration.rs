use chrono::{TimeZone, Utc};
use erp_adapter::SyncCursor;
use erp_adapter::{PlatformAdapter, ShopAuthContext};
use erp_adapter_douyin::Adapter;
use erp_types::{Platform, ShopId, TenantId};
use httpmock::Method::POST;
use httpmock::MockServer;

fn shop_with_gateway(base: &str) -> ShopAuthContext {
    ShopAuthContext {
        tenant_id: "00000000-0000-0000-0000-000000000000"
            .parse::<TenantId>()
            .unwrap(),
        shop_id: "00000000-0000-0000-0000-000000000000"
            .parse::<ShopId>()
            .unwrap(),
        platform: Platform::Douyin,
        platform_shop_id: "douyin-shop-001".to_string(),
        credentials: serde_json::json!({
            "app_key": "app-123",
            "app_secret": "secret-456",
            "access_token": "access-789",
            "gateway_base": base,
            "order_list_path": "/order/searchList",
            "order_list_method": "order.searchList",
            "order_detail_path": "/order/orderDetail",
        }),
        auth_meta: serde_json::json!({}),
    }
}

#[tokio::test]
async fn pull_orders_paginates_and_fetches_detail() {
    let server = MockServer::start();
    let order_list_page1 = serde_json::json!({
        "code": 0,
        "data": {
            "shop_order_list": [
                {"order_id": "ORDER-1", "update_time": 1700000100}
            ],
            "has_more": true,
            "cursor": "2",
            "page": 1,
            "size": 1
        }
    });
    let order_list_page2 = serde_json::json!({
        "code": 0,
        "data": {
            "shop_order_list": [
                {"order_id": "ORDER-2", "update_time": 1700000200}
            ],
            "has_more": false,
            "page": 2,
            "size": 1
        }
    });

    let _list_page1 = server.mock(|when, then| {
        when.method(POST)
            .path("/order/searchList")
            .body_contains("%22page%22%3A1");
        then.status(200).json_body(order_list_page1.clone());
    });
    let _list_page2 = server.mock(|when, then| {
        when.method(POST)
            .path("/order/searchList")
            .body_contains("%22page%22%3A2");
        then.status(200).json_body(order_list_page2.clone());
    });

    let order_detail_1 = serde_json::json!({
        "code": 0,
        "data": {
            "shop_order_detail": {
                "order_id": "ORDER-1",
                "order_status": 3,
                "create_time": 1700000000,
                "update_time": 1700000100,
                "pay_amount": 100,
                "sku_order_list": [
                    {"out_sku_id": "SKU-1", "sku_id": 1001, "product_name": "测试商品A", "item_num": 1, "goods_price": 100, "order_amount": 100}
                ]
            }
        }
    });
    let order_detail_2 = serde_json::json!({
        "code": 0,
        "data": {
            "shop_order_detail": {
                "order_id": "ORDER-2",
                "order_status": 3,
                "create_time": 1700000001,
                "update_time": 1700000200,
                "pay_amount": 200,
                "sku_order_list": [
                    {"out_sku_id": "SKU-2", "sku_id": 1002, "product_name": "测试商品B", "item_num": 2, "goods_price": 100, "order_amount": 200}
                ]
            }
        }
    });

    let _detail_1 = server.mock(|when, then| {
        when.method(POST)
            .path("/order/orderDetail")
            .body_contains("ORDER-1");
        then.status(200).json_body(order_detail_1.clone());
    });
    let _detail_2 = server.mock(|when, then| {
        when.method(POST)
            .path("/order/orderDetail")
            .body_contains("ORDER-2");
        then.status(200).json_body(order_detail_2.clone());
    });

    let adapter = Adapter::new();
    let shop = shop_with_gateway(&server.base_url());
    let start_time = Utc.timestamp_opt(1700000000, 0).unwrap();
    let cursor = SyncCursor {
        cursor: None,
        last_time: Some(start_time),
    };

    let result = adapter.pull_orders(&shop, cursor).await.expect("pull");
    assert_eq!(result.orders.len(), 1);
    assert_eq!(result.orders[0].platform_order_id, "ORDER-1");
    assert_eq!(result.next_cursor.as_deref(), Some("2"));
    assert_eq!(result.latest_time.unwrap().timestamp(), 1700000100);

    let cursor2 = SyncCursor {
        cursor: result.next_cursor.clone(),
        last_time: result.latest_time,
    };
    let result2 = adapter.pull_orders(&shop, cursor2).await.expect("pull2");
    assert_eq!(result2.orders.len(), 1);
    assert_eq!(result2.orders[0].platform_order_id, "ORDER-2");
    assert!(result2.next_cursor.is_none());
}

#[tokio::test]
async fn pull_refunds_should_use_refund_api_and_write_meta() {
    let server = MockServer::start();
    let refund_list = serde_json::json!({
        "code": 0,
        "data": {
            "list": [
                {
                    "order_id": "ORDER-R-1",
                    "order_status": 3,
                    "create_time": 1700000000,
                    "update_time": 1700000300,
                    "refund_reason": "商品破损",
                    "responsibility": "seller",
                    "evidence_urls": ["https://example.com/p1.jpg"],
                    "refund_updated_at": 1700000300,
                    "after_sale_id": "DY-AS-100",
                    "refund_status": 1,
                    "sku_order_list": [
                        {
                            "sku_order_id": "SO-R-1",
                            "out_sku_id": "SKU-R-1",
                            "item_num": 1,
                            "goods_price": 100,
                            "order_amount": 100,
                            "after_sale_info": {
                                "refund_status": 1,
                                "after_sale_status": 2,
                                "refund_amount": 100,
                                "after_sale_id": "DY-AS-100"
                            }
                        }
                    ]
                }
            ],
            "has_more": false
        }
    });
    let _refund = server.mock(|when, then| {
        when.method(POST)
            .path("/afterSale/list")
            .body_contains("order.refundList");
        then.status(200).json_body(refund_list.clone());
    });

    let adapter = Adapter::new();
    let mut shop = shop_with_gateway(&server.base_url());
    let mut creds = shop.credentials.as_object().cloned().unwrap_or_default();
    creds.insert(
        "refund_api_enabled".to_string(),
        serde_json::Value::Bool(true),
    );
    creds.insert(
        "refund_list_method".to_string(),
        serde_json::Value::String("order.refundList".to_string()),
    );
    creds.insert(
        "refund_list_path".to_string(),
        serde_json::Value::String("/afterSale/list".to_string()),
    );
    shop.credentials = serde_json::Value::Object(creds);

    let result = adapter
        .pull_refunds(
            &shop,
            SyncCursor {
                cursor: None,
                last_time: Some(Utc.timestamp_opt(1699999999, 0).unwrap()),
            },
        )
        .await
        .expect("pull refunds");
    assert_eq!(result.orders.len(), 1);
    let item_raw = &result.orders[0].items[0].raw;
    let meta = item_raw
        .get("_erp_refund_meta")
        .expect("refund meta should exist");
    assert_eq!(meta.get("reason").and_then(|v| v.as_str()), Some("商品破损"));
    assert_eq!(
        meta.get("responsibility").and_then(|v| v.as_str()),
        Some("seller")
    );
    assert_eq!(
        meta.get("source_refund_id").and_then(|v| v.as_str()),
        Some("DY-AS-100")
    );
    assert_eq!(
        meta.get("source_status").and_then(|v| v.as_str()),
        Some("1")
    );
    let urls = meta
        .get("evidence_urls")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].as_str(), Some("https://example.com/p1.jpg"));
    assert!(meta
        .get("updated_at")
        .and_then(|v| v.as_str())
        .map(|v| v.contains('T'))
        .unwrap_or(false));
}

#[tokio::test]
async fn pull_refunds_should_fallback_when_refund_api_failed() {
    let server = MockServer::start();
    let _refund_fail = server.mock(|when, then| {
        when.method(POST)
            .path("/afterSale/list")
            .body_contains("order.refundList");
        then.status(500).body("mock fail");
    });

    let adapter = Adapter::new();
    let mut shop = shop_with_gateway(&server.base_url());
    let mut creds = shop.credentials.as_object().cloned().unwrap_or_default();
    creds.insert(
        "refund_api_enabled".to_string(),
        serde_json::Value::Bool(true),
    );
    creds.insert(
        "refund_list_method".to_string(),
        serde_json::Value::String("order.refundList".to_string()),
    );
    creds.insert(
        "refund_list_path".to_string(),
        serde_json::Value::String("/afterSale/list".to_string()),
    );
    shop.credentials = serde_json::Value::Object(creds);
    shop.auth_meta = serde_json::json!({
        "mock_orders": [
            {
                "order_id": "ORDER-FALLBACK-1",
                "create_time": 1700000000,
                "update_time": 1700000400,
                "order_status": 3,
                "pay_amount": 100,
                "sku_order_list": [
                    {
                        "sku_order_id": "SO-F-1",
                        "out_sku_id": "SKU-F-1",
                        "item_num": 1,
                        "goods_price": 100,
                        "order_amount": 100,
                        "after_sale_info": {
                            "refund_status": 1,
                            "after_sale_status": 2,
                            "refund_amount": 100,
                            "after_sale_id": "DY-AS-FALLBACK"
                        }
                    }
                ]
            }
        ]
    });

    let result = adapter
        .pull_refunds(
            &shop,
            SyncCursor {
                cursor: None,
                last_time: Some(Utc.timestamp_opt(1699999999, 0).unwrap()),
            },
        )
        .await
        .expect("pull refunds with fallback");
    assert_eq!(result.orders.len(), 1);
    assert_eq!(result.orders[0].platform_order_id, "ORDER-FALLBACK-1");
}
