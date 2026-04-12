-- 0074: Product master data extensions

create table if not exists product_barcodes (
  barcode_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  barcode text not null,
  level text not null,
  unit text not null,
  ratio int not null default 1,
  is_default boolean not null default false,
  status text not null default 'ACTIVE',
  print_template text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, barcode)
);

create index if not exists idx_product_barcodes_tenant_sku
  on product_barcodes(tenant_id, sku_id);
create index if not exists idx_product_barcodes_tenant_status
  on product_barcodes(tenant_id, status);

create table if not exists product_unit_conversions (
  conversion_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  base_unit text not null,
  aux_unit text not null,
  ratio int not null default 1,
  rounding text not null default 'ROUND',
  scenario text not null,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, sku_id, aux_unit, scenario)
);

create index if not exists idx_product_unit_conversions_tenant_sku
  on product_unit_conversions(tenant_id, sku_id);
create index if not exists idx_product_unit_conversions_tenant_status
  on product_unit_conversions(tenant_id, status);

create table if not exists product_channel_availability (
  availability_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  channel_id uuid references channels(channel_id),
  warehouse_id uuid references warehouses(warehouse_id),
  status text not null default 'ON',
  available_qty int not null default 0,
  rule text,
  on_shelf_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, sku_id, channel_id, warehouse_id)
);

create index if not exists idx_product_channel_availability_tenant_sku
  on product_channel_availability(tenant_id, sku_id);
create index if not exists idx_product_channel_availability_tenant_status
  on product_channel_availability(tenant_id, status);

create table if not exists product_change_requests (
  request_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  title text not null,
  change_type text not null,
  status text not null default 'DRAFT',
  applicant text not null,
  approver text,
  reason text,
  history jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_product_change_requests_tenant_status
  on product_change_requests(tenant_id, status);
create index if not exists idx_product_change_requests_tenant_sku
  on product_change_requests(tenant_id, sku_id);

create table if not exists product_lifecycles (
  lifecycle_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  status text not null default 'ACTIVE',
  inventory_qty int not null default 0,
  remark text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, sku_id)
);

create index if not exists idx_product_lifecycles_tenant_status
  on product_lifecycles(tenant_id, status);
