# 库存模型说明

## 口径
- `inventory_balances.on_hand`: 实物可用库存（入库/退货/调整后累计）。
- `inventory_balances.reserved`: 已预留但未出库的数量（锁定量）。
- 可用库存 = `on_hand - reserved`（不落库，按需计算）。

## 流水
`inventory_ledgers` 记录库存变动流水（追加写入）：
- `kind`：ADJUST/RESERVE/RELEASE/SHIP/RETURN/TRANSFER_IN/TRANSFER_OUT
- `qty`：正负都允许，语义由 `kind` 决定。
- `ref_type`/`ref_id`：业务侧标识来源（订单/退货/调拨等）。

## 幂等
- `inventory_reservations` 通过 `unique (tenant_id, order_item_id)` 保证同一订单行的锁库幂等。
- `inventory_ledgers` 不做唯一约束，业务侧应基于 `ref_type`/`ref_id` 做去重或幂等控制。
- `inventory_balances` 通过主键 `(tenant_id, warehouse_id, sku_id)` 做幂等 upsert。

## 表结构摘要

### warehouses
- 字段: `warehouse_id` uuid pk, `tenant_id` uuid, `name` text, `code` text, `is_default` bool, `created_at` timestamptz, `updated_at` timestamptz
- 唯一约束: `unique (tenant_id, code)`

### inventory_balances
- 字段: `tenant_id` uuid, `warehouse_id` uuid, `sku_id` uuid, `on_hand` bigint, `reserved` bigint, `updated_at` timestamptz
- 主键: `(tenant_id, warehouse_id, sku_id)`

### inventory_ledgers
- 字段: `ledger_id` uuid pk, `tenant_id` uuid, `warehouse_id` uuid, `sku_id` uuid, `kind` text, `qty` bigint, `ref_type` text, `ref_id` text, `created_at` timestamptz
- 索引: `(tenant_id, sku_id, created_at)`, `(tenant_id, warehouse_id, created_at)`

### inventory_reservations
- 字段: `reservation_id` uuid pk, `tenant_id` uuid, `warehouse_id` uuid, `sku_id` uuid, `order_id` uuid, `order_item_id` uuid, `qty` bigint, `status` text, `created_at` timestamptz, `updated_at` timestamptz
- 唯一约束: `unique (tenant_id, order_item_id)`
