-- A published BOM version is not edited in place. A work order keeps that version.
-- Retiring a version does not delete its lines. Default ranges must not overlap.

create table if not exists shared_mfg_bom_versions (
  tenant_id uuid not null,
  version_id uuid not null,
  series_code text not null,
  version_no integer not null,
  status text not null,
  start_date text not null,
  end_date text not null default '',
  is_default boolean not null default false,
  source_bom_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, version_id),
  unique (tenant_id, series_code, version_no)
);

create table if not exists shared_mfg_bom_version_lines (
  tenant_id uuid not null,
  version_id uuid not null,
  line_no integer not null,
  parent_code text not null,
  component_code text not null,
  qty_per_micros bigint not null,
  unit_code text not null,
  scrap_ppm bigint not null,
  supply_kind text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, version_id, line_no)
);

create table if not exists shared_mfg_work_order_boms (
  tenant_id uuid not null,
  work_order_id uuid not null,
  version_id uuid not null,
  version_no integer not null,
  status_at_snapshot text not null,
  component_code text not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, work_order_id, component_code)
);
