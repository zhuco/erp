-- 0038: Reports + export jobs + report indexes

create table if not exists export_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  job_type text not null,
  params_json jsonb not null default '{}'::jsonb,
  status text not null default 'PENDING',
  output_url text,
  progress int not null default 0,
  attempts int not null default 0,
  last_error text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_export_jobs_tenant_status
  on export_jobs(tenant_id, status, created_at desc);
create index if not exists idx_export_jobs_tenant_type
  on export_jobs(tenant_id, job_type);

create index if not exists idx_orders_tenant_created_at
  on orders(tenant_id, order_created_at desc);
create index if not exists idx_order_items_tenant_created_at
  on order_items(tenant_id, created_at desc);
create index if not exists idx_shipments_tenant_created_at
  on shipments(tenant_id, created_at desc);
create index if not exists idx_inventory_ledgers_tenant_created_at
  on inventory_ledgers(tenant_id, created_at desc);
