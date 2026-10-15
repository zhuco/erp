-- 0111: manufacturing cost runs. A later estimate does not overwrite a confirmed row.
-- Unfinished and defect amounts are stored beside the finished unit cost.
-- A late fee is a new adjustment row, not an update of the confirmed run.

create table if not exists mfg_cost_runs (
  tenant_id uuid not null references tenants(tenant_id),
  cost_run_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  material_cent bigint not null,
  labor_cent bigint not null,
  overhead_cent bigint,
  total_cent bigint not null,
  unit_cent bigint not null,
  completed_qty_micros bigint not null,
  estimated boolean not null,
  unfinished_cent bigint not null default 0,
  defect_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  primary key (tenant_id, cost_run_id)
);

alter table mfg_cost_runs add column if not exists unfinished_cent bigint not null default 0;
alter table mfg_cost_runs add column if not exists defect_cent bigint not null default 0;

create table if not exists mfg_cost_adjustments (
  tenant_id uuid not null references tenants(tenant_id),
  adjustment_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  fee_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, adjustment_id)
);
