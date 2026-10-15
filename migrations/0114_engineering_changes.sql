-- 0114: substitutes and engineering changes. Historical BOM lines are not rewritten.
-- A change applies only to a planned work order that has not issued material.

create table if not exists mfg_substitutes (
  tenant_id uuid not null references tenants(tenant_id),
  substitute_id uuid not null default gen_random_uuid(),
  original_code text not null,
  replacement_code text not null,
  ratio_micros bigint not null,
  effective_from text not null,
  effective_to text not null,
  approver text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, substitute_id)
);

alter table mfg_routings add column if not exists version_no integer not null default 1;

create table if not exists mfg_routing_versions (
  tenant_id uuid not null references tenants(tenant_id),
  version_id uuid not null default gen_random_uuid(),
  product_code text not null,
  version_no integer not null,
  operation_code text not null,
  sequence_no integer not null,
  work_center text not null,
  minutes_per bigint not null,
  inspection_required boolean not null,
  approver text not null,
  change_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, version_id)
);

create table if not exists mfg_routing_diffs (
  tenant_id uuid not null references tenants(tenant_id),
  change_id uuid not null,
  product_code text not null,
  version_no integer not null,
  operation_code text not null,
  change_kind text not null,
  old_minutes bigint not null,
  new_minutes bigint not null,
  old_work_center text not null,
  new_work_center text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, change_id, operation_code)
);

create table if not exists mfg_routing_impacts (
  tenant_id uuid not null references tenants(tenant_id),
  change_id uuid not null,
  work_order_id uuid not null,
  status text not null,
  issued_micros bigint not null,
  consumed_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, change_id, work_order_id)
);

create table if not exists mfg_engineering_changes (
  tenant_id uuid not null references tenants(tenant_id),
  change_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  bom_id uuid not null,
  substitute_id uuid not null,
  original_code text not null,
  replacement_code text not null,
  old_qty_micros bigint not null,
  new_qty_micros bigint not null,
  need_date text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, change_id)
);
