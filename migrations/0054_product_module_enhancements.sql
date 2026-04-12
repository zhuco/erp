-- 0049: Product module enhancements (tags/images/suppliers/cost switch helpers)

alter table products
  add column if not exists status text not null default 'ACTIVE';

create index if not exists idx_products_tenant_status
  on products(tenant_id, status);

alter table skus
  add column if not exists tags text[] not null default '{}'::text[],
  add column if not exists image_urls text[] not null default '{}'::text[];

create index if not exists idx_skus_tags
  on skus using gin(tags);

alter table sku_boms
  add column if not exists expire_at timestamptz;

create table if not exists sku_suppliers (
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  supplier_id uuid not null references suppliers(supplier_id),
  is_primary boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, sku_id, supplier_id)
);

create index if not exists idx_sku_suppliers_supplier
  on sku_suppliers(tenant_id, supplier_id);
