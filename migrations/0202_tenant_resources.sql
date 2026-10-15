-- Owned resources carry the enterprise key. A cross-tenant read or write is refused.
-- Ops access is a separate audited row and is not a business login.

create table if not exists tenant_resources (
  tenant_id uuid not null,
  resource_id uuid not null,
  resource_kind text not null,
  body_text text not null,
  body_version integer not null default 1,
  created_at timestamptz not null default now(),
  primary key (tenant_id, resource_id)
);

create table if not exists tenant_ops_audits (
  tenant_id uuid not null,
  audit_id uuid not null,
  operator_id text not null,
  resource_kind text not null,
  reason_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, audit_id)
);
