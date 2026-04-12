-- 0052: Channel field mappings (platform field mapping)

create table if not exists channel_field_mappings (
  mapping_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  platform text not null,
  entity_type text not null,
  platform_field text not null,
  internal_field text not null,
  default_value text,
  transform_rule jsonb not null default '{}'::jsonb,
  enabled boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, platform, entity_type, platform_field)
);

create index if not exists idx_channel_field_mappings_tenant_platform
  on channel_field_mappings(tenant_id, platform, entity_type, enabled, updated_at desc);
