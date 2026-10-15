-- Registered full-run fixture boundary; no mutation or reset.
begin transaction read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
select json_build_object('database',current_database(),'role',current_user,
 'host',inet_server_addr(),'port',inet_server_port(),
 'tenant_ids',coalesce((select json_agg(tenant_id order by tenant_id) from tenants
   where name='QA-CANONICAL-PRECISION'),'[]'::json));
commit;
