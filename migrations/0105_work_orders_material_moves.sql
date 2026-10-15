-- 0105: work orders and material issue, return, and consumption.
-- Issued facts stay after cancel is refused. These rows are not kit BOM lines.

create table if not exists mfg_work_orders (
  tenant_id uuid not null references tenants(tenant_id),
  work_order_id uuid not null default gen_random_uuid(),
  work_order_no text not null,
  product_code text not null,
  bom_id uuid,
  status text not null,
  order_qty_micros bigint not null,
  issued_micros bigint not null default 0,
  consumed_micros bigint not null default 0,
  created_at timestamptz not null default now(),
  primary key (tenant_id, work_order_id),
  unique (tenant_id, work_order_no),
  constraint mfg_work_orders_status_check
    check (status in ('PLANNED', 'RELEASED', 'PAUSED', 'COMPLETED', 'CLOSED', 'CANCELLED'))
);

create table if not exists mfg_material_moves (
  tenant_id uuid not null references tenants(tenant_id),
  move_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  kind text not null,
  qty_micros bigint not null,
  reason text,
  created_at timestamptz not null default now(),
  primary key (tenant_id, move_id),
  constraint mfg_material_moves_kind_check
    check (kind in ('ISSUE', 'SUPPLEMENT', 'RETURN', 'CONSUME'))
);

create index if not exists idx_mfg_material_moves_order
  on mfg_material_moves(tenant_id, work_order_id);
