-- 0112: shortage, scrap, and rework stay attached to the original work order.
-- Scrap stores quantity and a cost destination. Close does not delete the order.

create table if not exists mfg_exceptions (
  tenant_id uuid not null references tenants(tenant_id),
  exception_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  kind text not null,
  qty_micros bigint not null,
  reason text not null,
  cost_cent bigint not null default 0,
  source_work_order_id uuid,
  added_minutes bigint not null default 0,
  posted boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, exception_id)
);
