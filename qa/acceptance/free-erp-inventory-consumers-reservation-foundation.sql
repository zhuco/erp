-- Original 0029 order-component schema, only on the protected QA identity.
\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory consumer QA identity required';
 end if;
end $$;
create table if not exists order_item_components (
 tenant_id uuid not null references tenants(tenant_id), order_id uuid not null,
 order_item_id uuid not null, bundle_sku_id uuid not null references skus(sku_id),
 component_sku_id uuid not null references skus(sku_id), component_qty bigint not null,
 bom_version int, bom_snapshot jsonb not null,
 created_at timestamptz not null default now(),
 primary key(tenant_id,order_item_id,component_sku_id)
);
create index if not exists idx_order_item_components_order on order_item_components(tenant_id,order_id);
commit;
