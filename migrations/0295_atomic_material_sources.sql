-- Keep old unknown documents unchanged; new material effects retain their actual source and result.
alter table atomic_source_documents add column if not exists work_order_id uuid;
alter table atomic_source_documents add column if not exists warehouse_id uuid;
alter table atomic_source_documents add column if not exists sku_id uuid;
alter table atomic_source_documents add column if not exists actor_user_id uuid;
alter table atomic_source_documents add column if not exists move_id uuid;
alter table atomic_source_documents add column if not exists request_id text;
alter table atomic_source_documents add column if not exists snapshot jsonb;

create table atomic_material_values (
  tenant_id uuid not null,
  document_id uuid not null,
  move_id uuid not null,
  qty_micros bigint not null check (qty_micros > 0),
  unit_cost_cent bigint not null check (unit_cost_cent >= 0),
  value_cent_micros numeric(38,0) not null,
  primary key (tenant_id,document_id),
  unique (tenant_id,move_id),
  foreign key (tenant_id,document_id) references atomic_source_documents(tenant_id,document_id),
  foreign key (tenant_id,move_id) references mfg_material_moves(tenant_id,move_id),
  check (value_cent_micros=qty_micros::numeric*unit_cost_cent::numeric)
);

create function keep_atomic_material_original() returns trigger language plpgsql as $$
begin
  raise exception 'atomic material original facts are immutable';
end $$;
create trigger atomic_material_original before update or delete on atomic_source_documents
  for each row when (old.snapshot is not null) execute function keep_atomic_material_original();
create trigger atomic_material_value_original before update or delete on atomic_material_values
  for each row execute function keep_atomic_material_original();

create function check_atomic_material_commit() returns trigger language plpgsql as $$
begin
  if new.snapshot is null or new.request_id is null or new.source_action<>'ISSUE'
     or new.work_order_id is null or new.warehouse_id is null or new.sku_id is null or new.move_id is null
     or (new.snapshot->>'schema') is distinct from '1'
     or (new.snapshot->>'document_id') is distinct from new.document_id::text
     or (new.snapshot->>'request_id') is distinct from new.request_id
     or (new.snapshot->>'work_order_id') is distinct from new.work_order_id::text
     or (new.snapshot->>'warehouse_id') is distinct from new.warehouse_id::text
     or (new.snapshot->>'sku_id') is distinct from new.sku_id::text
     or (new.snapshot->>'move_id') is distinct from new.move_id::text
     or (new.snapshot->>'actor_user_id') is distinct from new.actor_user_id::text
     or (new.snapshot->>'qty_micros') is distinct from new.qty_micros::text
     or (new.snapshot->>'action') is distinct from 'ISSUE'
     or (new.snapshot->>'action_no') is distinct from new.action_no
     or (new.snapshot->>'amount_cent') is distinct from new.amount_cent::text then
    raise exception 'new atomic material requires its actual source and original result';
  end if;
  if not exists(select 1 from mfg_material_moves m join mfg_work_orders w
      on w.tenant_id=m.tenant_id and w.work_order_id=m.work_order_id
      where m.tenant_id=new.tenant_id and m.move_id=new.move_id and m.work_order_id=new.work_order_id
        and m.warehouse_id=new.warehouse_id and m.sku_id=new.sku_id and m.kind='ISSUE' and m.qty_micros=new.qty_micros
        and m.material_issued_micros=(new.snapshot->'material'->>'issued_micros')::bigint
        and m.material_returned_micros=(new.snapshot->'material'->>'returned_micros')::bigint
        and m.material_consumed_micros=(new.snapshot->'material'->>'consumed_micros')::bigint
        and m.material_shop_micros=(new.snapshot->'material'->>'shop_micros')::bigint)
     or not exists(select 1 from atomic_material_values v where v.tenant_id=new.tenant_id
      and v.document_id=new.document_id and v.move_id=new.move_id and v.qty_micros=new.qty_micros
      and v.value_cent_micros::text=new.snapshot->>'value_cent_micros'
      and v.unit_cost_cent=(new.snapshot->>'unit_cost_cent')::bigint)
     or not exists(select 1 from command_receipts r where r.tenant_id=new.tenant_id and r.request_id=new.request_id
      and r.status='SUCCEEDED' and r.effect_id=new.document_id::text) then
    raise exception 'atomic material effects and original receipt must commit together';
  end if;
  if not exists(select 1 from inventory_ledgers l where l.tenant_id=new.tenant_id
      and l.ledger_id=(new.snapshot->>'inventory_ledger_id')::uuid
      and l.warehouse_id=new.warehouse_id and l.sku_id=new.sku_id
      and l.ref_type='mfg_material_move' and l.ref_id=new.move_id::text and l.delta_on_hand_micros=-new.qty_micros)
     or not exists(select 1 from operation_logs o where o.tenant_id=new.tenant_id
      and o.log_id=(new.snapshot->>'operation_log_id')::uuid and o.action='MFG_ATOMIC_ISSUE'
      and o.ref_type='atomic_material_document' and o.ref_id=new.document_id::text
      and o.actor_user_id is not distinct from new.actor_user_id
      and o.metadata=(new.snapshot-'operation_log_id'-'audit_id'-'outbox_id'-'posting_id'))
     or not exists(select 1 from audit_logs a where a.tenant_id=new.tenant_id
      and a.audit_id=(new.snapshot->>'audit_id')::uuid and a.action='mfg.atomic.issue'
      and a.entity_type='atomic_material_document' and a.entity_id=new.document_id::text
      and a.user_id is not distinct from new.actor_user_id
      and a.after_json=(new.snapshot-'operation_log_id'-'audit_id'-'outbox_id'-'posting_id'))
     or not exists(select 1 from outbox_events e where e.tenant_id=new.tenant_id
      and e.outbox_id=(new.snapshot->>'outbox_id')::uuid and e.event_type='manufacturing.material.issued'
      and e.payload=(new.snapshot-'outbox_id'-'posting_id'))
     or not exists(select 1 from atomic_posting_events p where p.tenant_id=new.tenant_id
      and p.event_id=(new.snapshot->>'posting_id')::uuid and p.document_id=new.document_id
      and p.source_action='ISSUE' and p.action_no=new.action_no and p.amount_cent=new.amount_cent) then
    raise exception 'atomic material stock logs and pending event must commit together';
  end if;
  return null;
end $$;
create constraint trigger atomic_material_commit after insert on atomic_source_documents
  deferrable initially deferred for each row
  when (new.source_action='ISSUE' or new.snapshot is not null)
  execute function check_atomic_material_commit();
