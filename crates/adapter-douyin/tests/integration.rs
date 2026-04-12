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
