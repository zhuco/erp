create table mfg_material_issue_values (
 tenant_id uuid not null, issue_id uuid not null, work_order_id uuid not null,
 warehouse_id uuid not null, sku_id uuid not null, qty_micros bigint not null check(qty_micros>0),
 state text not null check(state in('VALUED','UNVALUED')),
 total_cent bigint check(total_cent>=0), snapshot jsonb not null,
 primary key(tenant_id,issue_id), foreign key(tenant_id,issue_id) references mfg_material_moves(tenant_id,move_id),
 check((state='VALUED')=(total_cent is not null))
);
create table mfg_material_value_allocations (
 tenant_id uuid not null, allocation_id uuid not null, issue_id uuid not null,
 request_id text, kind text not null check(kind in('CONSUME','RETURN')),
 qty_micros bigint not null check(qty_micros>0), cost_cent bigint check(cost_cent>=0), snapshot jsonb not null,
 created_at timestamptz not null default now(), primary key(tenant_id,allocation_id),
 foreign key(tenant_id,issue_id) references mfg_material_moves(tenant_id,move_id)
);
create index mfg_material_value_source on mfg_material_value_allocations(tenant_id,issue_id);
create trigger material_issue_value_immutable before update or delete on mfg_material_issue_values
 for each row execute function protect_receipt_disposition_fact();
create trigger material_value_allocation_immutable before update or delete on mfg_material_value_allocations
 for each row execute function protect_receipt_disposition_fact();
