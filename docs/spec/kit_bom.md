# 套装/组合 SKU（KIT BOM）

## 目标
- 支持一个套装 SKU 由多个组件 SKU 组成。
- 通过 `sku_kind` 区分普通 SKU 与套装 SKU。

## 规则
- `skus.sku_kind`：`NORMAL` / `KIT`，默认 `NORMAL`。
- `sku_boms` 每行表示一个套装组件关系。
- `qty` 表示该组件在套装中的数量。

## 表结构

### skus（新增字段）
- 字段: `sku_kind` text default `NORMAL`

### sku_boms
- 字段: `tenant_id` uuid, `kit_sku_id` uuid, `component_sku_id` uuid, `qty` bigint
- 主键: `(tenant_id, kit_sku_id, component_sku_id)`
