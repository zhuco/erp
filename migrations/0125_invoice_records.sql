-- Manual invoice records. official_issued stays false; entry is not a tax-authority issue.
create table if not exists invoice_records (
  tenant_id uuid not null,
  invoice_id uuid not null,
  invoice_number text not null,
  issuer_code text not null,
  invoice_date text not null,
  currency text not null,
  gross_cent bigint not null,
  tax_cent bigint not null,
  source_type text not null,
  source_id text not null,
  balance_id uuid,
  official_issued boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, invoice_id),
  unique (tenant_id, issuer_code, invoice_number)
);

create table if not exists aging_snapshots (
  tenant_id uuid not null,
  snapshot_id uuid not null,
  balance_id uuid not null,
  cutoff_date text not null,
  open_cent bigint not null,
  bucket text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, snapshot_id, balance_id)
);
