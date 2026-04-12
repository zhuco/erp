-- 0049: CRM lite (customers, categories, member levels, tags)
create table if not exists customer_categories (
  category_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  description text,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_customer_categories_tenant
  on customer_categories(tenant_id, updated_at desc);

create table if not exists member_levels (
  level_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  level_rank int not null default 0,
  discount_rate double precision,
  rules text,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_member_levels_tenant
  on member_levels(tenant_id, updated_at desc);

create table if not exists customer_tags (
  tag_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  color text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, name)
);

create index if not exists idx_customer_tags_tenant
  on customer_tags(tenant_id, updated_at desc);

create table if not exists customers (
  customer_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  phone text,
  address text,
  note text,
  status text not null default 'ACTIVE',
  category_id uuid references customer_categories(category_id) on delete set null,
  member_level_id uuid references member_levels(level_id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists uq_customers_phone
  on customers(tenant_id, phone)
  where phone is not null;

create index if not exists idx_customers_tenant_updated
  on customers(tenant_id, updated_at desc);

create index if not exists idx_customers_tenant_status
  on customers(tenant_id, status);

create index if not exists idx_customers_tenant_category
  on customers(tenant_id, category_id);

create index if not exists idx_customers_tenant_member_level
  on customers(tenant_id, member_level_id);

create table if not exists customer_tag_links (
  tenant_id uuid not null references tenants(tenant_id),
  customer_id uuid not null references customers(customer_id) on delete cascade,
  tag_id uuid not null references customer_tags(tag_id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (tenant_id, customer_id, tag_id)
);

create index if not exists idx_customer_tag_links_tag
  on customer_tag_links(tenant_id, tag_id);

alter table orders
  add column if not exists customer_id uuid;

create index if not exists idx_orders_customer_id
  on orders(tenant_id, customer_id);
