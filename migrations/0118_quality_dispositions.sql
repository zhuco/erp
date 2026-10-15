-- 0118: quality holds and dispositions. A label change is not enough.
-- Each disposition posts a quantity ledger and keeps the remaining held quantity.

create table if not exists mfg_quality_holds (
  tenant_id uuid not null references tenants(tenant_id),
  hold_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  batch_code text not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, hold_id)
);

create table if not exists mfg_quality_dispositions (
  tenant_id uuid not null references tenants(tenant_id),
  disposition_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  batch_code text not null,
  warehouse_id uuid not null,
  sku_id uuid not null,
  kind text not null,
  qty_micros bigint not null,
  reason text not null,
  authorizer text not null,
  held_left_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, disposition_id)
);
