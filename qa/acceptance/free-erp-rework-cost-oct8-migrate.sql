do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test' or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then raise exception 'incorrect QA target'; end if; end $$;
create table mfg_material_cost_bases (
 tenant_id uuid not null, cost_run_id uuid not null,
 material_cent bigint not null check(material_cent>=0),
 rework_cent bigint not null check(rework_cent>=0 and rework_cent<=material_cent),
 legacy_qty_micros bigint not null check(legacy_qty_micros>=0),
 snapshot jsonb not null, created_at timestamptz not null default now(),
 primary key(tenant_id,cost_run_id),
 foreign key(tenant_id,cost_run_id) references mfg_cost_runs(tenant_id,cost_run_id)
);
create trigger material_cost_basis_immutable before update or delete on mfg_material_cost_bases
 for each row execute function protect_receipt_disposition_fact();
