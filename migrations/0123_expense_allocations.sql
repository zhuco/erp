-- Expense cost is the selected tax basis, not the gross amount by default.
create table if not exists expense_documents (
  tenant_id uuid not null,
  expense_id uuid not null,
  category text not null,
  partner_code text not null,
  source_type text not null,
  source_id text not null,
  currency text not null,
  gross_cent bigint not null,
  deductible_tax_cent bigint not null,
  tax_is_cost boolean not null,
  cost_cent bigint not null,
  cost_version integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, expense_id)
);

create table if not exists expense_allocations (
  tenant_id uuid not null,
  expense_id uuid not null,
  line_no integer not null,
  target_code text not null,
  weight bigint not null,
  amount_cent bigint not null,
  residual boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, expense_id, line_no)
);

create table if not exists expense_adjustments (
  tenant_id uuid not null,
  adjustment_id uuid not null,
  expense_id uuid not null,
  amount_cent bigint not null,
  reason text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, adjustment_id)
);
