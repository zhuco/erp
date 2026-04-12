# 平台订单下载 / 商品映射解析 / 平台同步设置 API 更新（2026-04-08）

## 1. 目标

为工业级电商 ERP 补齐以下后端能力的“可发布”最小闭环：

1. 平台订单下载：手动下载、按时间周期下载、按订单号下载。
2. 平台商品与本地商品编码匹配：组合装编码匹配、手动匹配、货品解析。
3. 平台库存同步：手动触发 + 同步设置。
4. 售后/退换货同步：手动触发 + 同步设置。
5. 权限门禁、OpenAPI 契约与流程级集成测试同步。

## 2. 新增 API

### 2.1 平台订单下载

1. `POST /v1/tenants/{tenant_id}/shops/{shop_id}/platform-orders/download/manual`
2. `POST /v1/tenants/{tenant_id}/shops/{shop_id}/platform-orders/download/by-time-range`
3. `POST /v1/tenants/{tenant_id}/shops/{shop_id}/platform-orders/download/by-order-nos`

说明：

1. `manual` 与 `by-time-range` 走 `sync_jobs + sync_job_runs` 统一调度链路。
2. `by-order-nos` 走“直接拉单 + 投影入库 + ORDER_IMPORTED 出站事件”链路。
3. 订单项 SKU 自动应用渠道映射；缺失映射时投递 `MAPPING_MISSING` 事件。

### 2.2 商品解析与组合装匹配

1. `POST /v1/tenants/{tenant_id}/shop-products/parse`
2. `POST /v1/tenants/{tenant_id}/shop-products/bundle-match/auto`
3. `POST /v1/tenants/{tenant_id}/shop-products/bundle-match/manual`

说明：

1. 解析输出包含：`resolved_sku_id/resolved_sku_code/parse_source/reason`。
2. 解析优先级：已有映射 -> 编码匹配 -> 组合编码匹配 -> 名称规格匹配 -> 未解析。

### 2.3 店铺级同步设置

1. `GET/PUT /v1/tenants/{tenant_id}/shops/{shop_id}/sync-settings/orders`
2. `GET/PUT /v1/tenants/{tenant_id}/shops/{shop_id}/sync-settings/inventory`
3. `GET/PUT /v1/tenants/{tenant_id}/shops/{shop_id}/sync-settings/after-sales`

说明：

1. 统一复用 `sales_settings` 存储键。
2. 更新设置后同步维护 `sync_jobs` 的启停状态（`PENDING/DISABLED`）。

### 2.4 手动同步触发

1. `POST /v1/tenants/{tenant_id}/shops/{shop_id}/platform-sync/inventory/run`
2. `POST /v1/tenants/{tenant_id}/shops/{shop_id}/platform-sync/after-sales/run`
3. `POST /v1/tenants/{tenant_id}/shops/{shop_id}/platform-sync/returns-exchanges/run`

说明：

1. 库存同步从 `stock_sync_jobs` 批量筛选并置为待执行。
2. 售后/退换货同步统一复用 `refund_sync` 调度类型。

## 3. 权限矩阵

1. 平台订单下载：`orders.download`
2. 货品解析：`shop_products.read` 或 `shop_products.write`
3. 组合装自动/手动匹配：`shop_products.write`
4. 订单同步设置读取：`settings.read` 或 `channels.read` 或 `orders.read`
5. 订单同步设置更新：`settings.read` 或 `channels.write` 或 `orders.write`
6. 库存同步设置读取：`settings.read` 或 `channels.read` 或 `inventory.read`
7. 库存同步设置更新：`settings.read` 或 `channels.write` 或 `inventory.write`
8. 售后同步设置读取：`settings.read` 或 `channels.read` 或 `aftersales.read`
9. 售后同步设置更新：`settings.read` 或 `channels.write` 或 `aftersales.write`
10. 库存手动同步触发：`inventory.write`
11. 售后/退换货手动同步触发：`aftersales.write`

## 4. OpenAPI 同步

已在 `openapi/erp_api_v1.yaml` 增补：

1. 上述新增路径与 `x-permissions`。
2. 新增请求/响应 schema（订单下载、同步设置、同步触发、商品解析/匹配）。

## 5. 测试补全

### 5.1 权限测试

文件：

1. `crates/erp-api/tests/platform_sync_and_mapping_permissions.rs`

覆盖：

1. 平台订单下载权限拦截
2. 商品解析/组合装匹配权限拦截
3. 同步设置更新权限拦截
4. 库存/售后手动同步权限拦截

### 5.2 流程级集成测试

文件：

1. `crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs`

流程覆盖：

1. 同步设置（订单/库存/售后）读写闭环
2. 平台订单手动下载与时间窗下载触发
3. 平台库存同步触发
4. 平台售后与退换货同步触发
5. `sync_jobs/sync_job_runs/settings/stock_sync_jobs` 结果落库校验

说明：

1. 未配置 `DATABASE_URL` 时按约定自动跳过（不阻断 CI）。

## 6. 回归命令

1. `cargo check -p erp-api --lib`
2. `cargo test -p erp-api --test platform_sync_and_mapping_permissions -- --nocapture`
3. `cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture`
