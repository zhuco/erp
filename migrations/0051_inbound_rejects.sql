-- 0049: inbound rejects
create table if not exists inbound_rejects (
  reject_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  doc_id uuid not null references inbound_docs(doc_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  reason text,
  operator_id uuid,
  created_at timestamptz not null default now()
);

create index if not exists idx_inbound_rejects_tenant_created
  on inbound_rejects(tenant_id, created_at desc);
create index if not exists idx_inbound_rejects_tenant_doc
  on inbound_rejects(tenant_id, doc_id);
create index if not exists idx_inbound_rejects_tenant_sku
  on inbound_rejects(tenant_id, sku_id);
