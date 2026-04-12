-- 0051: Directory enhancements (warehouse status + logistics + tags)

alter table warehouses
  add column if not exists status text not null default 'ACTIVE';

create index if not exists idx_warehouses_tenant_status
  on warehouses(tenant_id, status, updated_at desc);

create table if not exists logistics_products (
  product_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  carrier_code text not null,
  code text not null,
  name text not null,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, carrier_code, code),
  foreign key (tenant_id, carrier_code) references carriers(tenant_id, carrier_code)
);

create index if not exists idx_logistics_products_tenant_time
  on logistics_products(tenant_id, created_at desc);
create index if not exists idx_logistics_products_tenant_carrier
  on logistics_products(tenant_id, carrier_code);

create table if not exists logistics_prices (
  price_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  carrier_code text not null,
  product_id uuid references logistics_products(product_id),
  region_code text not null,
  weight_min_kg double precision not null default 0,
  weight_max_kg double precision not null default 0,
  price_cent bigint not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, carrier_code, region_code, weight_min_kg, weight_max_kg),
  foreign key (tenant_id, carrier_code) references carriers(tenant_id, carrier_code)
);

create index if not exists idx_logistics_prices_tenant_time
  on logistics_prices(tenant_id, created_at desc);
create index if not exists idx_logistics_prices_tenant_carrier
  on logistics_prices(tenant_id, carrier_code);

create table if not exists order_tag_definitions (
  tag_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  color text,
  priority int not null default 0,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_order_tag_defs_tenant_time
  on order_tag_definitions(tenant_id, updated_at desc);
create index if not exists idx_order_tag_defs_tenant_status
  on order_tag_definitions(tenant_id, status);

create table if not exists sku_tag_definitions (
  tag_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  color text,
  priority int not null default 0,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_sku_tag_defs_tenant_time
  on sku_tag_definitions(tenant_id, updated_at desc);
create index if not exists idx_sku_tag_defs_tenant_status
  on sku_tag_definitions(tenant_id, status);
