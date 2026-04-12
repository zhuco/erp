-- 0039: OMS resend + cancel request

alter table shipments
  add column if not exists origin_shipment_id uuid;

create index if not exists idx_shipments_tenant_origin
  on shipments(tenant_id, origin_shipment_id);

create table if not exists shipment_cancel_requests (
  cancel_request_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shipment_id uuid not null references shipments(shipment_id),
  order_id uuid not null,
  status text not null default 'PENDING',
  reason text,
  requested_by uuid,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists idx_shipment_cancel_requests_unique
  on shipment_cancel_requests(tenant_id, shipment_id);
create index if not exists idx_shipment_cancel_requests_tenant_status
  on shipment_cancel_requests(tenant_id, status, created_at desc);
