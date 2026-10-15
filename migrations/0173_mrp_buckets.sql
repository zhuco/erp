-- Dated MRP keeps its inputs and one suggestion per item and need date.
-- An unaccepted suggestion is not stored as supply. This does not rewrite the older snapshot table.

create table if not exists shared_mrp_runs (
  tenant_id uuid not null,
  run_id uuid not null,
  warehouse_code text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id)
);

create table if not exists shared_mrp_inputs (
  tenant_id uuid not null,
  run_id uuid not null,
  kind text not null,
  item_code text not null,
  need_date text not null default '',
  qty_micros bigint not null,
  accepted boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id, kind, item_code, need_date)
);

create table if not exists shared_mrp_suggestions (
  tenant_id uuid not null,
  run_id uuid not null,
  item_code text not null,
  need_date text not null,
  gross_micros bigint not null,
  net_micros bigint not null,
  suggested_micros bigint not null,
  supply_kind text not null,
  release_date text not null,
  explanation text not null,
  accepted boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id, item_code, need_date)
);
