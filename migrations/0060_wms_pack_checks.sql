-- 0051: WMS outbound check/weigh events

create table if not exists pack_check_events (
  check_event_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
  shipment_id uuid references shipments(shipment_id),
  package_no text not null,
  event_type text not null,
  result text not null,
  weight_gram int,
  reason text,
  operator_id uuid,
  created_at timestamptz not null default now()
);

create index if not exists idx_pack_check_events_tenant_doc
  on pack_check_events(tenant_id, outbound_doc_id, created_at desc);
create index if not exists idx_pack_check_events_tenant_type
  on pack_check_events(tenant_id, event_type, created_at desc);
