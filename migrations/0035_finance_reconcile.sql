-- 0035: Finance reconciliation + invoices + profit snapshots

create table if not exists platform_statements (
  statement_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null,
  shop_id uuid not null references shops(shop_id),
  period_start date not null,
  period_end date not null,
  status text not null default 'IMPORTED',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, platform, shop_id, period_start, period_end)
);

create index if not exists idx_platform_statements_tenant_status
  on platform_statements(tenant_id, status, created_at desc);
create index if not exists idx_platform_statements_tenant_period
  on platform_statements(tenant_id, period_start desc, period_end desc);

create table if not exists platform_statement_lines (
  tenant_id uuid not null references tenants(tenant_id),
  statement_id uuid not null references platform_statements(statement_id) on delete cascade,
  platform_order_id text not null,
  gross_cent bigint not null default 0,
  fee_cent bigint not null default 0,
  refund_cent bigint not null default 0,
  net_cent bigint not null default 0,
  occurred_at timestamptz,
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  primary key (tenant_id, statement_id, platform_order_id)
);

create index if not exists idx_platform_statement_lines_order
  on platform_statement_lines(tenant_id, platform_order_id);
create index if not exists idx_platform_statement_lines_statement
  on platform_statement_lines(tenant_id, statement_id);

create table if not exists shipping_statement_lines (
  line_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  tracking_no text not null,
  fee_cent bigint not null default 0,
  weight numeric,
  occurred_at timestamptz,
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists idx_shipping_statement_lines_tracking
  on shipping_statement_lines(tenant_id, tracking_no);
create index if not exists idx_shipping_statement_lines_occurred
  on shipping_statement_lines(tenant_id, occurred_at desc);

create table if not exists reconcile_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  job_type text not null,
  status text not null,
  started_at timestamptz,
  finished_at timestamptz,
  last_error text,
  created_at timestamptz not null default now()
);

create index if not exists idx_reconcile_jobs_tenant_status
  on reconcile_jobs(tenant_id, status, created_at desc);
create index if not exists idx_reconcile_jobs_tenant_type
  on reconcile_jobs(tenant_id, job_type, created_at desc);

create table if not exists reconcile_job_items (
  tenant_id uuid not null references tenants(tenant_id),
  job_id uuid not null references reconcile_jobs(job_id) on delete cascade,
  ref_type text not null,
  ref_id text not null,
  expected_cent bigint not null default 0,
  actual_cent bigint not null default 0,
  diff_cent bigint not null default 0,
  diff_reason text,
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id, ref_type, ref_id)
);

create index if not exists idx_reconcile_job_items_job
  on reconcile_job_items(tenant_id, job_id);
create index if not exists idx_reconcile_job_items_diff
  on reconcile_job_items(tenant_id, diff_cent);

create table if not exists invoices (
  invoice_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  invoice_title text not null,
  tax_no text,
  status text not null default 'APPLIED',
  issued_at timestamptz,
  result_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, order_id)
);

create index if not exists idx_invoices_tenant_status
  on invoices(tenant_id, status, created_at desc);

create table if not exists profit_snapshots (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  revenue_cent bigint not null default 0,
  cost_cent bigint not null default 0,
  platform_fee_cent bigint not null default 0,
  shipping_fee_cent bigint not null default 0,
  gross_profit_cent bigint not null default 0,
  computed_at timestamptz not null default now(),
  primary key (tenant_id, order_id)
);

create index if not exists idx_profit_snapshots_tenant_time
  on profit_snapshots(tenant_id, computed_at desc);
