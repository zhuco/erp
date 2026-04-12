-- 0003_after_sale_cases.sql
-- After-sale case aggregation (case-level projection)

create table if not exists after_sale_cases (
  tenant_id uuid not null references tenants(tenant_id),
  case_id uuid not null default gen_random_uuid(),
  platform text not null,
  shop_id uuid not null references shops(shop_id),
  after_sale_id text not null,
  after_sale_type text,
  refund_state text not null default 'NONE',
  after_sale_state text not null default 'NONE',
  refund_amount_cent bigint not null default 0,
  item_count int not null default 0,
  total_qty int not null default 0,
  order_count int not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, case_id),
  unique (tenant_id, platform, shop_id, after_sale_id)
);

create index if not exists idx_after_sale_cases_tenant_updated
  on after_sale_cases(tenant_id, updated_at desc);
create index if not exists idx_after_sale_cases_tenant_platform
  on after_sale_cases(tenant_id, platform);
create index if not exists idx_after_sale_cases_shop
  on after_sale_cases(tenant_id, shop_id);

create table if not exists after_sale_case_items (
  tenant_id uuid not null references tenants(tenant_id),
  case_id uuid not null,
  order_id uuid not null,
  order_month date not null,
  line_id text not null,
  sku_id text,
  title text,
  qty int not null default 1,
  refund_state text not null default 'NONE',
  after_sale_state text not null default 'NONE',
  refund_amount_cent bigint,
  after_sale_type text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, case_id, order_id, line_id),
  foreign key (tenant_id, case_id) references after_sale_cases(tenant_id, case_id) on delete cascade
);

create index if not exists idx_after_sale_case_items_order
  on after_sale_case_items(tenant_id, order_id);
create index if not exists idx_after_sale_case_items_case
  on after_sale_case_items(tenant_id, case_id);
