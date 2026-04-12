# 状态机落地对齐与多 AI 开发清单

> 目标：基于当前实现对齐销售单/履约/WMS/售后状态机落地现状，明确是否需要新增状态表，并提供多 AI 并行开发的任务拆分清单。

## 1. 当前实现快照（数据库 + 接口）
### 1.1 订单/OMS（orders）
- 主表：`orders`（`migrations/0001_init.sql` + `0076_orders_extended_fields.sql` + `0079_oms_order_statuses.sql`）。
- 状态字段：`order_status`、`fulfillment_status`、`exception_status`、`review_status`、`oms_status`（`0079_oms_order_statuses.sql` 拆分映射）。
- 审核与异常：`order_reviews`（`0030_system_management.sql`）、`order_exception_tickets`（`0079_oms_order_statuses.sql`）。
- API：`/v1/tenants/{tenant_id}/orders*`，审核动作 `/orders/{order_id}/review`。

### 1.2 履约/WMS
- 主要表：`outbound_docs`、`outbound_doc_items`、`outbound_pick_tasks`、`waves`、`wave_docs`、`pack_records`（`0032_wms_outbound.sql`）。
- 状态字段：`outbound_docs.status` + `0079_global_status_fields.sql` 增加的 `outbound_status/pick_status/verify_status/pack_status/ship_status`。
- 波次：`waves.status` 默认 `CREATED`，波次运行/下发逻辑已存在。
- API：`/v1/tenants/{tenant_id}/wms/*`（详见 `crates/erp-api/src/lib.rs`）。

### 1.3 售后与财务
- 售后表：`after_sales`、`return_receipts`、`refund_docs`（`0079_global_status_fields.sql` 扩展 `after_sale_status/return_status/refund_status`）。
- API：`/v1/tenants/{tenant_id}/after-sales*`、`/after-sale-cases*`。

### 1.4 审计与日志
- `audit_logs` 表已存在（`0030_system_management.sql`），字段含 `action`、`entity_type`、`entity_id`、`before_json`、`after_json`。
- `record_audit_log` 已被订单、售后、财务等接口复用。

## 2. 状态表结论（是否需要新建）
### 2.1 结论
当前库中**未发现** `order_status_history`、`order_action_log` 或 `flow_*` 状态/流程表，只有通用 `audit_logs`。

### 2.2 影响
- 无法统一记录“状态迁移轨迹”和“动作幂等/回放”。
- 无法高效按订单/动作维度追溯状态流转。

### 2.3 落地方案（需确认）
**方案 A（最小改动）**  
继续使用 `audit_logs` 承载状态迁移：  
`entity_type=sales_order`，`action` 写动作码，`before_json/after_json` 写状态快照。  
适合快速落地，但查询/报表复杂度较高。

**方案 B（已确认）**  
新增结构化状态表（建议同时保留 `audit_logs`）：  
| 表 | 关键字段 | 用途 |
| --- | --- | --- |
| `order_status_history` | `tenant_id/order_id/from_status/to_status/action_code/operator_id/reason/created_at/payload` | 状态迁移追溯 |
| `order_action_log` | `tenant_id/order_id/action_code/idempotency_key/request_json/response_json/status/created_at` | 幂等与回放 |
| `flow_instance`（可选） | `flow_code/version/current_node/status/started_at/ended_at` | 审批流程实例 |

✅ 已确认使用方案 B：落地 `order_status_history` 与 `order_action_log`。

### 2.4 销售单模型决策
当前没有 `sales_orders` 表，仅有平台订单 `orders`。  
✅ 已确认：**新增 `sales_orders/sales_order_items`** 作为内部销售单主表，后续由接口层完成映射与同步。

## 3. 文档对齐说明
- `docs/spec/SALES_ORDER_STATE_MACHINE.md`、`docs/spec/14_action_permission_matrix.md`、`docs/spec/15_integration_event_contract.md` 中引用的 `order_status_history/order_action_log` 尚未落地。
- 本文档作为“现有实现对齐”基准，后续实现请以**方案 A/B 决策**为准同步更新相关文档与 OpenAPI。

## 4. 多 AI 并行开发拆分清单（执行文档）
### 4.1 数据库与 Repo（AI-DB）
**目标**：补齐状态表/索引/模型。  
**产出**：
- 新增迁移（方案 B）：`order_status_history`、`order_action_log`（含索引）。
- `erp-repo` 中新增 CRUD/repo 接口。

### 4.2 状态机服务层（AI-CORE）
**目标**：实现销售单状态机动作与校验。  
**产出**：
- 动作入口：`Submit/Approve/Reject/Confirm/Close/Refund` 等。
- 统一写入状态历史（方案 A/B）。
- 幂等校验与版本锁校验。

### 4.3 WMS 对接（AI-WMS）
**目标**：对接波次/出库/包裹回传与状态映射。  
**产出**：
- 创建出库单/波次运行/回传落库。
- 更新 `delivery_status` 与 WMS 扩展字段。
- 回传幂等处理（`idempotency_key`）。

### 4.4 售后与财务（AI-AFTER/FIN）
**目标**：补齐售后/退款/对账的状态联动与日志。  
**产出**：
- 售后动作写入状态历史/审计日志。
- 退款/开票联动 `payment_status/refund_status/invoice_status`。

### 4.5 OpenAPI 与前端对齐（AI-API/WEB）
**目标**：文档与前端动作/状态同步。  
**产出**：
- OpenAPI 补齐 `available_actions`、`status` 枚举。
- 前端按 `sales_status` + `available_actions` 渲染。

## 5. 执行顺序与验收要点
1. **先定稿方案 A/B 与销售单模型决策**（是否新建 `sales_orders`）。
2. **数据库迁移 + Repo**（状态表与索引）。
3. **动作接口与状态机逻辑**（含幂等与日志）。
4. **WMS 回传联动**（波次/出库/包裹）。
5. **OpenAPI + UI**（枚举与动作按钮对齐）。

验收要点：
- 每次动作均有审计/状态记录。
- 状态不回退、幂等键重复请求不重放。
- WMS 回传可追溯，`delivery_status` 与包裹状态一致。
