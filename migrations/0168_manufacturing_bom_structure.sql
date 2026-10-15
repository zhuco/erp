-- Manufacturing BOM structure is not a trade kit. Scrap is stored once.
-- A published version is not edited in place. Query order is not line order.

create table if not exists shared_mfg_bom_lines (
  tenant_id uuid not null,
  bom_id uuid not null,
  line_no integer not null,
  parent_code text not null,
  component_code text not null,
  qty_per_micros bigint not null,
  unit_code text not null,
  scrap_ppm bigint not null,
  supply_kind text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, bom_id, line_no)
);

create table if not exists shared_mfg_bom_explodes (
  tenant_id uuid not null,
  explode_id uuid not null,
  bom_id uuid not null,
  parent_code text not null,
  component_code text not null,
  unit_code text not null,
  qty_micros bigint not null,
  line_no integer not null,
  depth integer not null,
  seq_no integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, explode_id, seq_no)
);
