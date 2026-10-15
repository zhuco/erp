-- Protected isolated cost fixture, read only. No repair/backfill of failed samples.
\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected legacy reservation QA identity required';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port(),
 'tenant',to_jsonb(t),
 'entries',coalesce((select jsonb_agg(to_jsonb(c) order by entry_sequence,entry_id) from shared_item_costs c where c.tenant_id=t.tenant_id),'[]'::jsonb),
 'receipts',coalesce((select jsonb_agg(to_jsonb(c) order by request_id) from command_receipts c where c.tenant_id=t.tenant_id),'[]'::jsonb)
) from tenants t where t.tenant_id=:'qa_cost_tenant'::uuid;
commit;
