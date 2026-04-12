-- 0050: WMS base data (areas/locations/containers/senders/workflow helpers)

create table if not exists warehouse_areas (
  area_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  code text not null,
  name text not null,
  area_type text not null default 'STORAGE',
  status text not null default 'ACTIVE',
  priority int not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, warehouse_id, code)
);

create index if not exists idx_warehouse_areas_tenant_warehouse
  on warehouse_areas(tenant_id, warehouse_id, code);

alter table warehouse_locations
  add column if not exists area_id uuid references warehouse_areas(area_id);
alter table warehouse_locations
  add column if not exists barcode text;
alter table warehouse_locations
  add column if not exists volume double precision;
alter table warehouse_locations
  add column if not exists max_weight double precision;
alter table warehouse_locations
  add column if not exists is_picking boolean not null default false;
alter table warehouse_locations
  add column if not exists is_disabled boolean not null default false;

create index if not exists idx_warehouse_locations_tenant_area
  on warehouse_locations(tenant_id, warehouse_id, area_id);

create table if not exists containers (
  container_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  code text not null,
  container_type text not null,
  capacity double precision,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, code)
);

create index if not exists idx_containers_tenant_updated
  on containers(tenant_id, updated_at desc);

create table if not exists sender_addresses (
  sender_address_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  phone text not null,
  province text not null,
  city text not null,
  district text not null,
  address text not null,
  is_default boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_sender_addresses_tenant_updated
  on sender_addresses(tenant_id, updated_at desc);

alter table outbound_pick_tasks
  add column if not exists container_id uuid references containers(container_id);
