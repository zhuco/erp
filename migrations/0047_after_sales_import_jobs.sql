-- 0047: after-sales import jobs + raw data support

alter table after_sales
  add column if not exists raw_json jsonb not null default '{}'::jsonb;

create table if not exists after_sales_import_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  file_name text,
  file_path text not null,
  file_type text not null default 'CSV',
  status text not null default 'PENDING',
  progress int not null default 0,
  total_rows int not null default 0,
  success_rows int not null default 0,
  failed_rows int not null default 0,
  attempts int not null default 0,
  last_error text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_after_sales_import_jobs_tenant_status
  on after_sales_import_jobs(tenant_id, status, created_at desc);
