-- A confirmed import can publish its row counts. A pending job cannot feed a batch.

create table if not exists shared_import_downloads (
  tenant_id uuid not null,
  download_id uuid not null,
  job_id uuid not null,
  imported_count bigint not null,
  error_count bigint not null,
  skipped_count bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, download_id)
);

create table if not exists shared_import_batches (
  tenant_id uuid not null,
  batch_id uuid not null,
  job_id uuid not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, batch_id)
);
