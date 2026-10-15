-- A conversion cites one suggestion line and one target line.
-- The converted total cannot exceed the suggestion. A retry does not add a second target.

create table if not exists shared_plan_conversions (
  tenant_id uuid not null,
  conversion_id uuid not null,
  run_id uuid not null,
  item_code text not null,
  need_date text not null,
  target_kind text not null,
  target_id uuid not null,
  target_line integer not null,
  qty_micros bigint not null,
  request_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, conversion_id),
  unique (tenant_id, request_id)
);
