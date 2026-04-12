-- 0052: WMS inbound inspection records

create table if not exists inbound_inspections (
  inspection_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  doc_id uuid not null references inbound_docs(doc_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  result text not null,
  note text,
  operator_id uuid,
  operation_id uuid not null,
  created_at timestamptz not null default now()
);

create unique index if not exists idx_inbound_inspections_tenant_operation
  on inbound_inspections(tenant_id, operation_id);
create index if not exists idx_inbound_inspections_tenant_doc
  on inbound_inspections(tenant_id, doc_id, created_at desc);
