# 商品档案 + SKU 映射（多平台多店铺）

## products
- 字段: `product_id` uuid pk, `tenant_id` uuid, `name` text, `brand` text, `category` text, `attrs` jsonb, `created_at` timestamptz, `updated_at` timestamptz
- 索引: 无
- 唯一约束: `product_id` 主键

## skus
- 字段: `sku_id` uuid pk, `tenant_id` uuid, `product_id` uuid fk, `sku_code` text, `barcode` text, `name` text, `spec` jsonb, `weight_gram` int, `length_mm` int, `width_mm` int, `height_mm` int, `is_active` boolean, `created_at` timestamptz, `updated_at` timestamptz
- 索引: `idx_skus_tenant_barcode` (`tenant_id`, `barcode`)
- 唯一约束: `unique (tenant_id, sku_code)`

## channel_sku_mappings
- 字段: `mapping_id` uuid pk, `tenant_id` uuid, `shop_id` uuid, `platform` text, `platform_sku_id` text, `platform_outer_sku_id` text, `sku_id` uuid, `status` text, `confidence` int, `created_at` timestamptz, `updated_at` timestamptz
- 索引: `idx_channel_sku_mappings_sku` (`tenant_id`, `sku_id`), `idx_channel_sku_mappings_shop_platform` (`tenant_id`, `shop_id`, `platform`)
- 唯一约束: `unique (tenant_id, shop_id, platform, platform_sku_id)`
