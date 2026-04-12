-- 0007: Shipments + Waybills

create table if not exists shipments (
  shipment_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  order_id uuid not null,
  platform text not null,
  platform_order_id text not null,
  warehouse_id uuid references warehouses(warehouse_id),
  carrier_code text,
  tracking_no text,
  status text not null default 'REQUESTED',
  requested_at timestamptz,
  shipped_at timestamptz,
  updated_at timestamptz not null default now(),
  unique (tenant_id, order_id)
);

create index if not exists idx_shipments_tenant_shop_status
  on shipments(tenant_id, shop_id, status);
create index if not exists idx_shipments_tenant_platform_order
  on shipments(tenant_id, platform, platform_order_id);

create table if not exists waybills (
  waybill_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  platform text not null,
  carrier_code text not null,
  tracking_no text not null,
  source text not null default 'PLATFORM',
  status text not null default 'ALLOCATED',
  allocated_at timestamptz not null default now(),
  used_at timestamptz,
  unique (tenant_id, tracking_no)
);
