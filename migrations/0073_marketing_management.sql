-- 0073: Marketing and pricing management

create table if not exists marketing_price_lists (
  price_list_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  channel_id uuid references channels(channel_id),
  currency text not null default 'CNY',
  includes_tax boolean not null default true,
  status text not null default 'DRAFT',
  version int not null default 1,
  effective_from timestamptz,
  effective_to timestamptz,
  notes text,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_marketing_price_lists_tenant_status
  on marketing_price_lists(tenant_id, status);
create index if not exists idx_marketing_price_lists_tenant_channel
  on marketing_price_lists(tenant_id, channel_id);

create table if not exists marketing_price_items (
  item_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  price_list_id uuid not null references marketing_price_lists(price_list_id) on delete cascade,
  sku_id uuid not null references skus(sku_id),
  base_price_cent bigint,
  channel_price_cent bigint,
  member_price_cent bigint,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (price_list_id, sku_id)
);

create index if not exists idx_marketing_price_items_list
  on marketing_price_items(tenant_id, price_list_id);

create table if not exists marketing_promotions (
  promotion_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  promo_type text not null,
  status text not null default 'DRAFT',
  priority int not null default 0,
  exclusive boolean not null default false,
  scope_type text not null default 'SKU',
  scope_json jsonb not null default '{}'::jsonb,
  rule_json jsonb not null default '{}'::jsonb,
  start_at timestamptz,
  end_at timestamptz,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_marketing_promotions_tenant_status
  on marketing_promotions(tenant_id, status);

create table if not exists marketing_gift_policies (
  policy_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  status text not null default 'DRAFT',
  strategy text not null default 'RANDOM',
  probability double precision,
  scope_json jsonb not null default '{}'::jsonb,
  gift_pool jsonb not null default '[]'::jsonb,
  start_at timestamptz,
  end_at timestamptz,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_marketing_gift_policies_tenant_status
  on marketing_gift_policies(tenant_id, status);

create table if not exists marketing_coupons (
  coupon_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  code_prefix text,
  discount_type text not null,
  discount_value numeric(18,4) not null,
  max_usage int not null default 0,
  per_user_limit int not null default 0,
  status text not null default 'DRAFT',
  rules_json jsonb not null default '{}'::jsonb,
  start_at timestamptz,
  end_at timestamptz,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_marketing_coupons_tenant_status
  on marketing_coupons(tenant_id, status);

create table if not exists marketing_costs (
  cost_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  channel_id uuid references channels(channel_id),
  amount_cent bigint not null,
  currency text not null default 'CNY',
  allocation_rule jsonb not null default '{}'::jsonb,
  period_start date,
  period_end date,
  status text not null default 'ACTIVE',
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_marketing_costs_tenant_status
  on marketing_costs(tenant_id, status);
create index if not exists idx_marketing_costs_tenant_channel
  on marketing_costs(tenant_id, channel_id);

create table if not exists marketing_postmortems (
  postmortem_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  promotion_id uuid references marketing_promotions(promotion_id),
  channel_id uuid references channels(channel_id),
  summary text,
  metrics_json jsonb not null default '{}'::jsonb,
  created_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_marketing_postmortems_tenant
  on marketing_postmortems(tenant_id, created_at desc);
