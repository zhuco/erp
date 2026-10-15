-- Documents carry a stable id and a version. A stale expected version updates zero rows.
-- Query generations cancel older reads. An accepted write is not cleared by a newer read.

create table if not exists versioned_documents (
  tenant_id uuid not null,
  document_id uuid not null,
  version_no bigint not null,
  body_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, document_id)
);

create table if not exists version_conflicts (
  tenant_id uuid not null,
  conflict_id uuid not null,
  document_id uuid not null,
  actor_user_id text not null,
  expected_version bigint not null,
  stored_version bigint not null,
  kept_input text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, conflict_id)
);

create table if not exists query_generations (
  tenant_id uuid not null,
  query_id uuid not null,
  generation bigint not null,
  condition_digest text not null,
  cancelled boolean not null default false,
  applied boolean not null default false,
  result_text text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, query_id)
);
