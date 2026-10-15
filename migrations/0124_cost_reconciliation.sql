-- Cost reconciliation at one cutoff. A difference references the source ledger and does not clear it.
create table if not exists cost_periods (
  tenant_id uuid not null,
  period_code text not null,
  currency text not null,
  cutoff_at text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, period_code)
);

create table if not exists cost_reconciliations (
  tenant_id uuid not null,
  reconciliation_id uuid not null,
  period_code text not null,
  bucket text not null,
  book_cent bigint not null,
  ledger_cent bigint not null,
  difference_cent bigint not null,
  source_ref text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, reconciliation_id, bucket)
);

create table if not exists cost_period_adjustments (
  tenant_id uuid not null,
  adjustment_id uuid not null,
  period_code text not null,
  document_date text not null,
  amount_cent bigint not null,
  reason text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, adjustment_id)
);
