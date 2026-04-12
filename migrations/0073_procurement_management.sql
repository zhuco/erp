-- 0073: Procurement management extensions (requests/plans/sourcing/inspections/reconciliations/performance)

create table if not exists procurement_requests (
  request_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  department text,
  requester text,
  status text not null default 'DRAFT',
  budget_cent bigint not null default 0,
  expected_at timestamptz,
  remark text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_procurement_requests_tenant_created
  on procurement_requests(tenant_id, created_at desc);
create index if not exists idx_procurement_requests_tenant_status
  on procurement_requests(tenant_id, status, created_at desc);

create table if not exists procurement_request_items (
  tenant_id uuid not null references tenants(tenant_id),
  request_id uuid not null references procurement_requests(request_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  price_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, request_id, sku_id)
);

create index if not exists idx_procurement_request_items_tenant_request
  on procurement_request_items(tenant_id, request_id);
create index if not exists idx_procurement_request_items_tenant_sku
  on procurement_request_items(tenant_id, sku_id);

create table if not exists procurement_plans (
  plan_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  request_id uuid references procurement_requests(request_id),
  supplier_id uuid references suppliers(supplier_id),
  status text not null default 'DRAFT',
  total_cent bigint not null default 0,
  planned_at timestamptz,
  expected_at timestamptz,
  remark text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_procurement_plans_tenant_created
  on procurement_plans(tenant_id, created_at desc);
create index if not exists idx_procurement_plans_tenant_status
  on procurement_plans(tenant_id, status, created_at desc);

create table if not exists procurement_plan_items (
  tenant_id uuid not null references tenants(tenant_id),
  plan_id uuid not null references procurement_plans(plan_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  price_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, plan_id, sku_id)
);

create index if not exists idx_procurement_plan_items_tenant_plan
  on procurement_plan_items(tenant_id, plan_id);
create index if not exists idx_procurement_plan_items_tenant_sku
  on procurement_plan_items(tenant_id, sku_id);

create table if not exists procurement_sourcing (
  sourcing_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  title text not null,
  category text,
  supplier_id uuid references suppliers(supplier_id),
  status text not null default 'OPEN',
  target_price_cent bigint not null default 0,
  remark text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_procurement_sourcing_tenant_created
  on procurement_sourcing(tenant_id, created_at desc);
create index if not exists idx_procurement_sourcing_tenant_status
  on procurement_sourcing(tenant_id, status, created_at desc);

create table if not exists procurement_inspections (
  inspection_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  receipt_id uuid references purchase_receipts(receipt_id),
  supplier_id uuid references suppliers(supplier_id),
  warehouse_id uuid references warehouses(warehouse_id),
  status text not null default 'PENDING',
  inspected_at timestamptz,
  remark text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_procurement_inspections_tenant_created
  on procurement_inspections(tenant_id, created_at desc);
create index if not exists idx_procurement_inspections_tenant_status
  on procurement_inspections(tenant_id, status, created_at desc);

create table if not exists procurement_inspection_items (
  tenant_id uuid not null references tenants(tenant_id),
  inspection_id uuid not null references procurement_inspections(inspection_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  passed_qty bigint not null default 0,
  failed_qty bigint not null default 0,
  result text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, inspection_id, sku_id)
);

create index if not exists idx_procurement_inspection_items_tenant_inspection
  on procurement_inspection_items(tenant_id, inspection_id);
create index if not exists idx_procurement_inspection_items_tenant_sku
  on procurement_inspection_items(tenant_id, sku_id);

create table if not exists procurement_reconciliations (
  reconciliation_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  supplier_id uuid not null references suppliers(supplier_id),
  period_start date not null,
  period_end date not null,
  amount_cent bigint not null default 0,
  status text not null default 'OPEN',
  remark text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_procurement_reconciliations_tenant_created
  on procurement_reconciliations(tenant_id, created_at desc);
create index if not exists idx_procurement_reconciliations_tenant_status
  on procurement_reconciliations(tenant_id, status, created_at desc);

create table if not exists supplier_performances (
  performance_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  supplier_id uuid not null references suppliers(supplier_id),
  period_start date not null,
  period_end date not null,
  delivery_score int not null default 0,
  quality_score int not null default 0,
  price_score int not null default 0,
  total_score int not null default 0,
  status text not null default 'ACTIVE',
  remark text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_supplier_performances_tenant_created
  on supplier_performances(tenant_id, created_at desc);
create index if not exists idx_supplier_performances_tenant_status
  on supplier_performances(tenant_id, status, created_at desc);
