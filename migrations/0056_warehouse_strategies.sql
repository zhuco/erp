-- 0049: Warehouse strategies (shipment preprocess, pick recommend, stock count suggestions)

alter table shipments
  add column if not exists sender_address_id uuid;

alter table warehouse_locations
  add column if not exists priority int not null default 0,
  add column if not exists pick_zone text;

create table if not exists shipment_preprocess_strategies (
  strategy_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  enabled boolean not null default true,
  priority int not null default 0,
  effective_from timestamptz,
  effective_to timestamptz,
  conditions_json jsonb not null default '[]'::jsonb,
  rules_json jsonb not null default '{}'::jsonb,
  auto_freeze boolean not null default false,
  auto_waybill_reserve boolean not null default false,
  sender_address_id uuid,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_shipment_preprocess_strategies_tenant_priority
  on shipment_preprocess_strategies(tenant_id, priority desc, updated_at desc);

create table if not exists shipment_preprocess_hits (
  hit_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  strategy_id uuid references shipment_preprocess_strategies(strategy_id),
  shipment_id uuid,
  order_id uuid,
  hit_reason text,
  result_json jsonb,
  success boolean not null default true,
  created_at timestamptz not null default now()
);

create index if not exists idx_shipment_preprocess_hits_tenant_time
  on shipment_preprocess_hits(tenant_id, created_at desc);
create index if not exists idx_shipment_preprocess_hits_strategy
  on shipment_preprocess_hits(strategy_id, created_at desc);
create index if not exists idx_shipment_preprocess_hits_shipment
  on shipment_preprocess_hits(shipment_id);

create table if not exists pick_recommend_strategies (
  strategy_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  enabled boolean not null default true,
  priority int not null default 0,
  method text not null default 'FIFO',
  prefer_location_priority boolean not null default false,
  prefer_pick_zone boolean not null default false,
  prefer_sufficient boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_pick_recommend_strategies_tenant_priority
  on pick_recommend_strategies(tenant_id, priority desc, updated_at desc);

create table if not exists pick_recommend_hits (
  hit_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  strategy_id uuid references pick_recommend_strategies(strategy_id),
  pick_task_id uuid,
  sku_id uuid,
  from_location_id uuid,
  reason text,
  success boolean not null default true,
  created_at timestamptz not null default now()
);

create index if not exists idx_pick_recommend_hits_tenant_time
  on pick_recommend_hits(tenant_id, created_at desc);
create index if not exists idx_pick_recommend_hits_strategy
  on pick_recommend_hits(strategy_id, created_at desc);

create table if not exists stock_count_strategies (
  strategy_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  enabled boolean not null default true,
  strategy_type text not null,
  config_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_stock_count_strategies_tenant
  on stock_count_strategies(tenant_id, updated_at desc);

create table if not exists stock_count_suggestions (
  suggestion_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  strategy_id uuid,
  strategy_name text,
  reason text,
  item_count int not null default 0,
  items_json jsonb not null default '[]'::jsonb,
  status text not null default 'OPEN',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_stock_count_suggestions_tenant_time
  on stock_count_suggestions(tenant_id, created_at desc);
create index if not exists idx_stock_count_suggestions_strategy
  on stock_count_suggestions(strategy_id, created_at desc);
