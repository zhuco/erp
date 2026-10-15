-- Formula and actual charge stay separate. Yield does not rewrite the formula.
-- These rows cite an existing work order; they are not a second inventory engine.

create table if not exists process_formulas (
  tenant_id uuid not null,
  formula_id uuid not null,
  work_order_id uuid not null,
  component_code text not null,
  mass_micros bigint not null,
  density_micros bigint not null,
  basis_version text not null,
  volume_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, formula_id)
);

create table if not exists process_charges (
  tenant_id uuid not null,
  charge_id uuid not null,
  formula_id uuid not null,
  work_order_id uuid not null,
  charged_micros bigint not null,
  product_micros bigint not null,
  waste_micros bigint not null,
  co_micros bigint not null,
  unexplained_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, charge_id)
);
