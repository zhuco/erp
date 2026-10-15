\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected kit source QA identity required';
 end if;
 if to_regclass('public.inventory_order_reservation_sources') is not null then
  raise exception 'kit source migration already present; preserve original outcome';
 end if;
end $$;
\i G:/erp/migrations/0287_inventory_order_reservation_sources.sql
commit;
