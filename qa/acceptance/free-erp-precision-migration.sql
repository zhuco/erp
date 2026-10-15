-- Additive protected QA schema application. No historical balance is rewritten.
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
-- HTTP acceptance uses the actual feature-flag schema and normal defaults.
select to_regclass('feature_flags') is null as needs_inventory_feature_flags \gset
\if :needs_inventory_feature_flags
\i migrations/0015_feature_flags.sql
\endif
-- Missing fixture prerequisites, exactly as 0006/0029 define them.
alter table skus add column if not exists sku_kind text not null default 'NORMAL';
alter table skus add column if not exists batch_enabled boolean not null default false;
create table if not exists order_item_components (
 tenant_id uuid not null references tenants(tenant_id),
 order_id uuid not null,
 order_item_id uuid not null,
 bundle_sku_id uuid not null references skus(sku_id),
 component_sku_id uuid not null references skus(sku_id),
 component_qty bigint not null,
 bom_version int,
 bom_snapshot jsonb not null,
 created_at timestamptz not null default now(),
 primary key(tenant_id,order_item_id,component_sku_id)
);
\i migrations/0283_inventory_precision.sql
commit;
