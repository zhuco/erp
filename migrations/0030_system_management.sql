-- 0030: System management (audit logs, settings, data scopes, order reviews, trace id)

alter table if exists outbox_events
  add column if not exists trace_id text;

create table if not exists audit_logs (
  audit_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid,
  action text not null,
  entity_type text not null,
  entity_id text,
  before_json jsonb,
  after_json jsonb,
  ip text,
  ua text,
  created_at timestamptz not null default now()
);

create index if not exists idx_audit_logs_tenant_created
  on audit_logs(tenant_id, created_at desc, audit_id desc);
create index if not exists idx_audit_logs_tenant_action
  on audit_logs(tenant_id, action);
create index if not exists idx_audit_logs_tenant_entity
  on audit_logs(tenant_id, entity_type, entity_id);
create index if not exists idx_audit_logs_tenant_user
  on audit_logs(tenant_id, user_id, created_at desc);

create table if not exists settings (
  setting_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  scope_type text not null, -- GLOBAL/TENANT/SHOP/WAREHOUSE
  scope_id uuid,
  key text not null,
  value_json jsonb not null default '{}'::jsonb,
  version int not null default 1,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists idx_settings_unique
  on settings(
    tenant_id,
    scope_type,
    coalesce(scope_id, '00000000-0000-0000-0000-000000000000'::uuid),
    key
  );
create index if not exists idx_settings_tenant_key
  on settings(tenant_id, key);
create index if not exists idx_settings_tenant_scope
  on settings(tenant_id, scope_type, scope_id);

create table if not exists user_shop_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  shop_id uuid not null references shops(shop_id),
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, shop_id)
);

create index if not exists idx_user_shop_access_user
  on user_shop_access(tenant_id, user_id);

create table if not exists user_warehouse_access (
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id, warehouse_id)
);

create index if not exists idx_user_warehouse_access_user
  on user_warehouse_access(tenant_id, user_id);

create table if not exists order_reviews (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  status text not null default 'PENDING', -- PENDING/APPROVED/REJECTED
  reviewed_by uuid,
  reviewed_at timestamptz,
  notes text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, order_id)
);

create index if not exists idx_order_reviews_tenant_status
  on order_reviews(tenant_id, status, updated_at desc);

create index if not exists idx_dead_letters_tenant_status
  on dead_letters(tenant_id, status, created_at desc);
