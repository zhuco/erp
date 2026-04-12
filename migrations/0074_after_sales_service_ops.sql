create table if not exists after_sales_intercepts (
  intercept_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  after_sales_id uuid references after_sales(id) on delete set null,
  carrier_name text,
  tracking_no text,
  status text not null default 'PROCESSING',
  reason text,
  requested_at timestamptz not null default now(),
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_after_sales_intercepts_tenant_status
  on after_sales_intercepts(tenant_id, status, requested_at desc);
create index if not exists idx_after_sales_intercepts_tenant_order
  on after_sales_intercepts(tenant_id, order_id);

create table if not exists after_sales_inspections (
  inspection_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_id uuid not null references after_sales(id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  warehouse_id uuid references warehouses(warehouse_id),
  qty bigint not null default 0,
  result_code text not null default 'PENDING',
  inspector_name text,
  inspected_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_after_sales_inspections_tenant_after_sales
  on after_sales_inspections(tenant_id, after_sales_id);
create index if not exists idx_after_sales_inspections_tenant_result
  on after_sales_inspections(tenant_id, result_code, inspected_at desc);

create table if not exists after_sales_sla_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_type text not null,
  response_hours int not null default 0,
  resolution_hours int not null default 0,
  escalation_role text,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_after_sales_sla_rules_tenant_status
  on after_sales_sla_rules(tenant_id, status, updated_at desc);
