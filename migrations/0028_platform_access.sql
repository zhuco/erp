-- 0028: Platform access
-- Shops auth status + platform tokens + webhook nonce + sync jobs

alter table shops
  add column if not exists name text not null default '',
  add column if not exists auth_meta_json jsonb not null default '{}'::jsonb;

alter table shops
  alter column status set default 'AUTHORIZED';

update shops
  set name = shop_name
where name is null or name = '';

update shops
  set status = 'AUTHORIZED'
where status = 'ACTIVE';

create index if not exists idx_shops_tenant_status
  on shops(tenant_id, status);

create table if not exists platform_tokens (
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  access_token text,
  refresh_token text,
  expires_at timestamptz,
  scopes text,
  last_refresh_at timestamptz,
  last_error text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, shop_id)
);

create index if not exists idx_platform_tokens_expires
  on platform_tokens(tenant_id, expires_at);

create table if not exists webhook_nonces (
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null,
  shop_id uuid not null references shops(shop_id),
  nonce text not null,
  received_at timestamptz not null default now(),
  expires_at timestamptz not null,
  primary key (tenant_id, platform, shop_id, nonce)
);

create index if not exists idx_webhook_nonces_expires
  on webhook_nonces(expires_at);

create table if not exists sync_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  job_type text not null,
  cursor text,
  last_time timestamptz,
  status text not null default 'PENDING',
  last_error text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, shop_id, job_type)
);

create index if not exists idx_sync_jobs_tenant_status
  on sync_jobs(tenant_id, status, updated_at desc);
