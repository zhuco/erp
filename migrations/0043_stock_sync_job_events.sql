-- 0043: Stock sync job retries + events

alter table stock_sync_jobs
  add column if not exists next_retry_at timestamptz not null default now();

create index if not exists idx_stock_sync_jobs_due
  on stock_sync_jobs(status, next_retry_at, updated_at desc);

create table if not exists stock_sync_job_events (
  event_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  job_id uuid not null references stock_sync_jobs(job_id),
  shop_id uuid not null references shops(shop_id),
  sku_id uuid not null references skus(sku_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  status text not null,
  attempt int not null default 0,
  payload jsonb not null,
  created_at timestamptz not null default now()
);

create index if not exists idx_stock_sync_job_events_tenant_job
  on stock_sync_job_events(tenant_id, job_id, created_at desc);
