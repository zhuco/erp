-- A precheck stores row errors only. Confirmation is the first write to shared items.

create table if not exists shared_import_jobs (
  tenant_id uuid not null,
  job_id uuid not null,
  file_digest text not null,
  column_map text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id)
);

create table if not exists shared_import_rows (
  tenant_id uuid not null,
  job_id uuid not null,
  line_no integer not null,
  item_code text not null,
  name text not null,
  unit_code text not null,
  status text not null,
  error_text text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id, line_no)
);
