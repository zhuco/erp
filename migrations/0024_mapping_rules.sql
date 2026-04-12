-- 0024_mapping_rules.sql
-- Field mapping rules for platform payloads

create table if not exists mapping_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null,
  entity_type text not null,
  version int not null,
  enabled boolean not null default true,
  rule_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, platform, entity_type, version)
);

create index if not exists idx_mapping_rules_tenant_platform
  on mapping_rules(tenant_id, platform, entity_type);

create index if not exists idx_mapping_rules_enabled
  on mapping_rules(tenant_id, platform, entity_type, enabled);
