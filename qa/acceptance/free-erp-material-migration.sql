-- Protected test setup; additive schema only, never a production upgrade claim.
begin;
do $$ begin
  if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
    or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
    raise exception 'Protected QA identity required';
  end if;
end $$;
-- Identical missing fixture prerequisite from 0029_product_center.sql.
create table if not exists sku_batches (
  batch_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  sku_id uuid not null references skus(sku_id),
  batch_no text not null,
  expire_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id,sku_id,batch_no)
);
create table if not exists inventory_batch_balances (
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  batch_id uuid not null references sku_batches(batch_id),
  on_hand bigint not null default 0,
  reserved bigint not null default 0,
  updated_at timestamptz not null default now(),
  primary key(tenant_id,warehouse_id,sku_id,batch_id)
);
\i migrations/0282_material_identity.sql
commit;
