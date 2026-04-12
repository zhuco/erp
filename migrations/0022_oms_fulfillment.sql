-- 0022: OMS fulfillment (warehouse rules, shipment items, reservation status)

alter table inventory_reservations
  alter column status set default 'RESERVED';

update inventory_reservations
set status = 'RESERVED'
where status = 'ACTIVE';

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

alter table orders
  add column if not exists inventory_shortage boolean not null default false;
alter table orders
  add column if not exists allocated_at timestamptz;

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
