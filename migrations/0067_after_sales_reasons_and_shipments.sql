create table if not exists after_sales_reasons (
  tenant_id uuid not null references tenants(tenant_id),
  reason_code text not null,
  label text not null,
  category text,
  status text not null default 'ACTIVE',
  sort_order int not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, reason_code)
);

create index if not exists idx_after_sales_reasons_tenant_status
  on after_sales_reasons(tenant_id, status, sort_order, updated_at desc);

create table if not exists after_sales_quality_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_type text not null,
  result_code text not null,
  status text not null default 'ACTIVE',
  sort_order int not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, after_sales_type, result_code)
);

create index if not exists idx_after_sales_quality_rules_tenant_type
  on after_sales_quality_rules(tenant_id, after_sales_type, status, sort_order);

create table if not exists after_sales_shipments (
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_id uuid not null references after_sales(id) on delete cascade,
  shipment_id uuid not null references shipments(shipment_id) on delete cascade,
  shipment_type text not null,
  origin_shipment_id uuid,
  created_at timestamptz not null default now(),
  primary key (tenant_id, after_sales_id, shipment_id)
);

create index if not exists idx_after_sales_shipments_tenant_after_sales
  on after_sales_shipments(tenant_id, after_sales_id, created_at desc);
create index if not exists idx_after_sales_shipments_tenant_shipment
  on after_sales_shipments(tenant_id, shipment_id);
