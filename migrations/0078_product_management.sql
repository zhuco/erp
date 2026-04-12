-- 0078: Product categories/brands/units + warehouse mappings

alter table products
  add column if not exists product_code text,
  add column if not exists alias text,
  add column if not exists base_unit text,
  add column if not exists second_unit text,
  add column if not exists default_supplier_id uuid references suppliers(supplier_id),
  add column if not exists default_warehouse_id uuid references warehouses(warehouse_id),
  add column if not exists primary_barcode text,
  add column if not exists product_tags text[] not null default '{}'::text[],
  add column if not exists remark text,
  add column if not exists warning_stock jsonb not null default '{}'::jsonb,
  add column if not exists batch_enabled boolean not null default false,
  add column if not exists shelf_life_days int,
  add column if not exists forbid_sale_days int,
  add column if not exists serial_enabled boolean not null default false,
  add column if not exists serial_push_platform boolean not null default false;

create unique index if not exists idx_products_tenant_code
  on products(tenant_id, product_code)
  where product_code is not null;

create index if not exists idx_products_tenant_primary_barcode
  on products(tenant_id, primary_barcode);

create table if not exists product_categories (
  category_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_product_categories_tenant_name
  on product_categories(tenant_id, name);

create table if not exists product_brands (
  brand_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_product_brands_tenant_name
  on product_brands(tenant_id, name);

create table if not exists product_units (
  unit_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_product_units_tenant_name
  on product_units(tenant_id, name);

create table if not exists product_warehouses (
  tenant_id uuid not null references tenants(tenant_id),
  product_id uuid not null references products(product_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  is_default boolean not null default false,
  allow_negative_stock boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, product_id, warehouse_id)
);

create index if not exists idx_product_warehouses_product
  on product_warehouses(tenant_id, product_id);

create index if not exists idx_product_warehouses_warehouse
  on product_warehouses(tenant_id, warehouse_id);
