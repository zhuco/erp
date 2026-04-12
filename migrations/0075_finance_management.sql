-- 0075: Finance management accounting extensions

create table if not exists ar_docs (
  ar_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  customer_id uuid references customers(customer_id) on delete set null,
  ref_type text not null,
  ref_id text not null,
  amount_cent bigint not null default 0,
  currency text not null default 'CNY',
  status text not null default 'OPEN',
  due_date date,
  written_off_at timestamptz,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_ar_docs_tenant_status
  on ar_docs(tenant_id, status, created_at desc);
create index if not exists idx_ar_docs_tenant_customer
  on ar_docs(tenant_id, customer_id, created_at desc);

create table if not exists ap_payment_plans (
  plan_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  ap_id uuid not null references ap_docs(ap_id) on delete cascade,
  supplier_id uuid references suppliers(supplier_id) on delete set null,
  planned_date date,
  amount_cent bigint not null default 0,
  status text not null default 'PLANNED',
  paid_at timestamptz,
  notes text,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_ap_payment_plans_tenant_status
  on ap_payment_plans(tenant_id, status, created_at desc);
create index if not exists idx_ap_payment_plans_tenant_ap
  on ap_payment_plans(tenant_id, ap_id);

create table if not exists finance_expenses (
  expense_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  category text not null,
  amount_cent bigint not null default 0,
  currency text not null default 'CNY',
  allocation_rule jsonb not null default '{}'::jsonb,
  period_start date,
  period_end date,
  status text not null default 'DRAFT',
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_finance_expenses_tenant_status
  on finance_expenses(tenant_id, status, created_at desc);
create index if not exists idx_finance_expenses_tenant_category
  on finance_expenses(tenant_id, category);

create table if not exists inventory_costing_runs (
  costing_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  method text not null,
  period_start date,
  period_end date,
  status text not null default 'DRAFT',
  variance_cent bigint not null default 0,
  result_json jsonb not null default '{}'::jsonb,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_inventory_costing_runs_tenant_status
  on inventory_costing_runs(tenant_id, status, created_at desc);

create table if not exists consignment_settlements (
  settlement_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  partner_name text not null,
  agreement_no text,
  period_start date,
  period_end date,
  sales_amount_cent bigint not null default 0,
  commission_rate numeric(10,4) not null default 0,
  commission_cent bigint not null default 0,
  status text not null default 'DRAFT',
  settled_at timestamptz,
  notes text,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_consignment_settlements_tenant_status
  on consignment_settlements(tenant_id, status, created_at desc);
create index if not exists idx_consignment_settlements_tenant_partner
  on consignment_settlements(tenant_id, partner_name);

create table if not exists tax_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  tax_code text not null,
  tax_rate numeric(10,4) not null default 0,
  effective_from date,
  effective_to date,
  status text not null default 'ACTIVE',
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, tax_code, effective_from)
);

create index if not exists idx_tax_rules_tenant_status
  on tax_rules(tenant_id, status, effective_from desc);

create table if not exists invoice_adjustments (
  adjustment_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  invoice_id uuid not null references invoices(invoice_id) on delete cascade,
  adjustment_type text not null,
  reason text,
  status text not null default 'PENDING',
  applied_at timestamptz,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_invoice_adjustments_tenant_status
  on invoice_adjustments(tenant_id, status, created_at desc);
create index if not exists idx_invoice_adjustments_invoice
  on invoice_adjustments(tenant_id, invoice_id);
