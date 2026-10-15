-- Operation WIP cites a frozen drawing and route.
-- Scrap and rework stay out of finished inventory.

create table if not exists operation_routes (
  tenant_id uuid not null,
  route_id uuid not null,
  work_order_id uuid not null,
  drawing_id text not null,
  route_version text not null,
  tool_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, route_id)
);

create table if not exists operation_wip (
  tenant_id uuid not null,
  wip_id uuid not null,
  work_order_id uuid not null,
  location_code text not null,
  owner_code text not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, wip_id)
);

create table if not exists operation_fees (
  tenant_id uuid not null,
  fee_id uuid not null,
  work_order_id uuid not null,
  source_location text not null,
  fee_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, fee_id)
);
