begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
create temp table qa_material_identity_probe_results (
 case_name text, expected_message text, observed_message text
) on commit drop;
do $$
declare
 base_issue mfg_material_moves%rowtype;
 spent_issue mfg_material_moves%rowtype;
 picked mfg_material_moves%rowtype;
 bad_sku uuid;
 foreign_warehouse uuid;
 foreign_batch uuid;
 wh uuid;
 sku uuid;
 batch uuid;
 source uuid;
 kind text;
 expected text;
 observed text;
 label text;
 caught boolean;
 i integer;
begin
 select m.* into strict base_issue from mfg_material_moves m
  where m.tenant_id='5f669a51-f085-40c2-b228-fc4422483319' and m.kind='ISSUE';
 select issue.* into strict spent_issue from mfg_material_moves issue
  join mfg_material_moves consumed on consumed.tenant_id=issue.tenant_id
   and consumed.original_issue_id=issue.move_id and consumed.kind='CONSUME'
  where issue.tenant_id='69218b10-35ad-4a7a-a155-098f771bba88';
 select sku_id into strict bad_sku from skus
  where tenant_id=base_issue.tenant_id and sku_code='QA-B';
 select warehouse_id into strict foreign_warehouse from warehouses
  where tenant_id='add18811-e919-4997-ae23-48d7f226b366';
 select batch_id into foreign_batch from sku_batches
  where tenant_id='fcfc8b54-0342-4b0f-a362-5c89a7909da8' order by batch_id limit 1;
 if foreign_batch is null then raise exception 'Expected batch fixture'; end if;
 for i in 1..7 loop
  picked:=base_issue; wh:=picked.warehouse_id; sku:=picked.sku_id;
  batch:=null; source:=null; kind:='ISSUE'; caught:=false; observed:=null;
  case i
   when 1 then
    wh:=null; label:='missing_identity';
    expected:='material warehouse, SKU and positive quantity are required';
   when 2 then
    wh:=foreign_warehouse; label:='foreign_warehouse';
    expected:='material order, warehouse or SKU is outside the enterprise';
   when 3 then
    batch:=foreign_batch; label:='foreign_batch';
    expected:='material batch is outside the selected SKU';
   when 4 then
    kind:='CONSUME'; source:=picked.move_id; sku:=bad_sku; label:='wrong_source_sku';
    expected:='original material issue does not match the selected material';
   when 5 then
    picked:=spent_issue; wh:=picked.warehouse_id; sku:=picked.sku_id;
    kind:='CONSUME'; source:=picked.move_id; label:='source_already_spent';
    expected:='material return or consume exceeds the original issue';
   when 6 then
    label:='missing_quantity_ledger';
    expected:='material movement requires its matching quantity ledger';
   when 7 then
    source:=picked.move_id; label:='issue_has_original_source';
    expected:='new material issue cannot consume an original issue';
  end case;
  begin
   insert into mfg_material_moves(tenant_id,move_id,work_order_id,kind,qty_micros,
    warehouse_id,sku_id,batch_id,original_issue_id)
   values(picked.tenant_id,gen_random_uuid(),picked.work_order_id,kind,1,wh,sku,batch,source);
  exception when raise_exception then
   get stacked diagnostics observed=message_text;
   caught:=true;
  end;
  if not caught or observed<>expected then
   raise exception 'Probe % expected %, received %',label,expected,observed;
  end if;
  insert into qa_material_identity_probe_results values(label,expected,observed);
 end loop;
end $$;
select jsonb_build_object(
 'database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port(),
 'probes',(select jsonb_agg(jsonb_build_object('case',case_name,'expected',expected_message,
  'observed',observed_message,'status','PASS') order by case_name) from qa_material_identity_probe_results),
 'transaction','ROLLBACK; no probe material effect is retained'
);
rollback;
