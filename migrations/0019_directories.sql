-- 0019: Courier / Channel / Supplier directories

create table if not exists couriers (
  courier_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  code text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, code)
);

create index if not exists idx_couriers_tenant_code
  on couriers(tenant_id, code);

create table if not exists channels (
  channel_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, platform)
);

create index if not exists idx_channels_tenant_platform
  on channels(tenant_id, platform);

create table if not exists suppliers (
  supplier_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_suppliers_tenant_name
  on suppliers(tenant_id, name);
