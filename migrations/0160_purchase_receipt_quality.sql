-- Arrived stock stays held until release. A supplier return cites the original receipt and its cost.

create table if not exists shared_receipt_quality (
  tenant_id uuid not null,
  quality_id uuid not null,
  receipt_id uuid not null,
  po_id uuid not null,
  line_no integer not null,
  warehouse_id uuid not null,
  location_id uuid not null,
  sku_id uuid not null,
  supplier_code text not null,
  batch_code text not null,
  arrived_micros bigint not null,
  released_micros bigint not null default 0,
  issued_micros bigint not null default 0,
  unit_cost_cent bigint not null,
  payable_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  primary key (tenant_id, quality_id)
);

create table if not exists shared_supplier_returns (
  tenant_id uuid not null,
  return_id uuid not null,
  receipt_id uuid not null,
  qty_micros bigint not null,
  cost_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, return_id)
);
