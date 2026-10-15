-- Dropship records ownership and a carrier. It does not post our on-hand inbound.
-- Consignment stock is available only for the named owner. A kit expand is not a BOM.

create table if not exists shared_dropships (
  tenant_id uuid not null,
  dropship_id uuid not null,
  supplier_code text not null,
  customer_code text not null,
  location_code text not null,
  item_code text not null,
  qty_micros bigint not null,
  carrier_ref text not null,
  our_inbound boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, dropship_id)
);

create table if not exists shared_consignment_stock (
  tenant_id uuid not null,
  stock_id uuid not null,
  owner_code text not null,
  location_code text not null,
  item_code text not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, stock_id)
);

create table if not exists shared_consignment_settlements (
  tenant_id uuid not null,
  settlement_id uuid not null,
  stock_id uuid not null,
  owner_code text not null,
  location_code text not null,
  sold_micros bigint not null,
  fee_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, settlement_id)
);

create table if not exists shared_trade_kits (
  tenant_id uuid not null,
  kit_id uuid not null,
  kit_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, kit_id),
  unique (tenant_id, kit_code)
);

create table if not exists shared_trade_kit_parts (
  tenant_id uuid not null,
  kit_id uuid not null,
  item_code text not null,
  qty_per_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, kit_id, item_code)
);

create table if not exists shared_trade_kit_expands (
  tenant_id uuid not null,
  expand_id uuid not null,
  kit_id uuid not null,
  kit_qty_micros bigint not null,
  component_code text not null,
  qty_micros bigint not null,
  production boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, expand_id, component_code)
);
