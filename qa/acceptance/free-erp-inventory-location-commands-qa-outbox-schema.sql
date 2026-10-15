-- QA lacks the nullable trace column from the real 0030 migration.
\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected location command QA identity required';
 end if;
end $$;
alter table if exists outbox_events
  add column if not exists trace_id text;
commit;
