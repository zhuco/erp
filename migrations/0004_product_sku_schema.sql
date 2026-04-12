-- 0004: Product + SKU master data and channel mappings

create table if not exists products (
  product_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  brand text,
  category text,
  attrs jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists skus (
  sku_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  product_id uuid not null references products(product_id),
  sku_code text not null,
  barcode text,
  name text,
  spec jsonb not null default '{}'::jsonb,
  weight_gram int,
  length_mm int,
  width_mm int,
  height_mm int,
  is_active boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, sku_code)
);

create index if not exists idx_skus_tenant_barcode
  on skus(tenant_id, barcode);

create table if not exists channel_sku_mappings (
  mapping_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  platform text not null,
  platform_sku_id text not null,
  platform_outer_sku_id text,
  sku_id uuid not null references skus(sku_id),
  status text not null default 'ACTIVE',
  confidence int not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, shop_id, platform, platform_sku_id)
);

create index if not exists idx_channel_sku_mappings_sku
  on channel_sku_mappings(tenant_id, sku_id);
create index if not exists idx_channel_sku_mappings_shop_platform
  on channel_sku_mappings(tenant_id, shop_id, platform);
