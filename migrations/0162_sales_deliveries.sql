-- A delivery plan cites one sales line. Review moves ownership once.
-- Issue and shipment do not deduct stock again.

create table if not exists shared_sales_deliveries (
  tenant_id uuid not null,
  delivery_id uuid not null,
  so_id uuid not null,
  line_no integer not null,
  order_version integer not null,
  qty_micros bigint not null,
  stage text not null,
  carrier_ref text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, delivery_id)
);

create table if not exists shared_sales_cancellations (
  tenant_id uuid not null,
  cancel_id uuid not null,
  so_id uuid not null,
  line_no integer not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, cancel_id)
);
