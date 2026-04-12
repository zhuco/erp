-- 0021: WMS inbound docs
create table if not exists inbound_docs (
  doc_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null,
  warehouse_id uuid not null references warehouses(warehouse_id),
  doc_type text not null,
  doc_no text not null,
  status text not null default 'DRAFT',
  ref_type text,
  ref_id text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, doc_no)
);

create index if not exists idx_inbound_docs_tenant_created
  on inbound_docs(tenant_id, created_at desc);
create index if not exists idx_inbound_docs_tenant_status
  on inbound_docs(tenant_id, status, created_at desc);
create index if not exists idx_inbound_docs_tenant_warehouse
  on inbound_docs(tenant_id, warehouse_id, created_at desc);

create table if not exists inbound_doc_items (
  tenant_id uuid not null,
  doc_id uuid not null references inbound_docs(doc_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  planned_qty bigint not null default 0,
  received_qty bigint not null default 0,
  putaway_qty bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, doc_id, sku_id)
);

create index if not exists idx_inbound_doc_items_tenant_doc
  on inbound_doc_items(tenant_id, doc_id);
create index if not exists idx_inbound_doc_items_tenant_sku
  on inbound_doc_items(tenant_id, sku_id);

create table if not exists inbound_item_ops (
  tenant_id uuid not null,
  operation_id uuid not null,
  doc_id uuid not null references inbound_docs(doc_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  action text not null,
  qty bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, operation_id)
);

create index if not exists idx_inbound_item_ops_tenant_doc
  on inbound_item_ops(tenant_id, doc_id);

create table if not exists inbound_putaway_tasks (
  task_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null,
  doc_id uuid not null references inbound_docs(doc_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  from_area text,
  to_location_id uuid,
  status text not null default 'DONE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_inbound_putaway_tasks_tenant_doc
  on inbound_putaway_tasks(tenant_id, doc_id);
create index if not exists idx_inbound_putaway_tasks_tenant_status
  on inbound_putaway_tasks(tenant_id, status, created_at desc);
