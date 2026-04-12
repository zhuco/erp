-- 0020: OMS order review/status/tags/notes/blacklist

-- ======================
-- Orders: OMS extensions
-- ======================
alter table orders
  add column if not exists oms_status text not null default 'NEW',
  add column if not exists review_status text not null default 'PENDING',
  add column if not exists exception_flags jsonb not null default '[]'::jsonb,
  add column if not exists version int not null default 0;

create index if not exists idx_orders_tenant_created
  on orders(tenant_id, order_created_at desc);
create index if not exists idx_orders_review_status
  on orders(tenant_id, review_status);
create index if not exists idx_orders_oms_status
  on orders(tenant_id, oms_status);
create index if not exists idx_orders_exception_flags
  on orders using gin (exception_flags);

-- =====================
-- Review rules & hits
-- =====================
create table if not exists order_review_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  enabled boolean not null default true,
  priority int not null default 0,
  conditions_json jsonb not null default '[]'::jsonb,
  action text not null,
  reason text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_order_review_rules_tenant_priority
  on order_review_rules(tenant_id, enabled, priority desc);

create table if not exists order_review_hits (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  rule_id uuid not null references order_review_rules(rule_id),
  hit boolean not null default false,
  message text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, order_id, rule_id)
);

create index if not exists idx_order_review_hits_order_rule
  on order_review_hits(tenant_id, order_id, rule_id);

-- ==================
-- Tags and notes
-- ==================
create table if not exists order_tags (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  tag text not null,
  created_by uuid,
  created_at timestamptz not null default now(),
  primary key (tenant_id, order_id, tag)
);

create index if not exists idx_order_tags_order
  on order_tags(tenant_id, order_id);

create table if not exists order_notes (
  note_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  note text not null,
  created_by uuid,
  created_at timestamptz not null default now()
);

create index if not exists idx_order_notes_order
  on order_notes(tenant_id, order_id, created_at desc);

-- =======================
-- Customer blacklist
-- =======================
create table if not exists customer_blacklist (
  blacklist_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  key_type text not null,
  key_value text not null,
  reason text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, key_type, key_value)
);

create index if not exists idx_customer_blacklist_key
  on customer_blacklist(tenant_id, key_type, key_value);

-- ==========================
-- Exception reason dictionary
-- ==========================
create table if not exists exception_reasons (
  tenant_id uuid not null references tenants(tenant_id),
  code text not null,
  label text not null,
  category text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, code)
);

create index if not exists idx_exception_reasons_tenant_category
  on exception_reasons(tenant_id, category);

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'MAPPING_MISSING', 'SKU映射缺失', 'MAPPING'
from tenants
on conflict (tenant_id, code) do nothing;

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'INVENTORY_SHORTAGE', '库存不足', 'INVENTORY'
from tenants
on conflict (tenant_id, code) do nothing;

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'ADDRESS_INVALID', '地址异常', 'ADDRESS'
from tenants
on conflict (tenant_id, code) do nothing;

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'DUPLICATE_ORDER', '疑似重复单', 'RISK'
from tenants
on conflict (tenant_id, code) do nothing;

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'BLACKLISTED_CUSTOMER', '黑名单客户', 'RISK'
from tenants
on conflict (tenant_id, code) do nothing;

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'REVIEW_REJECTED', '审单拒绝', 'REVIEW'
from tenants
on conflict (tenant_id, code) do nothing;

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'REVIEW_FLAGGED', '审单拦截', 'REVIEW'
from tenants
on conflict (tenant_id, code) do nothing;
