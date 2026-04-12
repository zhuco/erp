-- 0050: sync job runs for observability

create table if not exists sync_job_runs (
  run_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  job_id uuid not null references sync_jobs(job_id),
  shop_id uuid not null references shops(shop_id),
  job_type text not null,
  status text not null,
  item_count int not null default 0,
  duration_ms bigint not null default 0,
  error text not null default '',
  started_at timestamptz not null default now(),
  finished_at timestamptz not null default now()
);

create index if not exists idx_sync_job_runs_tenant_job
  on sync_job_runs(tenant_id, job_id, started_at desc);

create index if not exists idx_sync_job_runs_tenant_shop
  on sync_job_runs(tenant_id, shop_id, started_at desc);
