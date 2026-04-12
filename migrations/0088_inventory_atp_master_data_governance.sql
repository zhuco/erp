-- 0088: Inventory ATP / FEFO / packaging spec / reconciliation center

create table if not exists product_packaging_specs (
  spec_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  level_code text not null,
  unit_name text not null,
  units_per_level bigint not null default 1,
  length_mm int,
  width_mm int,
  height_mm int,
  weight_gram int,
  is_default boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, sku_id, level_code)
);

create index if not exists idx_product_packaging_specs_tenant_sku
  on product_packaging_specs(tenant_id, sku_id, is_default, updated_at desc);

create table if not exists inventory_reconciliation_center (
  reconciliation_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  expected_on_hand bigint not null,
  actual_on_hand bigint not null,
  diff_qty bigint not null,
  status text not null,
  source_ref text,
  created_at timestamptz not null default now()
);

create index if not exists idx_inventory_reconciliation_center_tenant_time
  on inventory_reconciliation_center(tenant_id, created_at desc);

create index if not exists idx_inventory_reconciliation_center_lookup
  on inventory_reconciliation_center(tenant_id, warehouse_id, sku_id, status);

