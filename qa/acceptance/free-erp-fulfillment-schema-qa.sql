\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected fulfillment QA identity required'; end if;
 if to_regclass('public.order_index') is not null or to_regclass('public.shipments') is not null
  or to_regclass('public.shipment_items') is not null or to_regclass('public.warehouse_rules') is not null
  or to_regclass('public.order_cogs') is not null then raise exception 'initial QA schema no longer matches'; end if;
end $$;
-- Nullable compatibility columns preserve every historical value as NULL.
-- New test orders explicitly supply their real initial lifecycle and flags.
alter table orders add column inventory_shortage boolean, add column allocated_at timestamptz,
 add column order_status text, add column fulfillment_status text, add column exception_status text,
 add column exception_flags jsonb, add column ship_at timestamptz;
alter table skus add column weight_gram integer;
alter table warehouses add column is_default boolean, add column updated_at timestamptz;
create table if not exists order_index (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  order_month date not null,
  platform text not null,
  platform_order_id text not null,
  shop_id uuid not null references shops(shop_id),
  order_created_at timestamptz not null,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, order_id)
) partition by hash (order_id);

do $$
declare i int;
begin
  for i in 0..31 loop
    execute format('create table if not exists order_index_p%1$s partition of order_index for values with (modulus 32, remainder %1$s);', i);
  end loop;
end $$;

create index if not exists idx_order_index_platform_order on order_index(tenant_id, platform, platform_order_id);


-- 0007: Shipments + Waybills

create table if not exists shipments (
  shipment_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  order_id uuid not null,
  platform text not null,
  platform_order_id text not null,
  warehouse_id uuid references warehouses(warehouse_id),
  carrier_code text,
  tracking_no text,
  status text not null default 'REQUESTED',
  requested_at timestamptz,
  shipped_at timestamptz,
  updated_at timestamptz not null default now(),
  unique (tenant_id, order_id)
);

create index if not exists idx_shipments_tenant_shop_status
  on shipments(tenant_id, shop_id, status);
create index if not exists idx_shipments_tenant_platform_order
  on shipments(tenant_id, platform, platform_order_id);

create table if not exists waybills (
  waybill_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  platform text not null,
  carrier_code text not null,
  tracking_no text not null,
  source text not null default 'PLATFORM',
  status text not null default 'ALLOCATED',
  allocated_at timestamptz not null default now(),
  used_at timestamptz,
  unique (tenant_id, tracking_no)
);

create table if not exists warehouse_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  province text,
  city text,
  carrier_code text,
  priority int not null default 0,
  is_active boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_warehouse_rules_tenant_priority
  on warehouse_rules(tenant_id, priority desc);
create index if not exists idx_warehouse_rules_tenant_location
  on warehouse_rules(tenant_id, province, city);
create index if not exists idx_warehouse_rules_tenant_carrier
  on warehouse_rules(tenant_id, carrier_code);


alter table shipments
  add column if not exists shipment_type text not null default 'PLATFORM';
alter table shipments
  add column if not exists shipment_no text;
alter table shipments
  add column if not exists weight_gram int;
alter table shipments
  add column if not exists created_at timestamptz not null default now();
alter table shipments
  add column if not exists last_error text not null default '';

alter table shipments drop constraint if exists shipments_tenant_id_order_id_key;

create index if not exists idx_shipments_tenant_type_status
  on shipments(tenant_id, shipment_type, status);
create index if not exists idx_shipments_tenant_order
  on shipments(tenant_id, order_id);
create unique index if not exists idx_shipments_platform_order_unique
  on shipments(tenant_id, order_id)
  where shipment_type = 'PLATFORM';
create unique index if not exists idx_shipments_tenant_shipment_no
  on shipments(tenant_id, shipment_no)
  where shipment_no is not null;

create table if not exists shipment_items (
  shipment_item_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shipment_id uuid not null references shipments(shipment_id),
  order_id uuid not null,
  order_item_id uuid not null,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  created_at timestamptz not null default now(),
  unique (tenant_id, shipment_id, order_item_id)
);

create index if not exists idx_shipment_items_tenant_shipment
  on shipment_items(tenant_id, shipment_id);
create index if not exists idx_shipment_items_tenant_order
  on shipment_items(tenant_id, order_id);
create index if not exists idx_shipment_items_tenant_sku
  on shipment_items(tenant_id, sku_id);

-- 0039: OMS resend + cancel request

alter table shipments
  add column if not exists origin_shipment_id uuid;

create index if not exists idx_shipments_tenant_origin
  on shipments(tenant_id, origin_shipment_id);

create table if not exists shipment_cancel_requests (
  cancel_request_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shipment_id uuid not null references shipments(shipment_id),
  order_id uuid not null,
  status text not null default 'PENDING',
  reason text,
  requested_by uuid,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists idx_shipment_cancel_requests_unique
  on shipment_cancel_requests(tenant_id, shipment_id);
create index if not exists idx_shipment_cancel_requests_tenant_status
  on shipment_cancel_requests(tenant_id, status, created_at desc);

create table if not exists order_cogs (
  ledger_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  shipment_id uuid,
  outbound_doc_id uuid,
  order_item_id uuid,
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  unit_cost_cent bigint not null,
  total_cost_cent bigint not null,
  ref_type text not null,
  ref_id text not null,
  created_at timestamptz not null default now(),
  unique (tenant_id, ref_type, ref_id, sku_id)
);

create index if not exists idx_order_cogs_tenant_order
  on order_cogs(tenant_id, order_id);
create index if not exists idx_order_cogs_tenant_shipment
  on order_cogs(tenant_id, shipment_id);


-- Status columns on new, empty shipment tables; no old status rewrite.
alter table shipments add column status_version int not null default 0,
 add column shipment_status text not null default 'REQUESTED';
commit;
