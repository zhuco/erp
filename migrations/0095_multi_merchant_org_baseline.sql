-- 0095: Multi-merchant, multi-organization, and shop permission baseline

alter table tenants
  add column if not exists multi_merchant_enabled boolean not null default false,
  add column if not exists multi_organization_enabled boolean not null default false;

-- merchant catalog scoped by tenant
create table if not exists merchants (
  tenant_id uuid not null references tenants(tenant_id),
  merchant_id uuid not null default gen_random_uuid(),
  name text not null default '',
  code text not null default '',
  description text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, merchant_id)
);

create index if not exists idx_merchants_tenant
  on merchants(tenant_id, merchant_id);

-- organization catalog scoped by tenant
create table if not exists organizations (
  tenant_id uuid not null references tenants(tenant_id),
  organization_id uuid not null default gen_random_uuid(),
  name text not null default '',
  code text not null default '',
  description text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, organization_id)
);

create index if not exists idx_organizations_tenant
  on organizations(tenant_id, organization_id);

create table if not exists user_merchant_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  merchant_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, merchant_id)
);

create index if not exists idx_user_merchant_access_user
  on user_merchant_access(tenant_id, user_id);

create table if not exists user_organization_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  organization_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, organization_id)
);

create index if not exists idx_user_organization_access_user
  on user_organization_access(tenant_id, user_id);
