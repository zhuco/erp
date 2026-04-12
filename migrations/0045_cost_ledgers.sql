-- 0045: SKU cost ledgers + order COGS tracking

create table if not exists sku_cost_ledgers (
  ledger_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  previous_cost_cent bigint,
  new_cost_cent bigint not null,
  ref_type text,
  ref_id text,
  created_at timestamptz not null default now()
);

create index if not exists idx_sku_cost_ledgers_tenant_sku
  on sku_cost_ledgers(tenant_id, sku_id, created_at desc);

create table if not exists order_cogs (
  ledger_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  shipment_id uuid,
  outbound_doc_id uuid,
  order_item_id uuid,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  unit_cost_cent bigint not null,
  total_cost_cent bigint not null,
  ref_type text not null,
  ref_id text not null,
  created_at timestamptz not null default now(),
  unique (tenant_id, ref_type, ref_id, sku_id)
);

create index if not exists idx_order_cogs_tenant_order
  on order_cogs(tenant_id, order_id);
create index if not exists idx_order_cogs_tenant_shipment
  on order_cogs(tenant_id, shipment_id);

alter table ap_docs
  add column if not exists paid_at timestamptz;
