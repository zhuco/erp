-- Opening posts ledger rows. A correction adds another row and does not replace the first.

create table if not exists opening_documents (
  tenant_id uuid not null,
  opening_id uuid not null,
  cutoff_date text not null,
  status text not null,
  version_no integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, opening_id)
);

create table if not exists opening_qty_ledgers (
  tenant_id uuid not null,
  ledger_id uuid not null,
  opening_id uuid not null,
  delta_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, ledger_id)
);

create table if not exists opening_cost_ledgers (
  tenant_id uuid not null,
  ledger_id uuid not null,
  opening_id uuid not null,
  amount_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, ledger_id)
);
