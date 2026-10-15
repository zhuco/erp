\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test' or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 or current_setting('data_directory')<>'G:/erp/qa/postgres-grid' then raise exception 'protected inspection QA identity required'; end if;
end $$;
\ir ../../migrations/0294_inspection_reading_outcomes.sql
commit;
