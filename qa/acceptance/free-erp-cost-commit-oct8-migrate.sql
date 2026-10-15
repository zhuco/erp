do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test' or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then raise exception 'incorrect QA target'; end if; end $$;
create table mfg_cost_commit_authorities (
 tenant_id uuid not null,request_id text not null,cost_run_id uuid not null,
 user_id uuid not null,session_id uuid not null,credential_epoch integer not null check(credential_epoch>=0),
 snapshot jsonb not null,created_at timestamptz not null default now(),
 primary key(tenant_id,request_id),foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id),
 foreign key(tenant_id,cost_run_id) references mfg_cost_runs(tenant_id,cost_run_id)
);
create trigger cost_commit_authority_immutable before update or delete on mfg_cost_commit_authorities
 for each row execute function protect_receipt_disposition_fact();
