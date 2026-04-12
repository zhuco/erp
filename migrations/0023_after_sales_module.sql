-- 0023: After-sales management (refund/return/exchange/resend)

create table if not exists after_sales (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  type text not null,
  status text not null default 'OPEN',
  reason_code text,
  amount_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_after_sales_tenant_created
  on after_sales(tenant_id, created_at desc);
create index if not exists idx_after_sales_tenant_status
  on after_sales(tenant_id, status, created_at desc);
create index if not exists idx_after_sales_tenant_order
  on after_sales(tenant_id, order_id);

create table if not exists after_sales_items (
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_id uuid not null references after_sales(id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null default 0,
  refund_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, after_sales_id, sku_id)
);

create index if not exists idx_after_sales_items_tenant_after_sales
  on after_sales_items(tenant_id, after_sales_id);
create index if not exists idx_after_sales_items_tenant_sku
  on after_sales_items(tenant_id, sku_id);

create table if not exists return_receipts (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_id uuid not null references after_sales(id) on delete cascade,
  warehouse_id uuid not null references warehouses(warehouse_id),
  status text not null default 'RECEIVING',
  received_at timestamptz,
  putaway_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, after_sales_id)
);

create index if not exists idx_return_receipts_tenant_created
  on return_receipts(tenant_id, created_at desc);
create index if not exists idx_return_receipts_tenant_status
  on return_receipts(tenant_id, status, created_at desc);

create table if not exists refund_docs (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_id uuid not null references after_sales(id) on delete cascade,
  amount_cent bigint not null,
  status text not null default 'PENDING',
  channel text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, after_sales_id)
);

create index if not exists idx_refund_docs_tenant_created
  on refund_docs(tenant_id, created_at desc);
create index if not exists idx_refund_docs_tenant_status
  on refund_docs(tenant_id, status, created_at desc);
