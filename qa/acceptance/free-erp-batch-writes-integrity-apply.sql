\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected batch writer QA identity required';
 end if;
end $$;
select current_database(),current_user,inet_server_addr(),inet_server_port();
\ir ../../migrations/0289_inventory_batch_movement_integrity.sql
commit;
