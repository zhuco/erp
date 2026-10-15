-- Auxiliary quantity converts to the six-digit base scale by a fixed ratio.
-- Display text is not a second quantity. Legacy whole units stay whole units.

create table if not exists unit_conversions (
  tenant_id uuid not null,
  conversion_id uuid not null,
  base_unit text not null,
  aux_unit text not null,
  ratio_micros bigint not null,
  rounding_code text not null,
  aux_micros bigint not null,
  base_micros bigint not null,
  back_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, conversion_id)
);

create table if not exists unit_qty_samples (
  tenant_id uuid not null,
  sample_id uuid not null,
  unit_code text not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, sample_id)
);

create table if not exists legacy_unit_reads (
  tenant_id uuid not null,
  read_id uuid not null,
  legacy_units bigint not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, read_id)
);
