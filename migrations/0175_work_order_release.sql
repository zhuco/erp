-- A released order freezes its BOM version and keeps one reservation.
-- Cancelling an executed order is refused. The order row is not deleted.

create table if not exists shared_work_order_releases (
  tenant_id uuid not null,
  work_order_id uuid not null,
  version_id uuid not null,
  version_no integer not null,
  warehouse_id uuid not null,
  sku_id uuid not null,
  reserved_micros bigint not null,
  status text not null,
  shortage_reason text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, work_order_id)
);
