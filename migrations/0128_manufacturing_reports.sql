-- Manufacturing report metrics stay separate. Plan is not issue, consume, report, or receipt.
create table if not exists mfg_report_runs (
  tenant_id uuid not null,
  run_id uuid not null,
  cutoff_at text not null,
  cost_version integer,
  quality_status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id)
);

create table if not exists mfg_report_lines (
  tenant_id uuid not null,
  run_id uuid not null,
  line_no integer not null,
  work_order_id uuid not null,
  work_order_no text not null,
  metric text not null,
  qty_micros bigint not null,
  amount_cent bigint not null,
  estimated boolean not null,
  quality_status text not null,
  source_type text not null,
  source_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id, line_no)
);
