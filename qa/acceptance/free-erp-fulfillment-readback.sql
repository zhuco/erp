\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected fulfillment QA identity required';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,'host',inet_server_addr(),
 'port',inet_server_port(),'catalog',coalesce((select jsonb_agg(jsonb_build_object(
 'table',table_name,'column',column_name,'type',data_type,'nullable',is_nullable,'default',column_default)
 order by table_name,ordinal_position) from information_schema.columns where table_schema='public'
 and table_name in('orders','skus','warehouses','order_index','shipments','shipment_items','order_cogs',
 'warehouse_rules','waybills','shipment_cancel_requests')),'[]'::jsonb));
-- Build one SELECT from a fixed table allowlist. No function, DDL or data write
-- is required; missing prerequisites are visible in the catalog record.
select format('select jsonb_build_object(''tenant_id'',t.tenant_id,''name'',t.name,
 ''facts'',jsonb_build_object(%s)) from tenants t order by t.tenant_id',
 string_agg(format('''%s'', %s',name,case when to_regclass(format('public.%I',name)) is null
 then '''[]''::jsonb' else format('coalesce((select jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text)
 from public.%I x where x.tenant_id=t.tenant_id),''[]''::jsonb)',name) end),',' order by name))
 from unnest(array[
 'inventory_balances','inventory_ledgers','inventory_reservations','inventory_order_reservation_sources',
 'inventory_order_reservation_components','inventory_quantity_activations','inventory_quantity_source_maps',
 'shared_stock_ledgers','inventory_batch_balances','sku_batches','inventory_location_reservation_events',
 'command_receipts','orders','order_items','order_index','shipments','shipment_items','order_cogs','warehouse_rules',
 'outbox_events','operation_logs','audit_logs','tenants','shops','users','skus','warehouses',
 'user_warehouse_access','feature_flags','sku_boms','order_item_components','channel_sku_mappings']) name
\gexec
commit;
