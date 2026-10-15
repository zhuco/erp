-- Original 0017 operation log schema, only on the protected QA identity.
\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory consumer QA identity required';
 end if;
end $$;
create table if not exists operation_logs (
 log_id uuid primary key default gen_random_uuid(), tenant_id uuid not null references tenants(tenant_id),
 actor_type text not null, actor_user_id uuid, action text not null, ref_type text, ref_id text,
 metadata jsonb not null default '{}'::jsonb, created_at timestamptz not null default now()
);
create index if not exists idx_operation_logs_tenant_created on operation_logs(tenant_id,created_at desc,log_id desc);
create index if not exists idx_operation_logs_action on operation_logs(tenant_id,action);
create index if not exists idx_operation_logs_ref on operation_logs(tenant_id,ref_type,ref_id);
commit;
