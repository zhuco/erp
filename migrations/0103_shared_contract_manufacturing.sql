-- 0103: shared command receipts and manufacturing snapshots.
-- Kit BOM tables are not reused. Suggestions do not change inventory balances.

create table if not exists command_receipts (
  tenant_id uuid not null references tenants(tenant_id),
  request_id text not null,
  payload_hash text not null,
  status text not null,
  effect_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, request_id),
  constraint command_receipts_status_check
    check (status in ('ACCEPTED', 'RUNNING', 'SUCCEEDED', 'FAILED', 'RECONCILING'))
);

create table if not exists mfg_boms (
  tenant_id uuid not null references tenants(tenant_id),
  bom_id uuid not null default gen_random_uuid(),
  product_code text not null,
  status text not null default 'DRAFT',
  version integer not null default 1,
  base_qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, bom_id),
  unique (tenant_id, product_code, version),
  constraint mfg_boms_status_check
    check (status in ('DRAFT', 'PUBLISHED', 'RETIRED'))
);

create table if not exists mfg_bom_items (
  tenant_id uuid not null references tenants(tenant_id),
  bom_id uuid not null,
  line_no integer not null,
  component_code text not null,
  qty_per_micros bigint not null,
  primary key (tenant_id, bom_id, line_no)
);

create index if not exists idx_mfg_bom_items_component
  on mfg_bom_items(tenant_id, component_code);

create table if not exists mfg_mrp_runs (
  tenant_id uuid not null references tenants(tenant_id),
  run_id uuid not null default gen_random_uuid(),
  algorithm text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id),
  constraint mfg_mrp_runs_status_check
    check (status in ('QUEUED', 'RUNNING', 'SUCCEEDED', 'FAILED', 'CANCELLED'))
);

create table if not exists mfg_mrp_results (
  tenant_id uuid not null references tenants(tenant_id),
  run_id uuid not null,
  item_code text not null,
  gross_micros bigint not null,
  net_micros bigint not null,
  explanation text not null,
  primary key (tenant_id, run_id, item_code)
);
