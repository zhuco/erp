-- 0069: Shipment package tracking

create table if not exists shipment_packages (
  package_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shipment_id uuid not null references shipments(shipment_id) on delete cascade,
  package_no text not null,
  carrier_code text,
  tracking_no text,
  weight_gram int,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, shipment_id, package_no)
);

create index if not exists idx_shipment_packages_tenant_shipment
  on shipment_packages(tenant_id, shipment_id);
create index if not exists idx_shipment_packages_tenant_tracking
  on shipment_packages(tenant_id, tracking_no);
