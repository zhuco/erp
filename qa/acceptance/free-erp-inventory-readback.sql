-- Independent protected-DB facts; no runtime or source mutation.
\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected inventory read QA identity required';
 end if;
end $$;
select jsonb_build_object(
 'database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port(),
 'fixtures',coalesce((select jsonb_agg(jsonb_build_object(
  'tenant_id',t.tenant_id,
  'balances',coalesce((select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from inventory_balances b where b.tenant_id=t.tenant_id),'[]'::jsonb),
  'ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from inventory_ledgers l where l.tenant_id=t.tenant_id),'[]'::jsonb),
  'activations',coalesce((select jsonb_agg(to_jsonb(a) order by warehouse_id,sku_id) from inventory_quantity_activations a where a.tenant_id=t.tenant_id),'[]'::jsonb),
  'source_maps',coalesce((select jsonb_agg(to_jsonb(m) order by to_jsonb(m)::text) from inventory_quantity_source_maps m where m.tenant_id=t.tenant_id),'[]'::jsonb),
  'skus',coalesce((select jsonb_agg(to_jsonb(s) order by sku_id) from skus s where s.tenant_id=t.tenant_id),'[]'::jsonb),
  'boms',coalesce((select jsonb_agg(to_jsonb(b) order by bundle_sku_id,component_sku_id) from sku_boms b where b.tenant_id=t.tenant_id),'[]'::jsonb),
  'warehouses',coalesce((select jsonb_agg(to_jsonb(w) order by warehouse_id) from warehouses w where w.tenant_id=t.tenant_id),'[]'::jsonb),
  'grants',coalesce((select jsonb_agg(to_jsonb(g) order by user_id,warehouse_id) from user_warehouse_access g where g.tenant_id=t.tenant_id),'[]'::jsonb),
  'settings',coalesce((select jsonb_agg(to_jsonb(s) order by setting_id) from settings s where s.tenant_id=t.tenant_id),'[]'::jsonb),
  'outbox',coalesce((select jsonb_agg(to_jsonb(o) order by outbox_id) from outbox_events o where o.tenant_id=t.tenant_id),'[]'::jsonb)
 ) order by t.tenant_id) from tenants t where t.name='QA-CANONICAL-READ'),'[]'::jsonb)
);
commit;
