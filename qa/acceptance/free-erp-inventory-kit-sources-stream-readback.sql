-- Independent read-only facts; never modify a fixture or activate historical stock.
\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory consumer QA identity required';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port());
select jsonb_build_object('tenant_id',t.tenant_id,'name',t.name,
  'balances',coalesce((select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from inventory_balances b where b.tenant_id=t.tenant_id),'[]'::jsonb),
  'ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from inventory_ledgers l where l.tenant_id=t.tenant_id),'[]'::jsonb),
  'command_receipts',coalesce((select jsonb_agg(to_jsonb(c) order by request_id) from command_receipts c where c.tenant_id=t.tenant_id),'[]'::jsonb),
  'shared_ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from shared_stock_ledgers l where l.tenant_id=t.tenant_id),'[]'::jsonb),
  'reservation_roots',coalesce((select jsonb_agg(to_jsonb(r) order by root_order_item_id) from inventory_order_reservation_sources r where r.tenant_id=t.tenant_id),'[]'::jsonb),
  'reservation_components',coalesce((select jsonb_agg(to_jsonb(c) order by source_id,sku_id) from inventory_order_reservation_components c where c.tenant_id=t.tenant_id),'[]'::jsonb),
  'sku_boms',coalesce((select jsonb_agg(to_jsonb(b) order by bundle_sku_id,component_sku_id) from sku_boms b where b.tenant_id=t.tenant_id),'[]'::jsonb),
  'order_item_components',coalesce((select jsonb_agg(to_jsonb(c) order by order_item_id,component_sku_id) from order_item_components c where c.tenant_id=t.tenant_id),'[]'::jsonb),
  'reservations',coalesce((select jsonb_agg(to_jsonb(r) order by order_item_id) from inventory_reservations r where r.tenant_id=t.tenant_id),'[]'::jsonb),
  'reservation_events',coalesce((select jsonb_agg(to_jsonb(e) order by request_id) from inventory_location_reservation_events e where e.tenant_id=t.tenant_id),'[]'::jsonb),
  'activations',coalesce((select jsonb_agg(to_jsonb(a) order by warehouse_id,sku_id) from inventory_quantity_activations a where a.tenant_id=t.tenant_id),'[]'::jsonb),
  'source_maps',coalesce((select jsonb_agg(to_jsonb(m) order by to_jsonb(m)::text) from inventory_quantity_source_maps m where m.tenant_id=t.tenant_id),'[]'::jsonb),
  'outbox',coalesce((select jsonb_agg(to_jsonb(o) order by outbox_id) from outbox_events o where o.tenant_id=t.tenant_id),'[]'::jsonb),
  'audits',coalesce((select jsonb_agg(to_jsonb(a) order by audit_id) from audit_logs a where a.tenant_id=t.tenant_id),'[]'::jsonb),
  'operations',coalesce((select jsonb_agg(to_jsonb(o) order by log_id) from operation_logs o where o.tenant_id=t.tenant_id),'[]'::jsonb),
  'orders',coalesce((select jsonb_agg(to_jsonb(o) order by order_month,order_id) from orders o where o.tenant_id=t.tenant_id),'[]'::jsonb),
  'order_items',coalesce((select jsonb_agg(to_jsonb(i) order by order_month,order_id,line_id) from order_items i where i.tenant_id=t.tenant_id),'[]'::jsonb),
  'users',coalesce((select jsonb_agg(to_jsonb(u) order by user_id) from users u where u.tenant_id=t.tenant_id),'[]'::jsonb),
  'grants',coalesce((select jsonb_agg(to_jsonb(g) order by user_id,warehouse_id) from user_warehouse_access g where g.tenant_id=t.tenant_id),'[]'::jsonb),
  'skus',coalesce((select jsonb_agg(to_jsonb(s) order by sku_id) from skus s where s.tenant_id=t.tenant_id),'[]'::jsonb),
  'warehouses',coalesce((select jsonb_agg(to_jsonb(w) order by warehouse_id) from warehouses w where w.tenant_id=t.tenant_id),'[]'::jsonb),
  'locations',coalesce((select jsonb_agg(to_jsonb(l) order by location_id) from warehouse_locations l where l.tenant_id=t.tenant_id),'[]'::jsonb),
  'channel_mappings',coalesce((select jsonb_agg(to_jsonb(m) order by to_jsonb(m)::text) from channel_sku_mappings m where m.tenant_id=t.tenant_id),'[]'::jsonb)
 ) from tenants t order by t.tenant_id;
commit;
