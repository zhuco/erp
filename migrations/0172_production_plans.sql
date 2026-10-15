-- A production plan cites the source order and line. A resale bucket is not production.
-- Cancelling open quantity does not erase executed quantity. The same source is not imported twice.

create table if not exists shared_production_plans (
  tenant_id uuid not null,
  plan_id uuid not null,
  source_id uuid not null,
  line_no integer not null,
  item_code text not null,
  version_no integer not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, plan_id),
  unique (tenant_id, source_id, line_no)
);

create table if not exists shared_production_buckets (
  tenant_id uuid not null,
  plan_id uuid not null,
  due_date text not null,
  qty_micros bigint not null,
  produce boolean not null,
  executed_micros bigint not null default 0,
  cancelled_micros bigint not null default 0,
  created_at timestamptz not null default now(),
  primary key (tenant_id, plan_id, due_date)
);
