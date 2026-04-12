-- 0033: Procurement (suppliers + purchase orders + receipts + returns + AP + outbound docs)

alter table suppliers
  add column if not exists contact text;
alter table suppliers
  add column if not exists settlement_terms text;
alter table suppliers
  add column if not exists status text not null default 'ACTIVE';

create index if not exists idx_suppliers_tenant_status
  on suppliers(tenant_id, status);

create table if not exists purchase_orders (
  po_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  supplier_id uuid not null references suppliers(supplier_id),
  status text not null default 'DRAFT',
  total_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_purchase_orders_tenant_created
  on purchase_orders(tenant_id, created_at desc);
create index if not exists idx_purchase_orders_tenant_status
  on purchase_orders(tenant_id, status, created_at desc);
create index if not exists idx_purchase_orders_tenant_supplier
  on purchase_orders(tenant_id, supplier_id, created_at desc);

create table if not exists purchase_order_items (
  tenant_id uuid not null references tenants(tenant_id),
  po_id uuid not null references purchase_orders(po_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  price_cent bigint not null,
  received_qty bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, po_id, sku_id)
);

create index if not exists idx_purchase_order_items_tenant_po
  on purchase_order_items(tenant_id, po_id);
create index if not exists idx_purchase_order_items_tenant_sku
  on purchase_order_items(tenant_id, sku_id);

create table if not exists purchase_receipts (
  receipt_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  po_id uuid not null references purchase_orders(po_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  status text not null default 'DRAFT',
  confirmed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_purchase_receipts_tenant_created
  on purchase_receipts(tenant_id, created_at desc);
create index if not exists idx_purchase_receipts_tenant_status
  on purchase_receipts(tenant_id, status, created_at desc);
create index if not exists idx_purchase_receipts_tenant_po
  on purchase_receipts(tenant_id, po_id, created_at desc);

create table if not exists receipt_items (
  tenant_id uuid not null references tenants(tenant_id),
  receipt_id uuid not null references purchase_receipts(receipt_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, receipt_id, sku_id)
);

create index if not exists idx_receipt_items_tenant_receipt
  on receipt_items(tenant_id, receipt_id);
create index if not exists idx_receipt_items_tenant_sku
  on receipt_items(tenant_id, sku_id);

create table if not exists purchase_returns (
  return_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  po_id uuid not null references purchase_orders(po_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  status text not null default 'DRAFT',
  confirmed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_purchase_returns_tenant_created
  on purchase_returns(tenant_id, created_at desc);
create index if not exists idx_purchase_returns_tenant_status
  on purchase_returns(tenant_id, status, created_at desc);
create index if not exists idx_purchase_returns_tenant_po
  on purchase_returns(tenant_id, po_id, created_at desc);

create table if not exists return_items (
  tenant_id uuid not null references tenants(tenant_id),
  return_id uuid not null references purchase_returns(return_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, return_id, sku_id)
);

create index if not exists idx_return_items_tenant_return
  on return_items(tenant_id, return_id);
create index if not exists idx_return_items_tenant_sku
  on return_items(tenant_id, sku_id);

create table if not exists purchase_suggestions (
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  suggested_qty bigint not null,
  reason text not null,
  computed_at timestamptz not null default now(),
  primary key (tenant_id, sku_id)
);

create index if not exists idx_purchase_suggestions_tenant_time
  on purchase_suggestions(tenant_id, computed_at desc);
create index if not exists idx_purchase_suggestions_tenant_qty
  on purchase_suggestions(tenant_id, suggested_qty desc);

create table if not exists ap_docs (
  ap_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  supplier_id uuid not null references suppliers(supplier_id),
  ref_type text not null,
  ref_id text not null,
  amount_cent bigint not null,
  status text not null default 'OPEN',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, ref_type, ref_id)
);

create index if not exists idx_ap_docs_tenant_status
  on ap_docs(tenant_id, status, created_at desc);
create index if not exists idx_ap_docs_tenant_supplier
  on ap_docs(tenant_id, supplier_id, created_at desc);
