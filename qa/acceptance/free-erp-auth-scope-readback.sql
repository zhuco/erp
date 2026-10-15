-- Independent readback for the 49-case authorization regression artifact.
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
select jsonb_pretty(jsonb_build_object(
 'database',current_database(),'role',current_user,
 'host',host(inet_server_addr()),'port',inet_server_port(),
 'tenant','0caf501b-c967-4a44-bedf-0cbbdfd2e36e',
 'user','c48c6dde-9ecf-465c-a4d5-685ff7c35789',
 'grants',(select jsonb_agg(warehouse_id order by warehouse_id) from public.user_warehouse_access
   where tenant_id='0caf501b-c967-4a44-bedf-0cbbdfd2e36e' and user_id='c48c6dde-9ecf-465c-a4d5-685ff7c35789'),
 'warehouses',(select count(*) from public.warehouses where tenant_id='0caf501b-c967-4a44-bedf-0cbbdfd2e36e'),
 'stock',(select jsonb_agg(jsonb_build_object('warehouse_id',warehouse_id,'sku_id',sku_id,
   'on_hand',on_hand,'reserved',reserved,'frozen',frozen)) from public.inventory_balances
   where tenant_id='0caf501b-c967-4a44-bedf-0cbbdfd2e36e'),
 'ledgers',(select count(*) from public.inventory_ledgers where tenant_id='0caf501b-c967-4a44-bedf-0cbbdfd2e36e'),
 'ledger_sum',(select sum(delta_on_hand) from public.inventory_ledgers where tenant_id='0caf501b-c967-4a44-bedf-0cbbdfd2e36e'),
 'policy_views',(select count(*) from information_schema.views
   where table_schema='qa_scope_policy_ab3f93e54a214323bdfce666e3b775d4'),
 'policy_tenant_columns',(select jsonb_agg(column_name order by ordinal_position) from information_schema.columns
   where table_schema='qa_scope_policy_ab3f93e54a214323bdfce666e3b775d4' and table_name='tenants'),
 'policy_scope_views',(select count(*) from information_schema.views
   where table_schema='qa_scope_policy_ab3f93e54a214323bdfce666e3b775d4'
   and table_name in ('user_shop_access','user_warehouse_access','user_merchant_access','user_organization_access')),
 'public_scope_tables',(select count(*) from information_schema.tables where table_schema='public'
   and table_type='BASE TABLE' and table_name in ('user_shop_access','user_warehouse_access','user_merchant_access','user_organization_access')),
 'public_policy_columns',(select count(*) from information_schema.columns where table_schema='public' and table_name='tenants'
   and column_name in ('multi_merchant_enabled','multi_organization_enabled') and data_type='boolean' and is_nullable='NO')
));
commit;
