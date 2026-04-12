-- 0069: Shop product download, matching tasks, and mapping source

alter table channel_sku_mappings
  add column if not exists match_source text not null default 'AUTO';

create index if not exists idx_channel_sku_mappings_match_source
  on channel_sku_mappings(tenant_id, match_source);

create table if not exists shop_products (
  product_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  platform text not null,
  platform_product_id text not null,
  merchant_code text,
  title text not null,
  product_type text not null default 'STANDARD',
  status text not null default 'ACTIVE',
  platform_updated_at timestamptz,
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, shop_id, platform, platform_product_id)
);

create index if not exists idx_shop_products_tenant_shop
  on shop_products(tenant_id, shop_id, platform);
create index if not exists idx_shop_products_tenant_updated
  on shop_products(tenant_id, platform_updated_at desc);

create table if not exists shop_product_skus (
  sku_record_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  platform text not null,
  platform_product_id text not null,
  platform_sku_id text not null,
  merchant_sku_code text,
  title text,
  spec_text text,
  price_cent bigint,
  stock_qty bigint,
  status text not null default 'ACTIVE',
  barcode text,
  image_url text,
  platform_updated_at timestamptz,
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, shop_id, platform, platform_sku_id)
);

create index if not exists idx_shop_product_skus_platform_product
  on shop_product_skus(tenant_id, platform_product_id);
create index if not exists idx_shop_product_skus_tenant_status
  on shop_product_skus(tenant_id, status);

create table if not exists shop_product_tasks (
  task_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  task_type text not null,
  status text not null default 'PENDING',
  shop_id uuid references shops(shop_id),
  platform text,
  total_count int not null default 0,
  success_count int not null default 0,
  failure_count int not null default 0,
  error text not null default '',
  payload_json jsonb not null default '{}'::jsonb,
  result_json jsonb not null default '{}'::jsonb,
  started_at timestamptz not null default now(),
  finished_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_shop_product_tasks_tenant_time
  on shop_product_tasks(tenant_id, created_at desc);
create index if not exists idx_shop_product_tasks_tenant_status
  on shop_product_tasks(tenant_id, status, created_at desc);
