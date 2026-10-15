begin read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
with selected(test_case,tenant_id) as (values
 ('wrong_sku','5f669a51-f085-40c2-b228-fc4422483319'::uuid),
 ('replay_original_snapshot','182c81ab-a354-4889-95d7-789dc1645a85'::uuid),
 ('missing_foreign_source','5141d991-1174-42b7-851b-b43c48f1d448'::uuid),
 ('late_stock_failure','2cea23af-1722-45ab-8b36-87485fa0f630'::uuid),
 ('same_sku_source_after_opening_correction','69218b10-35ad-4a7a-a155-098f771bba88'::uuid),
 ('twenty_same_identity','d295c901-8f3f-406c-b9c7-e875dd10aca4'::uuid),
 ('whole_batch_return','fcfc8b54-0342-4b0f-a362-5c89a7909da8'::uuid)
)
select jsonb_build_object(
 'database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port(),
 'samples',(select jsonb_agg(jsonb_build_object(
  'test_case',s.test_case,'tenant_id',s.tenant_id,
  'moves',(select coalesce(jsonb_agg(jsonb_build_object(
   'move_id',m.move_id,'work_order_id',m.work_order_id,'warehouse_id',m.warehouse_id,
   'sku_id',m.sku_id,'batch_id',m.batch_id,'original_issue_id',m.original_issue_id,
   'kind',m.kind,'qty_micros',m.qty_micros,'shop_snapshot_micros',m.material_shop_micros,
   'consumed_snapshot_micros',m.material_consumed_micros
  ) order by m.created_at,m.move_id),'[]'::jsonb) from mfg_material_moves m where m.tenant_id=s.tenant_id),
  'command_receipts',(select count(*) from command_receipts r where r.tenant_id=s.tenant_id),
  'precise_balances',(select coalesce(jsonb_agg(jsonb_build_object(
   'sku_code',k.sku_code,'sku_id',b.sku_id,'warehouse_id',b.warehouse_id,
   'on_hand_micros',b.on_hand_micros,'reserved_micros',b.reserved_micros,
   'ledger_micros',(select coalesce(sum(l.delta_micros),0) from mfg_qty_ledgers l
    where l.tenant_id=b.tenant_id and l.warehouse_id=b.warehouse_id and l.sku_id=b.sku_id)
  ) order by k.sku_code),'[]'::jsonb) from mfg_qty_balances b
   join skus k on k.tenant_id=b.tenant_id and k.sku_id=b.sku_id where b.tenant_id=s.tenant_id),
  'legacy_balances',(select coalesce(jsonb_agg(jsonb_build_object(
   'sku_code',k.sku_code,'sku_id',b.sku_id,'warehouse_id',b.warehouse_id,
   'on_hand',b.on_hand,'reserved',b.reserved,'frozen',b.frozen,
   'ledger_on_hand',(select coalesce(sum(l.delta_on_hand),0) from inventory_ledgers l
    where l.tenant_id=b.tenant_id and l.warehouse_id=b.warehouse_id and l.sku_id=b.sku_id)
  ) order by k.sku_code),'[]'::jsonb) from inventory_balances b
   join skus k on k.tenant_id=b.tenant_id and k.sku_id=b.sku_id where b.tenant_id=s.tenant_id),
  'batch_balances',(select coalesce(jsonb_agg(jsonb_build_object(
   'batch_id',b.batch_id,'on_hand',b.on_hand,'reserved',b.reserved
  ) order by b.batch_id),'[]'::jsonb) from inventory_batch_balances b where b.tenant_id=s.tenant_id)
 ) order by s.test_case) from selected s),
 'foreign_tenant_effects',(select count(*) from mfg_material_moves
  where tenant_id='add18811-e919-4997-ae23-48d7f226b366'),
 'historical_failed_run',(select jsonb_build_object(
  'tenant_id','bbbfa1c6-588d-4d27-a3db-644d9e8fb9ce','move_count',count(*),
  'unknown_identity_count',count(*) filter(where warehouse_id is null and sku_id is null),
  'issued_micros',coalesce(sum(qty_micros) filter(where kind='ISSUE'),0),
  'consumed_micros',coalesce(sum(qty_micros) filter(where kind='CONSUME'),0)
 ) from mfg_material_moves where tenant_id='bbbfa1c6-588d-4d27-a3db-644d9e8fb9ce')
);
rollback;
