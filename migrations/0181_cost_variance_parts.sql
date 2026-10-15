-- Frozen standard drivers and drillable variance parts. A rerun keeps the previous version.
create table if not exists mfg_standard_cost_drivers (
  tenant_id uuid not null,
  standard_id uuid not null,
  work_order_id uuid not null,
  version_no integer not null,
  material_qty_micros bigint not null,
  material_price_cent bigint not null,
  labor_minutes bigint not null,
  labor_rate_cent bigint not null,
  overhead_rate_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, standard_id),
  unique (tenant_id, work_order_id, version_no)
);

create table if not exists mfg_cost_variance_parts (
  tenant_id uuid not null,
  variance_id uuid not null,
  work_order_id uuid not null,
  standard_id uuid not null,
  version_no integer not null,
  kind text not null,
  standard_cent bigint not null,
  actual_cent bigint not null,
  variance_cent bigint not null,
  estimated boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, variance_id, kind)
);
