-- 0085: Sales orders (internal business orders)

create table if not exists sales_orders (
  sales_order_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid references shops(shop_id),
  sales_order_no text not null,
  source_type text not null,
  source_order_ids jsonb not null default '[]'::jsonb,
  customer_id uuid references customers(customer_id),
  currency_code text not null default 'CNY',
  tax_mode text,
  sales_order_status text not null default 'DRAFT',
  delivery_status text not null default 'NOT_SHIPPED',
  invoice_status text not null default 'NOT_INVOICED',
  payment_status text not null default 'UNPAID',
  refund_status text not null default 'NONE',
  return_status text,
  stock_status text,
  total_amount_cent bigint not null default 0,
  discount_amount_cent bigint not null default 0,
  shipping_amount_cent bigint not null default 0,
  receivable_amount_cent bigint not null default 0,
  paid_amount_cent bigint not null default 0,
  refunded_amount_cent bigint not null default 0,
  invoice_amount_cent bigint not null default 0,
  status_flags jsonb not null default '[]'::jsonb,
  remark text,
  version int not null default 1,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  created_by uuid references users(user_id),
  updated_by uuid references users(user_id),
  receiver_name text,
  receiver_phone text,
  receiver_address text,
  receiver_province text,
  receiver_city text,
  receiver_district text,
  receiver_postcode text,
  carrier_code text,
  logistics_service text,
  expected_ship_at timestamptz,
  wms_outbound_id uuid references outbound_docs(outbound_doc_id),
  wms_outbound_status text,
  wms_wave_id uuid references waves(wave_id),
  wms_wave_status text,
  wms_ship_status text,
  package_count int not null default 0,
  waybill_nos jsonb not null default '[]'::jsonb,
  review_status text,
  fulfillment_status text,
  exception_status text,
  freeze_reason text,
  exception_flags jsonb not null default '[]'::jsonb,
  unique (tenant_id, sales_order_no)
);

create index if not exists idx_sales_orders_tenant_status
  on sales_orders(tenant_id, sales_order_status, updated_at desc, sales_order_id);
create index if not exists idx_sales_orders_tenant_created
  on sales_orders(tenant_id, created_at desc, sales_order_id);
create index if not exists idx_sales_orders_tenant_shop
  on sales_orders(tenant_id, shop_id);
create index if not exists idx_sales_orders_wms_outbound
  on sales_orders(tenant_id, wms_outbound_id);

create table if not exists sales_order_items (
  tenant_id uuid not null references tenants(tenant_id),
  sales_order_id uuid not null references sales_orders(sales_order_id) on delete cascade,
  line_id uuid not null default gen_random_uuid(),
  sku_id uuid references skus(sku_id),
  sku_code text,
  sku_name text,
  qty int not null,
  price_cent bigint not null default 0,
  tax_rate double precision,
  discount_cent bigint not null default 0,
  amount_cent bigint not null default 0,
  warehouse_id uuid references warehouses(warehouse_id),
  delivery_deadline timestamptz,
  extra jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, sales_order_id, line_id)
);

create index if not exists idx_sales_order_items_tenant_order
  on sales_order_items(tenant_id, sales_order_id);
create index if not exists idx_sales_order_items_tenant_sku
  on sales_order_items(tenant_id, sku_id);
