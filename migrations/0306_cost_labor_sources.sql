create table mfg_cost_center_bindings (
 tenant_id uuid not null,work_order_id uuid not null,center_id uuid not null,
 snapshot jsonb not null,created_at timestamptz not null default now(),
 primary key(tenant_id,work_order_id),
 foreign key(tenant_id,work_order_id) references mfg_work_orders(tenant_id,work_order_id),
 foreign key(tenant_id,center_id) references shared_work_centers(tenant_id,center_id)
);
create table mfg_cost_labor_bases (
 tenant_id uuid not null,cost_run_id uuid not null,labor_cent bigint not null check(labor_cent>=0),
 snapshot jsonb not null,created_at timestamptz not null default now(),primary key(tenant_id,cost_run_id),
 foreign key(tenant_id,cost_run_id) references mfg_cost_runs(tenant_id,cost_run_id)
);
create trigger cost_center_binding_immutable before update or delete on mfg_cost_center_bindings
 for each row execute function protect_receipt_disposition_fact();
create trigger cost_labor_basis_immutable before update or delete on mfg_cost_labor_bases
 for each row execute function protect_receipt_disposition_fact();
