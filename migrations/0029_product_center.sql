-- 0029: Product center enhancements (products, BOM trace, batches, serials)

-- Products: add common list index
create index if not exists idx_products_tenant_created
  on products(tenant_id, created_at desc);

-- SKUs: add batch/serial flags + volume/status
alter table skus
  add column if not exists volume_cm3 int,
  add column if not exists status text not null default 'ACTIVE',
  add column if not exists batch_enabled boolean not null default false,
  add column if not exists serial_enabled boolean not null default false;

update skus
set status = case when is_active then 'ACTIVE' else 'INACTIVE' end
where status is null;

update skus
set volume_cm3 = (length_mm * width_mm * height_mm) / 1000
where volume_cm3 is null
  and length_mm is not null
  and width_mm is not null
  and height_mm is not null;

create index if not exists idx_skus_tenant_created
  on skus(tenant_id, created_at desc);

-- BOM table rename + versioning
alter table if exists kit_boms rename to sku_boms;
alter table if exists sku_boms rename column kit_sku_id to bundle_sku_id;
alter table if exists sku_boms
  add column if not exists bom_version int not null default 1,
  add column if not exists updated_at timestamptz not null default now();

create index if not exists idx_sku_boms_bundle
  on sku_boms(tenant_id, bundle_sku_id);

-- Order item BOM expansion trace
create table if not exists order_item_components (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  order_item_id uuid not null,
  bundle_sku_id uuid not null references skus(sku_id),
  component_sku_id uuid not null references skus(sku_id),
  component_qty bigint not null,
  bom_version int,
  bom_snapshot jsonb not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, order_item_id, component_sku_id)
);

create index if not exists idx_order_item_components_order
  on order_item_components(tenant_id, order_id);

-- SKU batches + inventory by batch
create table if not exists sku_batches (
  batch_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  batch_no text not null,
  expire_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, sku_id, batch_no)
);

create index if not exists idx_sku_batches_tenant_sku_expire
  on sku_batches(tenant_id, sku_id, expire_at);

create table if not exists inventory_batch_balances (
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  batch_id uuid not null references sku_batches(batch_id),
  on_hand bigint not null default 0,
  reserved bigint not null default 0,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, warehouse_id, sku_id, batch_id)
);

create index if not exists idx_inventory_batch_sku
  on inventory_batch_balances(tenant_id, sku_id, batch_id);

-- SKU serials + shipment serial scans
create table if not exists sku_serials (
  serial_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  serial_no text not null,
  status text not null default 'IN_STOCK',
  ref_doc_type text,
  ref_doc_id text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, serial_no)
);

create index if not exists idx_sku_serials_status
  on sku_serials(tenant_id, sku_id, status);

create table if not exists sku_serial_sequences (
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  prefix text not null,
  date_key text not null,
  last_seq bigint not null default 0,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, sku_id, prefix, date_key)
);

create table if not exists shipment_item_serials (
  tenant_id uuid not null references tenants(tenant_id),
  shipment_id uuid not null references shipments(shipment_id),
  order_id uuid not null,
  order_item_id uuid not null,
  sku_id uuid not null references skus(sku_id),
  serial_id uuid not null references sku_serials(serial_id),
  serial_no text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, shipment_id, serial_id)
);

create index if not exists idx_shipment_item_serials_order
  on shipment_item_serials(tenant_id, order_id, sku_id);
