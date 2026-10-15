-- A product identity stays when SKUs are added.
-- A trade kit is not the manufacturing flag.

create table if not exists item_products (
  tenant_id uuid not null,
  product_id uuid not null,
  product_code text not null,
  item_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, product_id),
  unique (tenant_id, product_code)
);

create table if not exists item_skus (
  tenant_id uuid not null,
  sku_id uuid not null,
  product_id uuid not null,
  sku_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, sku_id),
  unique (tenant_id, sku_code)
);

create table if not exists item_kit_flags (
  tenant_id uuid not null,
  item_id uuid not null,
  kit boolean not null,
  make boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, item_id)
);
