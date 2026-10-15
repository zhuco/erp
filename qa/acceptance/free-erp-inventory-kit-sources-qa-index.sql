-- The initial successful migration is archived as migration-v1.sql.txt.
-- Apply only its added reverse lookup index in the same protected QA database.
\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected kit source QA identity required';
 end if;
 if to_regclass('public.inventory_order_reservation_sources') is null
  or to_regclass('public.inventory_order_reservation_children') is not null then
  raise exception 'unexpected kit source index state; preserve original outcome';
 end if;
end $$;
create index inventory_order_reservation_children
 on inventory_reservations(tenant_id,reservation_source_id) where reservation_source_id is not null;
commit;
