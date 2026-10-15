-- 0104: manufacturing routing, execution, inspection, and cost.
-- These records are separate from kit BOM and inventory balances.

create table if not exists mfg_routings (
  tenant_id uuid not null references tenants(tenant_id),
  routing_id uuid not null default gen_random_uuid(),
  product_code text not null,
  operation_code text not null,
  sequence_no integer not null,
  work_center text not null,
  minutes_per bigint not null,
  inspection_required boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, routing_id),
  unique (tenant_id, product_code, operation_code)
);

create table if not exists mfg_executions (
  tenant_id uuid not null references tenants(tenant_id),
  execution_id uuid not null default gen_random_uuid(),
  work_order_no text not null,
  issued_micros bigint not null,
  consumed_micros bigint not null,
  good_micros bigint not null,
  rejected_micros bigint not null,
  received_micros bigint not null,
  wip_micros bigint not null,
  available_micros bigint not null,
  cost_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, execution_id),
  unique (tenant_id, work_order_no)
);

create table if not exists mfg_inspections (
  tenant_id uuid not null references tenants(tenant_id),
  inspection_id uuid not null default gen_random_uuid(),
  execution_id uuid not null,
  result text not null,
  good_micros bigint not null,
  rejected_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, inspection_id),
  constraint mfg_inspections_result_check
    check (result in ('HOLD', 'RELEASED', 'REJECTED'))
);

create table if not exists mfg_cost_entries (
  tenant_id uuid not null references tenants(tenant_id),
  cost_id uuid not null default gen_random_uuid(),
  execution_id uuid not null,
  material_cost_cent bigint not null,
  labor_minutes bigint not null,
  labor_rate_cent bigint not null,
  total_cost_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, cost_id)
);
