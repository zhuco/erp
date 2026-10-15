alter table mfg_cost_runs alter column unit_cent drop not null;
create table mfg_cost_allocations (
 tenant_id uuid not null,cost_run_id uuid not null,policy text not null check(policy='QUANTITY_SHARE_V1'),
 total_cent bigint not null check(total_cent>=0),finished_cent bigint not null check(finished_cent>=0),
 wip_cent bigint not null check(wip_cent>=0),rejected_cent bigint not null check(rejected_cent>=0),
 finished_qty_micros bigint not null check(finished_qty_micros>=0),wip_qty_micros bigint not null check(wip_qty_micros>=0),
 rejected_qty_micros bigint not null check(rejected_qty_micros>=0),unit_cent bigint,unit_cent_micros numeric(38,0),
 snapshot jsonb not null,created_at timestamptz not null default now(),primary key(tenant_id,cost_run_id),
 foreign key(tenant_id,cost_run_id) references mfg_cost_runs(tenant_id,cost_run_id),
 check(finished_cent::numeric+wip_cent::numeric+rejected_cent::numeric=total_cent::numeric),
 check((finished_qty_micros>0)=(unit_cent is not null and unit_cent_micros is not null))
);
create trigger cost_allocation_immutable before update or delete on mfg_cost_allocations
 for each row execute function protect_receipt_disposition_fact();
