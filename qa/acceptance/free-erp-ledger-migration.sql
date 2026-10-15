-- Guarded execution of the new migration against the protected QA database only.
begin;
do $$
begin
  if current_database() <> 'erp_shared_contract_test'
     or current_user <> 'erp_grid_test'
     or inet_server_port() <> 15438 then
    raise exception 'refusing a database outside the protected QA identity';
  end if;
end $$;
\i migrations/0281_cost_entry_order.sql
commit;
