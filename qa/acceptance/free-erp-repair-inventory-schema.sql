-- Protected QA fixture prerequisite, not a full production migration acceptance.
-- Only the missing reservation table is added; no existing data is changed.
begin;
do $$
begin
  if current_database() <> 'erp_shared_contract_test'
     or current_user <> 'erp_grid_test'
     or inet_server_port() <> 15438 then
    raise exception 'refusing a database outside the protected QA identity';
  end if;
end $$;

-- Same reservation contract as 0005_inventory_schema.sql.
create table if not exists inventory_reservations (
  reservation_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  order_id uuid,
  order_item_id uuid,
  qty bigint not null,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, order_item_id)
);
commit;
