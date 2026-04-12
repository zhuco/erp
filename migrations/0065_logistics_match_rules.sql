-- 0054: Logistics match rules

create table if not exists logistics_match_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  enabled boolean not null default true,
  priority int not null default 0,
  product_id uuid references logistics_products(product_id),
  conditions_json jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_logistics_match_rules_tenant_priority
  on logistics_match_rules(tenant_id, priority desc, updated_at desc);

create table if not exists logistics_match_hits (
  hit_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  rule_id uuid references logistics_match_rules(rule_id),
  shipment_id uuid,
  order_id uuid,
  product_id uuid references logistics_products(product_id),
  hit_reason text,
  result_json jsonb,
  success boolean not null default true,
  created_at timestamptz not null default now()
);

create index if not exists idx_logistics_match_hits_tenant_time
  on logistics_match_hits(tenant_id, created_at desc);
create index if not exists idx_logistics_match_hits_rule
  on logistics_match_hits(rule_id, created_at desc);
create index if not exists idx_logistics_match_hits_shipment
  on logistics_match_hits(shipment_id);
