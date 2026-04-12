-- 0032: WMS outbound/wave/pick/pack schemas

create table if not exists outbound_docs (
  outbound_doc_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  doc_type text not null,
  shipment_id uuid references shipments(shipment_id),
  doc_no text not null,
  status text not null default 'DRAFT',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists idx_outbound_docs_tenant_doc_no
  on outbound_docs(tenant_id, doc_no);
create unique index if not exists idx_outbound_docs_tenant_shipment
  on outbound_docs(tenant_id, shipment_id)
  where shipment_id is not null;
create index if not exists idx_outbound_docs_tenant_status_created
  on outbound_docs(tenant_id, status, created_at desc);

create table if not exists outbound_doc_items (
  tenant_id uuid not null references tenants(tenant_id),
  outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  planned_qty bigint not null,
  picked_qty bigint not null default 0,
  packed_qty bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, outbound_doc_id, sku_id)
);

create index if not exists idx_outbound_doc_items_tenant_sku
  on outbound_doc_items(tenant_id, sku_id);

create table if not exists outbound_pick_tasks (
  pick_task_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  from_location_id uuid,
  qty bigint not null,
  picked_qty bigint not null default 0,
  status text not null default 'PENDING',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_outbound_pick_tasks_tenant_doc
  on outbound_pick_tasks(tenant_id, outbound_doc_id);
create index if not exists idx_outbound_pick_tasks_tenant_status
  on outbound_pick_tasks(tenant_id, status);

create table if not exists waves (
  wave_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  status text not null default 'CREATED',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_waves_tenant_status_created
  on waves(tenant_id, status, created_at desc);

create table if not exists wave_docs (
  tenant_id uuid not null references tenants(tenant_id),
  wave_id uuid not null references waves(wave_id) on delete cascade,
  outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (tenant_id, wave_id, outbound_doc_id)
);

create index if not exists idx_wave_docs_tenant_outbound
  on wave_docs(tenant_id, outbound_doc_id);

create table if not exists pack_records (
  pack_record_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
  shipment_id uuid references shipments(shipment_id),
  package_no text not null,
  weight_gram int,
  operator_id uuid,
  packed_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create unique index if not exists idx_pack_records_tenant_package
  on pack_records(tenant_id, shipment_id, package_no);
create index if not exists idx_pack_records_tenant_outbound
  on pack_records(tenant_id, outbound_doc_id);
