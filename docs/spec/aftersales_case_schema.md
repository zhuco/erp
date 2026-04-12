# 售后聚合 Case Schema

## 1. `after_sale_cases`
售后单/退款单聚合表（按 `tenant_id` 隔离）。

**字段**
- `case_id` uuid pk，默认 `gen_random_uuid()`
- `tenant_id` uuid not null，租户
- `platform` text not null，平台标识
- `shop_id` uuid not null，店铺
- `platform_case_id` text not null，平台售后/退款单 ID
- `platform_order_id` text，平台订单 ID
- `platform_sku_order_id` text，子单/子订单 ID
- `case_type` text not null，`refund/return/exchange/repair/reship/other`
- `case_status` text not null，`processing/success/closed/rejected/unknown`
- `refund_amount_cent` bigint not null，默认 0
- `currency` text not null，默认 `CNY`
- `reason` text，可空
- `evidence` jsonb，可空
- `raw` jsonb not null，原始字段快照
- `created_at` timestamptz not null
- `updated_at` timestamptz not null
- `platform_created_at` timestamptz，可空
- `platform_updated_at` timestamptz，可空

**约束**
- 主键：`case_id`
- 幂等唯一：`unique(tenant_id, platform, shop_id, platform_case_id)`

**索引**
- `idx_cases_tenant_updated_at (tenant_id, updated_at desc)`
- `idx_cases_tenant_platform_order (tenant_id, platform, platform_order_id)`
- `idx_cases_tenant_shop_status (tenant_id, shop_id, case_status)`

**可选分区/归档策略（默认不分区）**
- 当近 3 个月累计超过 3000 万条时，可考虑按月 RANGE 分区（基于 `case_month`）。
- 分区键要求出现在唯一约束/主键中，因此需引入 `case_month` 并调整约束。
- 归档建议：分离早期月份分区到归档库或冷存储。
- 参考迁移模板（不直接启用，需按月份扩展）：
  - `alter table after_sale_cases add column if not exists case_month date;`
  - `update after_sale_cases set case_month = date_trunc('month', created_at)::date where case_month is null;`
  - `alter table after_sale_cases alter column case_month set not null;`
  - `create table after_sale_cases_p (like after_sale_cases including defaults) partition by range (case_month);`
  - `alter table after_sale_cases_p add primary key (tenant_id, case_month, case_id);`
  - `alter table after_sale_cases_p add unique (tenant_id, case_month, platform, shop_id, platform_case_id);`
  - `create table after_sale_cases_2025_01 partition of after_sale_cases_p for values from ('2025-01-01') to ('2025-02-01');`
  - `create index idx_cases_tenant_updated_at on after_sale_cases_p(tenant_id, updated_at desc);`
  - `insert into after_sale_cases_p select * from after_sale_cases;`
  - `alter table after_sale_cases rename to after_sale_cases_legacy;`
  - `alter table after_sale_cases_p rename to after_sale_cases;`
  - `alter table after_sale_cases detach partition after_sale_cases_2024_01;`

## 2. `after_sale_case_links`
售后单与订单明细关联表（支持一单多品/多售后场景）。

**字段**
- `tenant_id` uuid not null
- `case_id` uuid not null
- `order_month` date not null
- `order_id` uuid not null
- `line_id` text not null
- `created_at` timestamptz not null

**约束**
- 主键：`(tenant_id, case_id, order_month, order_id, line_id)`

**索引**
- `idx_case_links_order_item (tenant_id, order_month, order_id, line_id)`

## 3. 数据保留建议
- `raw`/`evidence` 建议长期保留以便审计与回放。
- 可在业务侧设定归档策略：例如超过 24 个月迁移到冷存储。
