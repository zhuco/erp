# WMS 出入库/采购/盘点链路 API 与权限同步（2026-04-08）

## 1. 目标

面向工业级电商 ERP 可上架交付，本次聚焦 WMS 入库与库存执行链路，补齐“业务化入口 API + 权限门禁 + OpenAPI 契约”。

覆盖范围：

1. 采购入库
2. 采购计划生成
3. 新建采购
4. 入库结算
5. 收货入库
6. 入库审核
7. 入库设置
8. 出入库记录
9. 库存变动明细
10. 调拨入库
11. 调拨出库
12. 调拨查询
13. 盘点开单
14. 盘点录入
15. 盘点单管理
16. 盘点审核

## 2. 本次新增 API（补缺口）

### 2.1 采购计划生成

- `POST /v1/tenants/{tenant_id}/procurement-plans/generate`
- 权限：`procurement_plans.write`
- 说明：基于采购建议自动聚合生成采购计划（支持 `sku_id/reason/max_items/min_suggested_qty` 过滤与默认单价）。

### 2.2 入库设置

- `GET /v1/tenants/{tenant_id}/wms/inbound-settings`
- `PUT /v1/tenants/{tenant_id}/wms/inbound-settings`
- 权限：
  - 读取：`wms.inbounds.read`
  - 更新：`wms.inbounds.write`
- 设置项：
  - `inv.inbound_require_audit`
  - `wms.inbound_require_inspection`
  - `wms.inbound_require_location`
  - `service.enable_return_inbound_link`

### 2.3 出入库记录与库存变动明细

- `GET /v1/tenants/{tenant_id}/wms/inout-records`
- `GET /v1/tenants/{tenant_id}/wms/stock-change-details`
- 权限：`inventory.read`
- 说明：统一复用库存台账能力（`inventory_ledgers`）提供 WMS 侧业务入口。

### 2.4 调拨别名接口

- `POST /v1/tenants/{tenant_id}/stock-transfers/{transfer_id}/outbound`
- `POST /v1/tenants/{tenant_id}/stock-transfers/{transfer_id}/inbound`
- 权限：`inventory.write`
- 说明：分别映射既有 `confirm-outbound` / `confirm-inbound`，对齐业务语义“调拨出库/调拨入库”。

### 2.5 盘点审核别名接口

- `POST /v1/tenants/{tenant_id}/stock-counts/{count_id}/audit`
- 权限：`inventory.write`
- 说明：映射既有 `confirm-diff`，用于盘点审核节点语义化。

### 2.6 采购入库别名接口

- `POST /v1/tenants/{tenant_id}/purchase-orders/{po_id}/inbound`
- 权限：`purchase_orders.write`
- 说明：映射既有 `purchase-orders/{po_id}/receive`。

### 2.7 入库结算别名接口

- `GET /v1/tenants/{tenant_id}/wms/inbound-settlements`
- `POST /v1/tenants/{tenant_id}/wms/inbound-settlements`
- 权限：
  - 查询：`procurement_reconciliations.read`
  - 创建：`procurement_reconciliations.write`
- 说明：映射采购对账（`procurement-reconciliations`），作为入库结算业务入口。

## 3. 需求项与 API 映射

| 需求项 | API |
| --- | --- |
| 采购入库 | `POST /purchase-orders/{po_id}/inbound`、`POST /purchase-orders/{po_id}/receive` |
| 采购计划生成 | `POST /procurement-plans/generate` |
| 新建采购 | `POST /purchase-orders` |
| 入库结算 | `GET/POST /wms/inbound-settlements` |
| 收货入库 | `POST /wms/inbounds/{doc_id}/receive` |
| 入库审核 | `POST /wms/inbounds/{doc_id}/audit` |
| 入库设置 | `GET/PUT /wms/inbound-settings` |
| 出入库记录 | `GET /wms/inout-records` |
| 库存变动明细 | `GET /wms/stock-change-details` |
| 调拨入库 | `POST /stock-transfers/{transfer_id}/inbound` |
| 调拨出库 | `POST /stock-transfers/{transfer_id}/outbound` |
| 调拨查询 | `GET /stock-transfers`、`GET /stock-transfers/{transfer_id}` |
| 盘点开单 | `POST /stock-counts` |
| 盘点录入 | `PATCH /stock-counts/{count_id}/items` |
| 盘点单管理 | `GET /stock-counts`、`GET /stock-counts/{count_id}`、`POST /stock-counts/{count_id}/cancel` |
| 盘点审核 | `POST /stock-counts/{count_id}/audit` |

## 4. 权限矩阵（发布建议）

1. 采购：`purchase_orders.read|write`
2. 采购计划：`procurement_plans.read|write`
3. 入库主流程：`wms.inbounds.read|write`
4. 库存执行：`inventory.read|write`
5. 入库结算：`procurement_reconciliations.read|write`

## 5. 本次回归验证

已完成：

1. `cargo test -p erp-api --test wms_inventory_workflow_permissions -- --nocapture`（11 通过）
2. `cargo test -p erp-api --test wms_workflow_permissions -- --nocapture`（4 通过）

结果：新增业务入口均具备权限门禁，缺权限返回 `403`。

## 6. Flow-Level Integration Tests (2026-04-08)

�������̼����ɲ����ļ���

- `crates/erp-api/tests/wms_inbound_inventory_workflow_integration.rs`

��������ҵ��ջ���

1. `procurement_inbound_and_settlement_flow_integration`
   - `POST /procurement-plans/generate`
   - `POST /purchase-orders`
   - `POST /purchase-orders/{po_id}/send`
   - `POST /purchase-orders/{po_id}/confirm`
   - `POST /purchase-orders/{po_id}/inbound`
   - `GET /wms/inout-records`
   - `GET /wms/stock-change-details`
   - `POST /wms/inbound-settlements`
   - `GET /wms/inbound-settlements`
2. `wms_manual_inbound_audit_receive_putaway_flow_integration`
   - `PUT /wms/inbound-settings` + `GET /wms/inbound-settings`
   - `POST /wms/inbounds`
   - `POST /wms/inbounds/{doc_id}/audit`
   - `POST /wms/inbounds/{doc_id}/receive`
   - `POST /wms/inbounds/{doc_id}/putaway`
   - `GET /wms/inout-records`

������֤���

1. `cargo test -p erp-api --test wms_inbound_inventory_workflow_integration -- --nocapture`
2. `cargo test -p erp-api --test wms_inventory_workflow_permissions -- --nocapture`
3. `cargo test -p erp-api --test wms_workflow_permissions -- --nocapture`
