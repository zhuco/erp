-- Actual consume/return of a source-linked rework issue, with frozen source value.
create table mfg_rework_material_values (
 tenant_id uuid not null, move_id uuid not null, disposition_id uuid not null,
 original_issue_id uuid not null, receipt_id uuid not null,
 kind text not null check(kind in('RETURN','CONSUME')),
 qty_micros bigint not null check(qty_micros>0), cost_cent bigint not null check(cost_cent>=0),
 snapshot jsonb not null, created_at timestamptz not null default now(),
 primary key(tenant_id,move_id),
 foreign key(tenant_id,move_id) references mfg_material_moves(tenant_id,move_id),
 foreign key(tenant_id,original_issue_id) references mfg_material_moves(tenant_id,move_id),
 foreign key(tenant_id,disposition_id) references mfg_receipt_disposition_commands(tenant_id,disposition_id)
);
create index mfg_rework_material_source on mfg_rework_material_values(tenant_id,original_issue_id);
create trigger rework_material_value_immutable before update or delete on mfg_rework_material_values
 for each row execute function protect_receipt_disposition_fact();
alter table mfg_receipt_disposition_commands add column returned_source_move_id uuid;
alter table mfg_receipt_disposition_commands add foreign key(tenant_id,returned_source_move_id)
 references mfg_rework_material_values(tenant_id,move_id);
create index mfg_receipt_returned_allocation on mfg_receipt_disposition_commands(tenant_id,returned_source_move_id);
