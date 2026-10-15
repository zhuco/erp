-- Service terms and exit requests stay with one enterprise.
-- The export is a manifest. It is not a database backup of another enterprise.

create table if not exists service_terms (
  tenant_id uuid not null,
  term_id uuid not null,
  version_text text not null,
  purpose_text text not null,
  retention_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, term_id)
);

create table if not exists exit_requests (
  tenant_id uuid not null,
  request_row_id uuid not null,
  applicant_id text not null,
  confirmation_text text not null,
  cutoff_text text not null,
  file_range text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, request_row_id)
);

create table if not exists exit_manifests (
  tenant_id uuid not null,
  manifest_id uuid not null,
  item_kind text not null,
  item_no text not null,
  line_count bigint,
  digest text,
  created_at timestamptz not null default now(),
  primary key (tenant_id, manifest_id)
);

alter table exit_manifests add column if not exists line_count bigint;
alter table exit_manifests add column if not exists digest text;
