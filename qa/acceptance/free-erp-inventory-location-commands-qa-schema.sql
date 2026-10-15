-- Only the missing real location table and its disabled field, from 0031/0058.
\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected location command QA identity required';
 end if;
end $$;
create table if not exists warehouse_locations (
  location_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  code text not null,
  name text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, warehouse_id, code)
);
create index if not exists idx_warehouse_locations_tenant_warehouse
  on warehouse_locations(tenant_id, warehouse_id, code);
alter table warehouse_locations
  add column if not exists is_disabled boolean not null default false;
commit;
