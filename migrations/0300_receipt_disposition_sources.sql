-- Source-linked quality allocation. Original arrival facts are never rewritten.
create table mfg_receipt_disposition_costs (
 tenant_id uuid not null, receipt_id uuid not null, cost_run_id uuid not null,
 unit_cent bigint not null check(unit_cent>=0), total_cent bigint not null check(total_cent>=0),
 snapshot jsonb not null, created_at timestamptz not null default now(),
 primary key(tenant_id,receipt_id),
 foreign key(tenant_id,receipt_id) references mfg_receipts(tenant_id,receipt_id)
);
create table mfg_receipt_disposition_commands (
 tenant_id uuid not null, request_id text not null, disposition_id uuid not null,
 receipt_id uuid not null, parent_disposition_id uuid, kind text not null
  check(kind in('ACCEPT','CONCESSION','ISOLATE','SCRAP','RETURN_SUPPLIER','REWORK')),
 qty_micros bigint not null check(qty_micros>0), held_left_micros bigint not null check(held_left_micros>=0),
 depth integer not null check(depth between 0 and 32), version_no integer not null check(version_no>0),
 cost_cent bigint check(cost_cent>=0), rework_work_order_id uuid, material_move_id uuid,
 payload_hash text not null, snapshot jsonb not null, created_at timestamptz not null default now(),
 primary key(tenant_id,request_id), unique(tenant_id,disposition_id),
 foreign key(tenant_id,receipt_id) references mfg_receipts(tenant_id,receipt_id),
 foreign key(tenant_id,disposition_id) references mfg_quality_dispositions(tenant_id,disposition_id),
 foreign key(tenant_id,parent_disposition_id) references mfg_receipt_disposition_commands(tenant_id,disposition_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id),
 check((kind='REWORK')=(rework_work_order_id is not null and material_move_id is not null))
);
create index mfg_receipt_disposition_remaining on mfg_receipt_disposition_commands(tenant_id,receipt_id,parent_disposition_id);
create function protect_receipt_disposition_fact() returns trigger language plpgsql as $$
begin raise exception 'receipt disposition facts are immutable; append an explicitly linked action'; end $$;
create trigger receipt_disposition_command_immutable before update or delete on mfg_receipt_disposition_commands
 for each row execute function protect_receipt_disposition_fact();
create trigger receipt_disposition_cost_immutable before update or delete on mfg_receipt_disposition_costs
 for each row execute function protect_receipt_disposition_fact();
