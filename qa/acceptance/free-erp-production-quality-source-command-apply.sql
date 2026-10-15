\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438
  or to_regclass('public.inspection_original_command_key') is null
  or to_regclass('public.inspection_command_identity') is not null then
  raise exception 'protected QA-only first inspection command identity repair required';
 end if;
end $$;
\ir ../../migrations/0292_inspection_command_identity.sql
commit;
