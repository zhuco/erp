-- Additive prerequisites for the protected QA database only. Sources:
-- 0004_product_sku_schema, 0006_kit_bom + 0029_product_center,
-- 0027_sku_cost, 0030_system_management, 0054_product_module_enhancements,
-- and 0001_init's original outbox for read-side effect auditing.
-- This is not acceptance of the production migration chain. Existing QA SKU
-- rows are not backfilled; product_id stays nullable for those old fixtures.
\set ON_ERROR_STOP on
begin;
do $$
begin
  if current_database() <> 'erp_shared_contract_test'
     or current_user <> 'erp_grid_test'
     or inet_server_addr() <> '127.0.0.1'::inet
     or inet_server_port() <> 15438 then
    raise exception 'inventory read QA database admission rejected';
  end if;
end $$;
create table if not exists products (
  product_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null, brand text, category text,
  attrs jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
alter table skus add column if not exists product_id uuid references products(product_id);
alter table skus add column if not exists barcode text;
alter table skus add column if not exists cost_cent bigint;
alter table skus add column if not exists tags text[] not null default '{}';
create table if not exists sku_boms (
  tenant_id uuid not null references tenants(tenant_id),
  bundle_sku_id uuid not null references skus(sku_id),
  component_sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  bom_version int not null default 1,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, bundle_sku_id, component_sku_id)
);
create table if not exists settings (
  setting_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  scope_type text not null, scope_id uuid, key text not null,
  value_json jsonb not null default '{}'::jsonb,
  version int not null default 1,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create unique index if not exists idx_settings_unique on settings(
  tenant_id, scope_type,
  coalesce(scope_id, '00000000-0000-0000-0000-000000000000'::uuid), key
);
create index if not exists idx_settings_tenant_key on settings(tenant_id, key);
create index if not exists idx_settings_tenant_scope on settings(tenant_id, scope_type, scope_id);
create table if not exists outbox_events (
  tenant_id uuid not null references tenants(tenant_id),
  event_month date not null,
  outbox_id uuid not null default gen_random_uuid(),
  event_type text not null, payload jsonb not null,
  status text not null default 'PENDING', try_count int not null default 0,
  next_retry_at timestamptz not null default now(), last_error text not null default '',
  created_at timestamptz not null default now(), updated_at timestamptz not null default now(),
  primary key (tenant_id, event_month, outbox_id)
) partition by range(event_month);
create table if not exists outbox_events_default partition of outbox_events default;
create index if not exists idx_outbox_pending on outbox_events(status, next_retry_at);
create index if not exists idx_outbox_by_id on outbox_events(tenant_id, outbox_id);
commit;
