-- 0107: manufacturing shortage reservations in millionths of a base unit.
-- These rows are not order reservations and do not rewrite legacy integer balances.

create table if not exists mfg_reservations (
  tenant_id uuid not null references tenants(tenant_id),
  reservation_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  warehouse_id uuid not null,
  sku_id uuid not null,
  reserved_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, reservation_id)
);

create table if not exists mfg_qty_balances (
  tenant_id uuid not null,
  warehouse_id uuid not null,
  sku_id uuid not null,
  on_hand_micros bigint not null default 0,
  reserved_micros bigint not null default 0,
  primary key (tenant_id, warehouse_id, sku_id)
);
