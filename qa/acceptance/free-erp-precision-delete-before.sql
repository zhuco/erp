\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
with candidate as (
 select i.ledger_id from inventory_ledgers i
 where i.tenant_id='d5be1bb8-628c-42ac-96cc-b650c431a2d1'::uuid
  and not exists(select 1 from inventory_quantity_source_maps m where m.inventory_ledger_id=i.ledger_id)
 order by i.occurred_at desc,i.ledger_id limit 1
), removed as (
 delete from inventory_ledgers where ledger_id in(select ledger_id from candidate) returning ledger_id
)
select json_build_object('tenant_id','d5be1bb8-628c-42ac-96cc-b650c431a2d1','deleted_history_rows',count(*),'rollback',true) from removed;
rollback;
