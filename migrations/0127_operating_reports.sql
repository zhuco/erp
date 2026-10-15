-- Operating report snapshots. Totals are the full authorized range, grouped by currency.
create table if not exists operating_report_runs (
  tenant_id uuid not null,
  run_id uuid not null,
  cutoff_at text not null,
  date_from text,
  date_to text,
  partner_code text,
  warehouse_id uuid,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id)
);

create table if not exists operating_report_totals (
  tenant_id uuid not null,
  run_id uuid not null,
  metric text not null,
  currency text not null,
  amount_cent bigint not null,
  qty bigint not null,
  definition text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id, metric, currency)
);

create table if not exists operating_report_lines (
  tenant_id uuid not null,
  run_id uuid not null,
  line_no integer not null,
  metric text not null,
  currency text not null,
  source_type text not null,
  source_id text not null,
  partner_code text not null,
  warehouse_id uuid,
  amount_cent bigint not null,
  qty bigint not null,
  document_date text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id, line_no)
);
