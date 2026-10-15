-- Standard snapshot and actual variance lines stay in separate rows. A rerun keeps the old version.
create table if not exists mfg_standard_costs (
  tenant_id uuid not null,
  standard_id uuid not null,
  work_order_id uuid not null,
  version_no integer not null,
  material_cent bigint not null,
  labor_cent bigint not null,
  overhead_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, standard_id)
);

create table if not exists mfg_cost_variances (
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
