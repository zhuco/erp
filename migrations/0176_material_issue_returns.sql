-- An issue moves stock into WIP. A return cites that issue and moves it back.
-- Consume reduces shop stock and does not restore warehouse stock. A retry does not post twice.

create table if not exists shared_material_issues (
  tenant_id uuid not null,
  issue_id uuid not null,
  work_order_id uuid not null,
  warehouse_id uuid not null,
  location_id uuid not null,
  sku_id uuid not null,
  batch_code text not null,
  qty_micros bigint not null,
  returned_micros bigint not null default 0,
  kind text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, issue_id)
);

create table if not exists shared_material_returns (
  tenant_id uuid not null,
  return_id uuid not null,
  issue_id uuid not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, return_id)
);
