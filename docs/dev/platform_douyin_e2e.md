# 抖音电商（Douyin/Jinritemai）平台 E2E 验收

> 目标：跑通授权 → token 刷新 → 拉单补偿 → 订单入库 → 发货回传 → 库存回写的闭环。

## 1. 环境变量（示例）

```bash
export DOUYIN_APP_KEY=your_app_id
export DOUYIN_APP_SECRET=your_app_secret
export DOUYIN_REDIRECT_URI=https://your-domain.com/oauth/douyin/callback
export DOUYIN_WEBHOOK_SECRET=your_event_secret
export ERP_API=http://localhost:8080
export TOKEN=your_jwt_token
export TENANT_ID=your_tenant_id
export SHOP_ID=your_shop_id
```

## 2. 配置店铺凭证（shop.credentials + auth_meta）

```bash
curl -X PATCH "$ERP_API/v1/tenants/$TENANT_ID/shops/$SHOP_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"credentials\": {
      \"app_key\": \"$DOUYIN_APP_KEY\",
      \"app_secret\": \"$DOUYIN_APP_SECRET\",
      \"access_token\": \"\",
      \"gateway_base\": \"https://openapi.jinritemai.com\",
      \"order_list_path\": \"/order/searchList\",
      \"order_detail_path\": \"/order/orderDetail\",
      \"ship_path\": \"/order/logisticsAdd\",
      \"stock_path\": \"/sku/syncStock\",
      \"sign_method\": \"md5\",
      \"webhook_secret\": \"$DOUYIN_WEBHOOK_SECRET\"
    },
    \"auth_meta_json\": {
      \"authorize_url\": \"https://openapi.jinritemai.com/oauth2/authorize?app_id=$DOUYIN_APP_KEY&response_type=code&scope=all\"
    }
  }"
```

## 3. 授权与回调

1) 获取授权链接：

```bash
curl -X POST "$ERP_API/v1/tenants/$TENANT_ID/shops/$SHOP_ID/auth/authorize" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"redirect_uri\":\"$DOUYIN_REDIRECT_URI\"}"
```

2) 在浏览器完成 Douyin 授权，拿到 `access_token`/`refresh_token`。

3) 调用回调接口写入平台 token：

```bash
curl -X POST "$ERP_API/v1/tenants/$TENANT_ID/shops/$SHOP_ID/auth/callback" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"access_token\": \"ACCESS_TOKEN_FROM_DOUYIN\",
    \"refresh_token\": \"REFRESH_TOKEN_FROM_DOUYIN\",
    \"expires_at\": \"2026-01-10T00:00:00Z\",
    \"scopes\": \"all\"
  }"
```

## 4. 拉单补偿 → 订单入库

```bash
curl -X POST "$ERP_API/v1/tenants/$TENANT_ID/shops/$SHOP_ID/sync-jobs/run" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"job_type\":\"order_sync\"}"
```

查看入库结果：

```sql
select platform_order_id, status, platform_updated_at
from orders
where tenant_id = '$TENANT_ID' and platform = 'douyin'
order by updated_at desc
limit 5;
```

## 5. 发货回传（SHIPMENT_SHIPPED 触发）

1) 创建 OMS 发货单：

```bash
curl -X POST "$ERP_API/v1/tenants/$TENANT_ID/oms/orders/$ORDER_ID/shipments" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"warehouse_id\":\"$WAREHOUSE_ID\",\"carrier_code\":\"SF\",\"tracking_no\":\"SF123456\"}"
```

2) 触发发货回传：

```bash
curl -X POST "$ERP_API/v1/tenants/$TENANT_ID/oms/shipments/$SHIPMENT_ID/ship" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"shipped_at\":\"2026-01-10T10:00:00Z\"}"
```

查看发货推送结果：

```sql
select shipment_id, status, last_error
from shipments
where tenant_id = '$TENANT_ID'
order by updated_at desc
limit 5;
```

## 6. 库存回写（INVENTORY_CHANGED / stock_sync_jobs）

```bash
curl -X POST "$ERP_API/v1/tenants/$TENANT_ID/inv/adjust" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"warehouse_id\":\"$WAREHOUSE_ID\",\"sku_id\":\"$SKU_ID\",\"quantity_delta\":5}"
```

查看库存同步任务：

```sql
select job_id, status, attempts, last_error
from stock_sync_jobs
where tenant_id = '$TENANT_ID'
order by updated_at desc
limit 5;
```
