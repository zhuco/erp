-- 0115: co-product cost shares, disassembly recoveries, and backflush.
-- A recovery cites the original output and does not delete it.
-- Backflush is a consume move and must not repeat a manual consume.

create table if not exists mfg_co_product_splits (
  tenant_id uuid not null references tenants(tenant_id),
  split_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  total_cent bigint not null,
  primary_code text not null,
  primary_weight bigint not null,
  primary_cent bigint not null,
  co_code text not null,
  co_weight bigint not null,
  co_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, split_id)
);

create table if not exists mfg_disassembly_recoveries (
  tenant_id uuid not null references tenants(tenant_id),
  recovery_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  source_output text not null,
  component_code text not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, recovery_id)
);
