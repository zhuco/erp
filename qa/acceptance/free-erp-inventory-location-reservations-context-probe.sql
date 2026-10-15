\set ON_ERROR_STOP on
begin;
do $$
declare sample record; constraint_name text;
begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory consumer QA identity required';
 end if;
 select t.tenant_id,u.user_id,r.warehouse_id,r.sku_id into sample
 from tenants t join inventory_reservations r on r.tenant_id=t.tenant_id
 join users u on u.tenant_id=t.tenant_id
 where t.name='QA-INVENTORY-LOCATION-RESERVATIONS' and r.source_type='LOCATION_COMMAND'
  and u.password_hash='QA-synthetic-no-login' limit 1;
 if not found then raise exception 'owned synthetic QA sample required'; end if;
 begin
  insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id,
   actor_user_id,warehouse_id,sku_id,movement_kind)
  values(sample.tenant_id,gen_random_uuid()::text,'QA_CONTEXT_PROBE','RUNNING','',
   sample.user_id,sample.warehouse_id,sample.sku_id,null);
  raise exception 'missing movement kind incorrectly admitted';
 exception when check_violation then
  get stacked diagnostics constraint_name=CONSTRAINT_NAME;
  if constraint_name<>'command_receipts_movement_context' then
   raise exception 'wrong context guard: %',constraint_name;
  end if;
 end;
end $$;
select jsonb_build_object('status','PASS','missing_kind_rejected',true,
 'constraint','command_receipts_movement_context','committed_rows',0);
commit;
