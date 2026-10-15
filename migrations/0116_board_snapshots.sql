-- 0116: production board snapshots. Quantities come from submitted facts.
-- An estimated date is not a promised date. Missing inputs stay unknown.

create table if not exists mfg_board_snapshots (
  tenant_id uuid not null references tenants(tenant_id),
  snapshot_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  plan_qty_micros bigint not null,
  issued_micros bigint not null,
  consumed_micros bigint not null,
  good_micros bigint not null,
  rejected_micros bigint not null,
  received_micros bigint not null,
  shortage_micros bigint not null,
  load_minutes bigint not null,
  due_date text,
  estimate_status text not null,
  estimated_date text,
  late boolean,
  basis text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, snapshot_id)
);
