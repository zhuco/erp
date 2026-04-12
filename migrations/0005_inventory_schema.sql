-- 0005: Inventory schema (warehouses + balances + ledgers + reservations)

create table if not exists warehouses (
  warehouse_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  code text not null,
  is_default boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, code)
);

create table if not exists inventory_balances (
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  on_hand bigint not null default 0,
  reserved bigint not null default 0,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, warehouse_id, sku_id)
);

create table if not exists inventory_ledgers (
  ledger_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  kind text not null,
  qty bigint not null,
  ref_type text,
  ref_id text,
  created_at timestamptz not null default now()
);

create index if not exists idx_inventory_ledgers_sku_time
  on inventory_ledgers(tenant_id, sku_id, created_at desc);
create index if not exists idx_inventory_ledgers_warehouse_time
  on inventory_ledgers(tenant_id, warehouse_id, created_at desc);

create table if not exists inventory_reservations (
  reservation_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  order_id uuid,
  order_item_id uuid,
  qty bigint not null,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, order_item_id)
);
