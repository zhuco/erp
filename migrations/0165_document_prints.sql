-- A print snapshot freezes one saved document version. A reprint does not change that version.
-- Download is checked against the same tenant. Paper output is not claimed here.

create table if not exists shared_document_prints (
  tenant_id uuid not null,
  print_id uuid not null,
  document_kind text not null,
  document_id uuid not null,
  version_no integer not null,
  company_name text not null,
  line_count integer not null,
  total_cent bigint not null,
  snapshot_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, print_id)
);

create table if not exists shared_document_reprints (
  tenant_id uuid not null,
  reprint_id uuid not null,
  print_id uuid not null,
  reason text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, reprint_id)
);
