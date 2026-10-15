\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438
  or to_regclass('public.inspection_command_identity') is null
  or to_regprocedure('public.verify_inspection_original_command()') is null then
  raise exception 'protected first QA-draft command-key forward repair required';
 end if;
end $$;
\ir ../../migrations/0293_inspection_bounded_command_keys.sql
commit;
