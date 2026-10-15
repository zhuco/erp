-- A location id stays stable when the node moves. A move does not write a stock transfer.

create table if not exists shared_locations (
  tenant_id uuid not null,
  location_id uuid not null,
  parent_id uuid,
  code text not null,
  purpose text not null,
  is_default boolean not null default false,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, location_id)
);

create table if not exists shared_location_refs (
  tenant_id uuid not null,
  location_id uuid not null,
  document_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, location_id, document_id)
);

create table if not exists shared_location_stock (
  tenant_id uuid not null,
  location_id uuid not null,
  sku_id uuid not null,
  on_hand bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, location_id, sku_id)
);
