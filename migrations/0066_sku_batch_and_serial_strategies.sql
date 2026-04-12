-- 0054: SKU batch strategies + serial rules

create table if not exists sku_batch_strategies (
  strategy_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid references skus(sku_id),
  name text not null,
  enabled boolean not null default true,
  expire_threshold_days int not null default 0,
  fifo_enabled boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_sku_batch_strategies_tenant
  on sku_batch_strategies(tenant_id, enabled, updated_at desc);

create index if not exists idx_sku_batch_strategies_sku
  on sku_batch_strategies(tenant_id, sku_id);

create table if not exists sku_serial_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid references skus(sku_id),
  name text not null,
  prefix text not null,
  seq_length int not null default 6,
  increment_step int not null default 1,
  enabled boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_sku_serial_rules_tenant
  on sku_serial_rules(tenant_id, enabled, updated_at desc);

create index if not exists idx_sku_serial_rules_sku
  on sku_serial_rules(tenant_id, sku_id);
