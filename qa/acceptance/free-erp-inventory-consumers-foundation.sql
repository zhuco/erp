-- Add only the original 0001 order tables missing from the protected QA database.
-- No seed, reset, backfill, business migration or existing data update.
\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory consumer QA identity required';
 end if;
end $$;
create table if not exists orders (
 tenant_id uuid not null references tenants(tenant_id), order_month date not null,
 order_id uuid not null, platform text not null, shop_id uuid not null references shops(shop_id),
 platform_order_id text not null, status text not null, total_amount_cent bigint not null default 0,
 currency text not null default 'CNY', buyer_name text, receiver_name text, receiver_phone text,
 receiver_address text, order_created_at timestamptz not null, platform_updated_at timestamptz,
 ingested_at timestamptz not null default now(), updated_at timestamptz not null default now(),
 raw jsonb not null default '{}'::jsonb,
 primary key (tenant_id,order_month,order_id), unique(tenant_id,order_month,platform,platform_order_id)
) partition by range(order_month);
create table if not exists orders_default partition of orders default;
create index if not exists idx_orders_shop_time on orders(tenant_id,shop_id,order_created_at desc);
create index if not exists idx_orders_platform_time on orders(tenant_id,platform,order_created_at desc);
create table if not exists order_items (
 tenant_id uuid not null references tenants(tenant_id), order_month date not null, order_id uuid not null,
 line_id text not null, sku_id text, title text, qty int not null default 1,
 price_cent bigint not null default 0, amount_cent bigint not null default 0,
 raw jsonb not null default '{}'::jsonb, created_at timestamptz not null default now(),
 updated_at timestamptz not null default now(), primary key(tenant_id,order_month,order_id,line_id)
) partition by range(order_month);
create table if not exists order_items_default partition of order_items default;
create index if not exists idx_order_items_order on order_items(tenant_id,order_id);
commit;
