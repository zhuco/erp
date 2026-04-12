# 订单链路 API 与权限同步说明（2026-04-07）

## 背景

为支持工业级电商 ERP 的订单链路闭环，本次已在后端落地并同步 OpenAPI 文档以下能力：

1. 下载订单（触发拉单任务）
2. 新建订单（手工建单）
3. 审核订单（既有）
4. 复核订单（新增）
5. 提交订单（新增，进入配货分配）

同时补齐了对应权限点文档，确保租户权限控制与接口契约一致。

## 本次新增/补齐接口

### 主路径（`/orders`）

1. `POST /v1/tenants/{tenant_id}/orders`
- 用途：手工新建订单
- 权限：`orders.create`
- 请求体：`ManualOrderCreateRequest`
- 响应：`201`，`OmsOrderDetail`

2. `POST /v1/tenants/{tenant_id}/orders/new`
- 用途：手工新建订单（显式新建路径）
- 权限：`orders.create`
- 请求体：`ManualOrderCreateRequest`
- 响应：`201`，`OmsOrderDetail`

3. `POST /v1/tenants/{tenant_id}/orders/download`
- 用途：触发订单下载任务
- 权限：`orders.download`
- 请求体：`OrderDownloadRequest`
- 响应：`202`，`SyncJob`

4. `POST /v1/tenants/{tenant_id}/orders/{order_id}/review`
- 用途：审核订单（既有）
- 权限：`orders.write`
- 请求体：`OrderReviewRequest`
- 响应：`200`，`OrderReviewState`

5. `POST /v1/tenants/{tenant_id}/orders/{order_id}/recheck`
- 用途：复核已审核通过订单
- 权限：`orders.recheck`
- 请求体：`OrderRecheckRequest`
- 响应：`200`，`OrderReviewState`

6. `POST /v1/tenants/{tenant_id}/orders/{order_id}/submit`
- 用途：提交订单进入分配/配货
- 权限：`orders.submit`
- 请求体：`OrderSubmitRequest`
- 响应：`200`，`OmsAllocationResponse`

### 别名路径（`/oms/orders`）

1. `POST /v1/tenants/{tenant_id}/oms/orders`
- 用途：手工新建订单（别名）
- 权限：`orders.create`
- 请求体：`ManualOrderCreateRequest`
- 响应：`201`，`OmsOrderDetail`

2. `POST /v1/tenants/{tenant_id}/oms/orders/new`
- 用途：手工新建订单（显式新建别名路径）
- 权限：`orders.create`
- 请求体：`ManualOrderCreateRequest`
- 响应：`201`，`OmsOrderDetail`

3. `POST /v1/tenants/{tenant_id}/oms/orders/download`
- 用途：触发订单下载任务（别名）
- 权限：`orders.download`
- 请求体：`OrderDownloadRequest`
- 响应：`202`，`SyncJob`

4. `POST /v1/tenants/{tenant_id}/oms/orders/{order_id}/review`
- 用途：审核订单（别名，既有）
- 权限：`orders.write`
- 请求体：`OrderReviewRequest`
- 响应：`200`，`OrderReviewState`

5. `POST /v1/tenants/{tenant_id}/oms/orders/{order_id}/recheck`
- 用途：复核已审核通过订单（别名）
- 权限：`orders.recheck`
- 请求体：`OrderRecheckRequest`
- 响应：`200`，`OrderReviewState`

6. `POST /v1/tenants/{tenant_id}/oms/orders/{order_id}/submit`
- 用途：提交订单进入分配/配货（别名）
- 权限：`orders.submit`
- 请求体：`OrderSubmitRequest`
- 响应：`200`，`OmsAllocationResponse`

## 新增 OpenAPI Schema

1. `OrderDownloadRequest`
- 字段：`shop_id`、`cursor`、`watermark`、`last_time`（兼容别名，已标记 deprecated）

2. `ManualOrderItemRequest`
- 字段：`line_id`、`sku_id`、`title`、`qty`、`price_cent`、`amount_cent`、`pay_amount_cent`、`discount_cent`

3. `ManualOrderCreateRequest`
- 关键字段：`shop_id`、`platform_order_id`、`items`
- 扩展字段：收件信息、物流信息、买卖家备注、下单/平台更新时间、金额字段等

4. `OrderRecheckRequest`
- 字段：`reason`

5. `OrderSubmitRequest`
- 字段：`carrier_code`

## 权限矩阵（本次变更范围）

1. `orders.download`：订单下载
2. `orders.create`：新建订单
3. `orders.recheck`：复核订单
4. `orders.submit`：提交订单
5. `orders.write`：审核等既有写操作（未变更）

## 链路建议（业务顺序）

1. 下载订单：`download`
2. 新建订单：`new` 或 `POST /orders`
3. 审核订单：`review`
4. 复核订单：`recheck`
5. 提交订单：`submit`

说明：`recheck` 与 `submit` 均要求订单处于可继续推进的审核状态，后端会执行状态与数据完整性校验。

