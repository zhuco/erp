\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438
  or to_regclass('public.shared_inspection_source_bindings') is not null
  or to_regclass('public.mfg_quality_release_proofs') is not null then
  raise exception 'protected QA-only first inspection binding application required';
 end if;
end $$;
\ir ../../migrations/0291_inspection_source_bindings.sql
commit;
