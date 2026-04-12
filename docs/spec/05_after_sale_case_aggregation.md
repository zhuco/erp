# AfterSaleCase 聚合（售后 Case 投影）

## 背景
售后/退款事件在平台侧通常以 **case** 形态存在（一个 case 可能覆盖多个子单）。
ERP 侧需要对 `order_items` 中的售后字段进行聚合，形成可查询的售后 case 视图。

## 目标
- 基于 `order_items.after_sale_id` 聚合生成 `AfterSaleCase`
- 支持按租户/店铺/平台查询与详情查看
- 保持 at-least-once 语义，消费端幂等

## 数据模型
- `after_sale_cases`：case 级汇总信息（状态、金额、数量）
- `after_sale_case_items`：case 与子单行的关联快照
- 唯一键：`(tenant_id, platform, shop_id, after_sale_id)`

## 聚合逻辑
1) Worker 订阅 `ORDER_UPSERTED`，读取该订单 `order_items`
2) 先删除该订单在 `after_sale_case_items` 中的旧记录
3) 按 `after_sale_id` 分组写入 case/items（upsert）
4) 基于 case_items 重新计算汇总字段：
   - `refund_state/after_sale_state` 采用 **优先级** 聚合（SUCCESS > PROCESSING > CLOSED > UNKNOWN > NONE）
   - `refund_amount_cent`/`item_count`/`total_qty`/`order_count` 重新汇总

## 幂等性
- case 表使用唯一键 upsert，确保重复消费不会生成新 case
- case_items 以 `(tenant_id, case_id, order_id, line_id)` 作为主键
- 重复事件会先清空该订单关联项，再重建快照，保持一致性

## API
- `GET /v1/tenants/{tenant_id}/after-sale-cases`
- `GET /v1/tenants/{tenant_id}/after-sale-cases/{case_id}`
