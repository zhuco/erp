-- 0001_init.sql
-- Core schema for SaaS multi-tenant ERP (platform orders as source of truth)
-- Notes:
-- - Large tables use partitioning (monthly RANGE or HASH) for write/read scalability.
-- - Idempotency is enforced via inbox_dedup (HASH partitioned).
-- - Order uniqueness across partitions is guaranteed by order_key_index (HASH partitioned).

create extension if not exists pgcrypto;

-- ==============
-- Tenants / Shops
-- ==============
create table if not exists tenants (
  tenant_id uuid primary key default gen_random_uuid(),
  name text not null,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

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

create index if not exists idx_shops_tenant_platform on shops(tenant_id, platform);

-- ==========================
-- Inbox: raw + dedup + events
-- ==========================
create table if not exists inbox_raw_payloads (
  raw_id bigserial primary key,
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null,
  shop_id uuid not null references shops(shop_id),
  headers jsonb not null default '{}'::jsonb,
  body bytea not null,
  received_at timestamptz not null default now()
);

-- Dedup table (HASH partitioned)
create table if not exists inbox_dedup (
  tenant_id uuid not null references tenants(tenant_id),
  dedup_key text not null,
  first_seen_at timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  seen_count int not null default 1,
  primary key (tenant_id, dedup_key)
) partition by hash (dedup_key);

do $$
declare i int;
begin
  for i in 0..31 loop
    execute format('create table if not exists inbox_dedup_p%1$s partition of inbox_dedup for values with (modulus 32, remainder %1$s);', i);
  end loop;
end $$;

-- Inbox events (monthly RANGE partitioned)
create table if not exists inbox_events (
  tenant_id uuid not null references tenants(tenant_id),
  event_month date not null, -- date_trunc('month', received_at)::date
  inbox_event_id uuid not null default gen_random_uuid(),
  platform text not null,
  shop_id uuid not null references shops(shop_id),
  dedup_key text not null,
  raw_id bigint not null references inbox_raw_payloads(raw_id),
  status text not null default 'RECEIVED', -- RECEIVED/VALIDATED/DUPLICATE/PROCESSED/FAILED
  error text not null default '',
  received_at timestamptz not null default now(),
  processed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, event_month, inbox_event_id)
) partition by range (event_month);

-- Default partition (safety)
create table if not exists inbox_events_default partition of inbox_events default;

create index if not exists idx_inbox_events_status on inbox_events(status, received_at desc);
create index if not exists idx_inbox_events_shop on inbox_events(tenant_id, shop_id, received_at desc);
create index if not exists idx_inbox_events_by_id on inbox_events(tenant_id, inbox_event_id);

-- ==================
-- Outbox (DB outbox)
-- ==================
create table if not exists outbox_events (
  tenant_id uuid not null references tenants(tenant_id),
  event_month date not null,
  outbox_id uuid not null default gen_random_uuid(),
  event_type text not null,
  payload jsonb not null,
  status text not null default 'PENDING', -- PENDING/SENT/FAILED/DEAD
  try_count int not null default 0,
  next_retry_at timestamptz not null default now(),
  last_error text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, event_month, outbox_id)
) partition by range (event_month);

create table if not exists outbox_events_default partition of outbox_events default;

create index if not exists idx_outbox_pending on outbox_events(status, next_retry_at);
create index if not exists idx_outbox_by_id on outbox_events(tenant_id, outbox_id);

-- =================
-- Orders projection
-- =================

-- Order key index: unique mapping from (tenant, platform, platform_order_id) to internal (order_id, order_month)
create table if not exists order_key_index (
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null,
  platform_order_id text not null,
  order_id uuid not null,
  order_month date not null,
  shop_id uuid not null references shops(shop_id),
  order_created_at timestamptz not null,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, platform, platform_order_id)
) partition by hash (platform_order_id);

do $$
declare i int;
begin
  for i in 0..31 loop
    execute format('create table if not exists order_key_index_p%1$s partition of order_key_index for values with (modulus 32, remainder %1$s);', i);
  end loop;
end $$;

create index if not exists idx_order_key_index_order on order_key_index(tenant_id, order_id);

-- Order index: locate partition by order_id
create table if not exists order_index (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  order_month date not null,
  platform text not null,
  platform_order_id text not null,
  shop_id uuid not null references shops(shop_id),
  order_created_at timestamptz not null,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, order_id)
) partition by hash (order_id);

do $$
declare i int;
begin
  for i in 0..31 loop
    execute format('create table if not exists order_index_p%1$s partition of order_index for values with (modulus 32, remainder %1$s);', i);
  end loop;
end $$;

create index if not exists idx_order_index_platform_order on order_index(tenant_id, platform, platform_order_id);

-- Orders table (monthly RANGE partitioned by order_month)
create table if not exists orders (
  tenant_id uuid not null references tenants(tenant_id),
  order_month date not null,
  order_id uuid not null,
  platform text not null,
  shop_id uuid not null references shops(shop_id),
  platform_order_id text not null,
  status text not null,
  total_amount_cent bigint not null default 0,
  currency text not null default 'CNY',
  buyer_name text,
  receiver_name text,
  receiver_phone text,
  receiver_address text,
  order_created_at timestamptz not null,
  platform_updated_at timestamptz,
  ingested_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  raw jsonb not null default '{}'::jsonb,
  primary key (tenant_id, order_month, order_id),
  unique (tenant_id, order_month, platform, platform_order_id)
) partition by range (order_month);

create table if not exists orders_default partition of orders default;

create index if not exists idx_orders_shop_time on orders(tenant_id, shop_id, order_created_at desc);
create index if not exists idx_orders_platform_time on orders(tenant_id, platform, order_created_at desc);

-- Order items (monthly RANGE partitioned)
create table if not exists order_items (
  tenant_id uuid not null references tenants(tenant_id),
  order_month date not null,
  order_id uuid not null,
  line_id text not null,
  sku_id text,
  title text,
  qty int not null default 1,
  price_cent bigint not null default 0,
  amount_cent bigint not null default 0,
  raw jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, order_month, order_id, line_id)
) partition by range (order_month);

create table if not exists order_items_default partition of order_items default;

create index if not exists idx_order_items_order on order_items(tenant_id, order_id);

-- ===================
-- RBAC (JWT + Roles)
-- ===================
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

create table if not exists roles (
  role_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  created_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create table if not exists user_roles (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  role_id uuid not null references roles(role_id),
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, role_id)
);

create table if not exists role_permissions (
  tenant_id uuid not null references tenants(tenant_id),
  role_id uuid not null references roles(role_id),
  perm_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, role_id, perm_code)
);

create index if not exists idx_role_permissions_perm on role_permissions(tenant_id, perm_code);
