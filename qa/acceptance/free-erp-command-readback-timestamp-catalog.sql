\set ON_ERROR_STOP on
begin transaction read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected readback QA identity required';
 end if;
end $$;
select coalesce(jsonb_object_agg(table_name,cols),'{}'::jsonb)
from (select table_name,jsonb_agg(column_name order by column_name) as cols
 from information_schema.columns where table_schema='public'
 and data_type='timestamp with time zone' group by table_name) t;
commit;
