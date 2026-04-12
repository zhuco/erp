# 发货与面单

## shipments
- 字段: `shipment_id` uuid pk, `tenant_id` uuid, `shop_id` uuid, `order_id` uuid, `platform` text, `platform_order_id` text, `warehouse_id` uuid, `carrier_code` text, `tracking_no` text, `status` text, `requested_at` timestamptz, `shipped_at` timestamptz, `updated_at` timestamptz
- 状态: `REQUESTED` / `SUCCESS` / `FAILED`
- 唯一约束: `unique (tenant_id, order_id)`（一单一包裹）
- 索引: `(tenant_id, shop_id, status)`, `(tenant_id, platform, platform_order_id)`

## waybills
- 字段: `waybill_id` uuid pk, `tenant_id` uuid, `shop_id` uuid, `platform` text, `carrier_code` text, `tracking_no` text, `source` text, `status` text, `allocated_at` timestamptz, `used_at` timestamptz
- 来源: `PLATFORM` / `OWN_POOL`
- 状态: `ALLOCATED` / `VOIDED` / `USED`
- 唯一约束: `unique (tenant_id, tracking_no)`
