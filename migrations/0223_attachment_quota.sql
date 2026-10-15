-- An attachment citation does not publish the storage key.
-- The quota is filled per tenant. A key from another tenant is refused.

create table if not exists shared_attachment_cites (
  tenant_id uuid not null,
  cite_id uuid not null,
  object_id text not null,
  attachment_id uuid not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, cite_id)
);

create table if not exists shared_attachment_quotas (
  tenant_id uuid primary key,
  quota_bytes bigint not null,
  created_at timestamptz not null default now()
);

create table if not exists shared_attachment_quota_checks (
  tenant_id uuid not null,
  check_id uuid not null,
  used_bytes bigint not null,
  next_bytes bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, check_id)
);
