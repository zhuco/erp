-- 0031: WMS inventory operations (stock count/transfer/freeze/adjust/move)

alter table inventory_balances
  add column if not exists frozen bigint not null default 0;
alter table inventory_balances
  add column if not exists in_transit bigint not null default 0;

alter table inventory_ledgers
  add column if not exists delta_frozen bigint not null default 0;

create table if not exists warehouse_locations (
  location_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  code text not null,
  name text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, warehouse_id, code)
);

create index if not exists idx_warehouse_locations_tenant_warehouse
  on warehouse_locations(tenant_id, warehouse_id, code);

create table if not exists inventory_location_balances (
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  location_id uuid not null references warehouse_locations(location_id),
  sku_id uuid not null references skus(sku_id),
  on_hand bigint not null default 0,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, warehouse_id, location_id, sku_id)
);

create table if not exists stock_counts (
  count_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  status text not null default 'DRAFT',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_stock_counts_tenant_created
  on stock_counts(tenant_id, created_at desc, count_id desc);
create index if not exists idx_stock_counts_tenant_status
  on stock_counts(tenant_id, status);

create table if not exists stock_count_items (
  count_item_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  count_id uuid not null references stock_counts(count_id),
  sku_id uuid not null references skus(sku_id),
  location_id uuid references warehouse_locations(location_id),
  system_qty bigint not null default 0,
  counted_qty bigint not null default 0,
  diff_qty bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, count_id, sku_id, location_id)
);

create index if not exists idx_stock_count_items_tenant_count
  on stock_count_items(tenant_id, count_id);

create table if not exists stock_transfers (
  transfer_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  from_warehouse_id uuid not null references warehouses(warehouse_id),
  to_warehouse_id uuid not null references warehouses(warehouse_id),
  status text not null default 'DRAFT',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_stock_transfers_tenant_created
  on stock_transfers(tenant_id, created_at desc, transfer_id desc);
create index if not exists idx_stock_transfers_tenant_status
  on stock_transfers(tenant_id, status);

create table if not exists stock_transfer_items (
  transfer_item_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  transfer_id uuid not null references stock_transfers(transfer_id),
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, transfer_id, sku_id)
);

create index if not exists idx_stock_transfer_items_tenant_transfer
  on stock_transfer_items(tenant_id, transfer_id);

create table if not exists inventory_freezes (
  freeze_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  reason text,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_inventory_freezes_tenant_created
  on inventory_freezes(tenant_id, created_at desc, freeze_id desc);
create index if not exists idx_inventory_freezes_tenant_status
  on inventory_freezes(tenant_id, status);

create table if not exists stock_adjust_docs (
  adjust_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  reason text not null,
  status text not null default 'DRAFT',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_stock_adjust_docs_tenant_created
  on stock_adjust_docs(tenant_id, created_at desc, adjust_id desc);
create index if not exists idx_stock_adjust_docs_tenant_status
  on stock_adjust_docs(tenant_id, status);

create table if not exists stock_adjust_items (
  adjust_item_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  adjust_id uuid not null references stock_adjust_docs(adjust_id),
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, adjust_id, sku_id)
);

create index if not exists idx_stock_adjust_items_tenant_adjust
  on stock_adjust_items(tenant_id, adjust_id);

create table if not exists stock_move_docs (
  move_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  from_location_id uuid references warehouse_locations(location_id),
  to_location_id uuid references warehouse_locations(location_id),
  status text not null default 'DRAFT',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_stock_move_docs_tenant_created
  on stock_move_docs(tenant_id, created_at desc, move_id desc);
create index if not exists idx_stock_move_docs_tenant_status
  on stock_move_docs(tenant_id, status);

create table if not exists stock_move_items (
  move_item_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  move_id uuid not null references stock_move_docs(move_id),
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, move_id, sku_id)
);

create index if not exists idx_stock_move_items_tenant_move
  on stock_move_items(tenant_id, move_id);
