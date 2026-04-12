-- 0040: Permissions catalog
create table if not exists permissions (
  tenant_id uuid not null references tenants(tenant_id),
  code text not null,
  name text not null,
  module text not null,
  description text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, code)
);

create index if not exists idx_permissions_tenant_module
  on permissions(tenant_id, module);
