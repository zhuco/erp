-- A draft stays with one enterprise, user, page, and object version.
-- Local undo cannot cancel a write the server already accepted.

create table if not exists edit_drafts (
  tenant_id uuid not null,
  draft_id uuid not null,
  user_id text not null,
  page_code text not null,
  object_id text not null,
  version_no bigint not null,
  body_text text not null,
  status text not null,
  service_address text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, draft_id),
  unique (tenant_id, user_id, page_code, object_id)
);

create table if not exists draft_recoveries (
  tenant_id uuid not null,
  recovery_id uuid not null,
  draft_id uuid not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, recovery_id)
);

create table if not exists lost_response_lookups (
  tenant_id uuid not null,
  lookup_id uuid not null,
  request_id text not null,
  stored_status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, lookup_id),
  unique (tenant_id, request_id)
);
