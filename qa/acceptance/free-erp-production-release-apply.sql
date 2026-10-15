\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected production release QA identity required';
 end if;
 if to_regclass('public.mfg_receipt_release_commands') is not null then
  raise exception 'production release migration already exists; inspect before retry';
 end if;
end $$;
\ir ../../migrations/0290_production_receipt_release_sources.sql
commit;
