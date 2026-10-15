\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory consumer QA identity required';
 end if;
end $$;
alter table command_receipts drop constraint command_receipts_movement_context;
alter table command_receipts add constraint command_receipts_movement_context check (
 (movement_kind is null and actor_user_id is null and warehouse_id is null
  and sku_id is null and original_result is null) or
 (movement_kind is not null and movement_kind in ('INBOUND','OUTBOUND','ADJUSTMENT','RESERVE','RELEASE')
  and actor_user_id is not null and warehouse_id is not null and sku_id is not null
  and (status <> 'SUCCEEDED' or
   (original_result is not null and jsonb_typeof(original_result)='object')))
);
commit;
