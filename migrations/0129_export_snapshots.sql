-- Export jobs bind tenant, actor, query or selection snapshot, and output schema.
create table if not exists export_snapshots (
  tenant_id uuid not null,
  job_id uuid not null,
  actor_user_id text not null,
  scope_kind text not null,
  query_snapshot text not null,
  output_schema text not null,
  status text not null,
  progress integer not null,
  read_cancelled boolean not null default false,
  expires_at timestamptz not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id),
  constraint export_snapshots_scope_check check (scope_kind in ('QUERY', 'SELECTED', 'FROZEN_MATCH'))
);

create table if not exists export_snapshot_items (
  tenant_id uuid not null,
  job_id uuid not null,
  item_no integer not null,
  source_id text not null,
  status text not null,
  error_text text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id, item_no)
);

create table if not exists export_files (
  tenant_id uuid not null,
  file_id uuid not null,
  job_id uuid not null,
  file_name text not null,
  byte_count bigint not null,
  content_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, file_id)
);

create table if not exists export_downloads (
  tenant_id uuid not null,
  download_id uuid not null,
  file_id uuid not null,
  actor_user_id text not null,
  authorized boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, download_id)
);
