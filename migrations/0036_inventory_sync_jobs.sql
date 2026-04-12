-- 0036: Inventory ledger deltas + stock sync jobs

alter table inventory_balances
  add column if not exists in_transit bigint not null default 0;

alter table inventory_ledgers
  add column if not exists delta_on_hand bigint not null default 0;
alter table inventory_ledgers
  add column if not exists delta_reserved bigint not null default 0;
alter table inventory_ledgers
  add column if not exists delta_in_transit bigint not null default 0;
alter table inventory_ledgers
  alter column kind drop not null;
alter table inventory_ledgers
  alter column qty drop not null;
alter table inventory_ledgers
  add column if not exists reason_type text;
alter table inventory_ledgers
  add column if not exists occurred_at timestamptz not null default now();

update inventory_ledgers
set delta_on_hand = qty,
    reason_type = coalesce(reason_type, kind),
    occurred_at = coalesce(occurred_at, created_at)
where delta_on_hand = 0 and qty <> 0;

create index if not exists idx_inventory_ledgers_tenant_time
  on inventory_ledgers(tenant_id, occurred_at desc);
create index if not exists idx_inventory_ledgers_sku_time
  on inventory_ledgers(tenant_id, sku_id, occurred_at desc);
create index if not exists idx_inventory_ledgers_warehouse_time
  on inventory_ledgers(tenant_id, warehouse_id, occurred_at desc);

create table if not exists stock_sync_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  sku_id uuid not null references skus(sku_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  target_qty bigint not null,
  status text not null default 'PENDING',
  attempts int not null default 0,
  last_error text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_stock_sync_jobs_tenant_status
  on stock_sync_jobs(tenant_id, status, created_at desc);
create index if not exists idx_stock_sync_jobs_tenant_sku
  on stock_sync_jobs(tenant_id, sku_id, warehouse_id);
create unique index if not exists idx_stock_sync_jobs_pending
  on stock_sync_jobs(tenant_id, shop_id, sku_id, warehouse_id)
  where status = 'PENDING';
