-- Protected QA prerequisites copied from the named original migrations.

begin;

set local lock_timeout='3s';

do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;

-- From migrations/0001_init.sql

create table if not exists users (
  user_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  email text not null,
  password_hash text not null,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, email)
);

-- From migrations/0001_init.sql

create table if not exists shops (
  shop_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null, -- taobao/jd/pdd/douyin
  platform_shop_id text not null,
  shop_name text not null default '',
  status text not null default 'ACTIVE',
  credentials jsonb not null default '{}'::jsonb, -- token/app_key/etc (encrypted at app layer)
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, platform, platform_shop_id)
);

-- From migrations/0030_system_management.sql

create table if not exists user_shop_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  shop_id uuid not null references shops(shop_id),
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, shop_id)
);

create index if not exists idx_user_shop_access_user
  on user_shop_access(tenant_id, user_id);

-- From migrations/0030_system_management.sql

create table if not exists user_warehouse_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, warehouse_id)
);

create index if not exists idx_user_warehouse_access_user
  on user_warehouse_access(tenant_id, user_id);

-- From migrations/0095_multi_merchant_org_baseline.sql

create table if not exists user_merchant_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  merchant_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, merchant_id)
);

create index if not exists idx_user_merchant_access_user
  on user_merchant_access(tenant_id, user_id);

-- From migrations/0095_multi_merchant_org_baseline.sql

create table if not exists user_organization_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  organization_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, organization_id)
);

create index if not exists idx_user_organization_access_user
  on user_organization_access(tenant_id, user_id);

-- From migrations/0095_multi_merchant_org_baseline.sql

alter table tenants
  add column if not exists multi_merchant_enabled boolean not null default false,
  add column if not exists multi_organization_enabled boolean not null default false;

commit;
