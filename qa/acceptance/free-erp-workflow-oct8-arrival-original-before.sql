\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected production release QA identity required';
 end if;
end $$;
do $$ declare name text; begin foreach name in array array['inventory_balances','inventory_ledgers','inventory_reservations','inventory_order_reservation_sources','inventory_order_reservation_components','inventory_quantity_activations','inventory_quantity_source_maps','shared_stock_ledgers','inventory_batch_balances','sku_batches','inventory_location_reservation_events','command_objects','browser_sessions','atomic_source_documents','atomic_posting_events','command_receipts','orders','order_items','order_index','shipments','shipment_items','order_cogs','warehouse_rules','outbox_events','operation_logs','audit_logs','tenants','shops','users','skus','warehouses','user_warehouse_access','feature_flags','sku_boms','order_item_components','channel_sku_mappings','inventory_batch_movements','mfg_material_moves','mfg_qty_ledgers','mfg_qty_balances','mfg_work_orders','mfg_receipts','mfg_reports','shared_purchase_revisions','shared_purchase_lines','mfg_exceptions','mfg_inspections','mfg_quality_items','shared_inspection_freezes','shared_inspection_freeze_lines','mfg_receipt_release_commands','mfg_receipt_release_effects','shared_inspection_templates','shared_inspection_template_lines','shared_inspection_source_bindings','shared_sales_revisions','shared_sales_lines','shared_sales_deliveries','shared_material_issues','mfg_quality_releases','mfg_quality_release_proofs','shared_inspection_reading_events','shared_inspection_reading_reviews','atomic_material_values','shared_mfg_bom_versions','shared_mfg_bom_version_lines','mfg_boms','mfg_bom_items','shared_purchase_receipts','shared_receipt_quality','shared_supplier_returns','partner_balances','shared_receipt_value_actors','shared_receipt_value_commands','shared_receipt_cost_lots','shared_receipt_cost_allocations','roles','user_roles','role_permissions','auth_sessions'] loop if to_regclass(format('public.%I',name)) is null then raise exception 'required snapshot table missing: %',name; end if; end loop; end $$;
select jsonb_build_object('selected_tables',80,'database',current_database(),'role',current_user,'host',inet_server_addr(),
 'port',inet_server_port(),'catalog',coalesce((select jsonb_agg(jsonb_build_object(
 'table',table_name,'column',column_name,'type',data_type,'nullable',is_nullable,'default',column_default)
 order by table_name,ordinal_position) from information_schema.columns where table_schema='public'
 and table_name in('inventory_balances','inventory_ledgers','inventory_batch_balances','sku_batches',
 'inventory_batch_movements','mfg_material_moves','mfg_qty_ledgers','mfg_qty_balances','mfg_work_orders',
 'mfg_receipts','mfg_reports','shared_purchase_revisions','shared_purchase_lines','mfg_exceptions','mfg_inspections','mfg_quality_items','shared_inspection_freezes',
 'shared_inspection_freeze_lines','mfg_receipt_release_commands','mfg_receipt_release_effects',
 'shared_inspection_templates','shared_inspection_template_lines','shared_inspection_source_bindings',
 'mfg_quality_releases','mfg_quality_release_proofs','shared_inspection_reading_events','shared_inspection_reading_reviews','atomic_material_values','shared_mfg_bom_versions','shared_mfg_bom_version_lines','mfg_boms','mfg_bom_items','shared_purchase_receipts','shared_receipt_quality','shared_supplier_returns','partner_balances','shared_receipt_value_actors','shared_receipt_value_commands','shared_receipt_cost_lots','shared_receipt_cost_allocations','roles','user_roles','role_permissions','auth_sessions')),'[]'::jsonb),
 'batch_movements_present',to_regclass('public.inventory_batch_movements') is not null, 'release_commands_present',to_regclass('public.mfg_receipt_release_commands') is not null, 'inspection_bindings_present',to_regclass('public.shared_inspection_source_bindings') is not null);
select format('select jsonb_build_object(''tenant_id'',t.tenant_id,''name'',t.name,
 ''facts'',(select jsonb_object_agg(p.name,p.value) from (values %s) p(name,value))) from tenants t order by t.tenant_id',
 string_agg(format('(''%s'', %s)',name,case when to_regclass(format('public.%I',name)) is null
 then '''[]''::jsonb' else format('coalesce((select jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text)
 from public.%I x where x.tenant_id=t.tenant_id),''[]''::jsonb)',name) end),',' order by name))
 from unnest(array[
 'inventory_balances','inventory_ledgers','inventory_reservations','inventory_order_reservation_sources',
 'inventory_order_reservation_components','inventory_quantity_activations','inventory_quantity_source_maps',
 'shared_stock_ledgers','inventory_batch_balances','sku_batches','inventory_location_reservation_events',
 'command_objects','browser_sessions','atomic_source_documents','atomic_posting_events','command_receipts','orders','order_items','order_index','shipments','shipment_items','order_cogs','warehouse_rules',
 'outbox_events','operation_logs','audit_logs','tenants','shops','users','skus','warehouses',
 'user_warehouse_access','feature_flags','sku_boms','order_item_components','channel_sku_mappings',
 'inventory_batch_movements','mfg_material_moves','mfg_qty_ledgers','mfg_qty_balances','mfg_work_orders','mfg_receipts','mfg_reports','shared_purchase_revisions','shared_purchase_lines','mfg_exceptions','mfg_inspections','mfg_quality_items','shared_inspection_freezes','shared_inspection_freeze_lines','mfg_receipt_release_commands','mfg_receipt_release_effects','shared_inspection_templates','shared_inspection_template_lines','shared_inspection_source_bindings','shared_sales_revisions','shared_sales_lines','shared_sales_deliveries','shared_material_issues','mfg_quality_releases','mfg_quality_release_proofs','shared_inspection_reading_events','shared_inspection_reading_reviews','atomic_material_values','shared_mfg_bom_versions','shared_mfg_bom_version_lines','mfg_boms','mfg_bom_items','shared_purchase_receipts','shared_receipt_quality','shared_supplier_returns','partner_balances','shared_receipt_value_actors','shared_receipt_value_commands','shared_receipt_cost_lots','shared_receipt_cost_allocations','roles','user_roles','role_permissions','auth_sessions']) name
\gexec
commit;
