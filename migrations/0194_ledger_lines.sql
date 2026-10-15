-- Ledger lines keep source, currency, rate cutoff, and base amount.
-- A closed period does not accept another original line.

create table if not exists ledger_periods (
  tenant_id uuid not null,
  period_id uuid not null,
  period_code text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, period_id),
  unique (tenant_id, period_code)
);

create table if not exists ledger_lines (
  tenant_id uuid not null,
  line_id uuid not null,
  period_code text not null,
  voucher_no text not null,
  source_id text not null,
  account_code text not null,
  debit_cent bigint not null,
  credit_cent bigint not null,
  currency text not null,
  original_cent bigint not null,
  fx_cutoff text not null,
  base_currency text not null,
  base_cent bigint not null,
  rounding_cent bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, line_id)
);
