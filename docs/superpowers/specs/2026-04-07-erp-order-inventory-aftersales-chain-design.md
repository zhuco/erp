# 工业级电商 ERP 订单-库存-售后-发货-入库链路设计与落地（2026-04-07）

## 1. 目标与范围

目标：将 `订单 -> 发货 -> 售后 -> 入库 -> 库存` 主链路提升到可市场发布的企业级标准，确保接口语义清晰、权限闭环完整、审计可追踪。

本轮范围（已落地代码）：
- 修复售后入库联动接口的权限缺口（防止仅凭售后权限即可写库存/入库单）。
- 新增订单链路聚合查询 API，输出订单、发货、售后、入库的统一视图。
- 补充权限回归测试，确保后续不会回退。

## 2. 端到端链路状态机

### 2.1 订单（OMS）
- `NEW/REVIEW_PENDING`：待审核。
- `PENDING_SHIPMENT/PENDING_ALLOCATE`：已审单，待分配/待发货。
- `SHIPPED/DONE`：已发货/履约完成。
- `CANCELLED`：订单终态取消。

关键原则：
- 发货动作必须联动库存扣减（`inventory.write`）与物流动作（`shipping.write`）。
- 审核回退、异常处理必须写审计与状态历史。

### 2.2 发货（Fulfillment/OMS Shipment）
- `CREATED`：创建履约单。
- `SHIPPED`：已出库发运。
- `CANCEL_REQUESTED/CANCELLED`：取消申请及取消完成。

关键原则：
- 发货可见性与操作权限分离：读 `shipping.read`，写 `shipping.write`。
- 发货链路对订单状态回写必须具备幂等保障与审计记录。

### 2.3 售后（AfterSales）
- `OPEN`：售后创建。
- `APPROVED/REJECTED`：审核通过/拒绝。
- `RECEIVED/PUTAWAY`：退货收货与上架（可驱动入库与库存调整）。

关键原则：
- 售后收货/上架/撤销/重做是库存与入库域变更动作，不仅是售后域动作。

### 2.4 入库（WMS Inbound）
- `DRAFT/AUDITED/RECEIVING/PUTAWAY_DONE/CANCELLED`。
- 对售后逆向场景，`ref_type=AFTER_SALES`，`ref_id=after_sales_id` 形成单据关联。

关键原则：
- 入库单状态变化必须与库存流水一致，操作全程可追溯。

## 3. API 清单（市场发布视角）

### 3.1 新增聚合查询 API（本轮新增）
- `GET /v1/tenants/:tenant_id/orders/:order_id/chain`
- `GET /v1/tenants/:tenant_id/oms/orders/:order_id/chain`

返回聚合：
- 订单详情（含订单审计日志）
- 发货单列表（含发货明细）
- 售后明细列表（含售后审计日志）
- 与售后关联的入库单列表（`AFTER_SALES` 反向追踪）

读取权限要求（全部必需）：
- `orders.read`
- `shipping.read`
- `aftersales.read`
- `wms.inbounds.read`

### 3.2 权限修复 API（本轮增强）
以下接口在原有 `aftersales.write` 基础上，新增强制校验：
- `POST /v1/tenants/:tenant_id/after-sales/:after_sales_id/receive`
- `POST /v1/tenants/:tenant_id/after-sales/:after_sales_id/putaway`
- `POST /v1/tenants/:tenant_id/after-sales/:after_sales_id/receive/cancel`
- `POST /v1/tenants/:tenant_id/after-sales/:after_sales_id/receive/redo`

新增校验项：
- `inventory.write`
- `wms.inbounds.write`

## 4. 权限矩阵（订单/发货/售后/入库）

| 业务域 | 典型动作 | 必需权限 |
|---|---|---|
| 订单 | 查询订单详情/列表 | `orders.read` |
| 订单 | 审核/改收件信息/异常处理 | `orders.write` 或 `orders.review`（按接口） |
| 发货 | 查询发货单 | `shipping.read` |
| 发货 | 分配/发货/取消申请 | `shipping.write`（并按接口叠加 `orders.write`、`inventory.write`） |
| 售后 | 查询售后单 | `aftersales.read` |
| 售后 | 创建/审批/拒绝/退款 | `aftersales.write` |
| 售后入库联动 | 收货/上架/撤销/重做 | `aftersales.write` + `inventory.write` + `wms.inbounds.write` |
| 入库 | 查询入库单 | `wms.inbounds.read` |
| 入库 | 创建/收货/质检/上架/作废 | `wms.inbounds.write`（并受库存写能力约束） |

权限设计原则：
- 读写分离、域边界清晰。
- 跨域写操作采用“叠加权限”策略，杜绝单域权限越权。

## 5. 企业级发布基线

### 5.1 可运维性
- 全链路审计：订单、售后、入库写操作均应记录审计日志。
- 状态历史：关键状态流转写状态历史表，支持审计追责。
- 请求可追踪：透传 `trace_id/request_id`。

### 5.2 一致性与幂等
- 写接口要求幂等键（已有接口继续沿用）。
- 订单/发货/库存回写失败需具备明确错误码与可重放机制。

### 5.3 安全与租户隔离
- 每个接口先做 `tenant access` 校验，再做最小必要权限校验。
- 所有聚合查询必须在租户边界内进行二次资源校验（例如 shop scope）。

### 5.4 回归质量门禁
- 权限回归测试必须覆盖“缺任一必需权限即 403”。
- 链路聚合 API 必须有权限组合测试，防止将来回归成弱校验。

## 6. 本轮交付结论

已完成：
- 售后入库联动权限补齐（4 个接口）。
- 订单链路聚合 API 双路由新增（`orders` / `oms/orders`）。
- 新增权限回归测试并通过。

后续建议（下一迭代）：
- 为链路聚合 API 增加成功场景集成测试（含真实 DB 数据装配）。
- 补链路 SLA 指标（从售后创建到入库上架耗时，发货时效等）。
- 在 OpenAPI 中补齐新接口 schema 与权限说明，便于市场化对接。
