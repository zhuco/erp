-- Only new releases receive original snapshots. Do not infer or rewrite historical results.
create unique index mfg_qty_ledgers_owned_release_key
 on mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,ledger_id);
create unique index shared_stock_ledgers_owned_release_key
 on shared_stock_ledgers(tenant_id,warehouse_id,sku_id,ledger_id);

create table mfg_receipt_release_commands (
 tenant_id uuid not null,
 request_id text not null,
 payload_hash text not null,
 receipt_id uuid not null,
 work_order_id uuid not null,
 warehouse_id uuid not null,
 sku_id uuid not null,
 qty_micros bigint not null check(qty_micros>0),
 available_total_micros bigint not null check(available_total_micros>=qty_micros),
 source_qty_ledger_id uuid not null,
 source_stock_ledger_id uuid not null,
 source_inventory_ledger_id uuid not null,
 source_projection_ledger_id uuid not null,
 snapshot jsonb not null,
 created_at timestamptz not null default clock_timestamp(),
 primary key(tenant_id,request_id),
 unique(tenant_id,receipt_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id),
 foreign key(tenant_id,receipt_id) references mfg_receipts(tenant_id,receipt_id),
 foreign key(tenant_id,work_order_id) references mfg_work_orders(tenant_id,work_order_id),
 foreign key(tenant_id,warehouse_id,sku_id,source_qty_ledger_id)
  references mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 foreign key(tenant_id,warehouse_id,sku_id,source_stock_ledger_id)
  references shared_stock_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 foreign key(tenant_id,warehouse_id,sku_id,source_inventory_ledger_id)
  references inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 foreign key(tenant_id,warehouse_id,sku_id,source_projection_ledger_id)
  references inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 check(btrim(request_id)<>''),
 check(coalesce(jsonb_typeof(snapshot)='object' and snapshot->>'schema'='1'
  and snapshot->>'tenant_id'=tenant_id::text and snapshot->>'request_id'=request_id
  and snapshot->>'receipt_id'=receipt_id::text and snapshot->>'work_order_id'=work_order_id::text
  and snapshot->>'warehouse_id'=warehouse_id::text and snapshot->>'sku_id'=sku_id::text
  and snapshot->>'qty_micros'=qty_micros::text
  and snapshot->>'available_total_micros'=available_total_micros::text
  and snapshot#>>'{source,quantity,ledger_id}'=source_qty_ledger_id::text
  and snapshot#>>'{source,stock,ledger_id}'=source_stock_ledger_id::text
  and snapshot#>>'{source,inventory,ledger_id}'=source_inventory_ledger_id::text
  and snapshot#>>'{source,projection,ledger_id}'=source_projection_ledger_id::text,false))
);

create table mfg_receipt_release_effects (
 tenant_id uuid not null,
 request_id text not null,
 warehouse_id uuid not null,
 sku_id uuid not null,
 quality_status text not null check(quality_status in('HOLD','GOOD')),
 stock_ledger_id uuid not null,
 inventory_ledger_id uuid not null,
 projection_ledger_id uuid not null,
 quantity_ledger_id uuid not null,
 delta_on_hand_micros bigint not null,
 delta_frozen_micros bigint not null,
 primary key(tenant_id,request_id,quality_status),
 foreign key(tenant_id,request_id) references mfg_receipt_release_commands(tenant_id,request_id),
 foreign key(tenant_id,warehouse_id,sku_id,stock_ledger_id)
  references shared_stock_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 foreign key(tenant_id,warehouse_id,sku_id,inventory_ledger_id)
  references inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 foreign key(tenant_id,warehouse_id,sku_id,projection_ledger_id)
  references inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 foreign key(tenant_id,warehouse_id,sku_id,quantity_ledger_id)
  references mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 check((quality_status='HOLD' and delta_on_hand_micros<0 and delta_frozen_micros=delta_on_hand_micros)
   or(quality_status='GOOD' and delta_on_hand_micros>0 and delta_frozen_micros=0))
);
create index mfg_receipt_release_command_qty_source on mfg_receipt_release_commands(tenant_id,source_qty_ledger_id);
create index mfg_receipt_release_command_stock_source on mfg_receipt_release_commands(tenant_id,source_stock_ledger_id);
create index mfg_receipt_release_command_inventory_source on mfg_receipt_release_commands(tenant_id,source_inventory_ledger_id);
create index mfg_receipt_release_command_projection_source on mfg_receipt_release_commands(tenant_id,source_projection_ledger_id);
create index mfg_receipt_release_effect_qty_source on mfg_receipt_release_effects(tenant_id,quantity_ledger_id);
create index mfg_receipt_release_effect_stock_source on mfg_receipt_release_effects(tenant_id,stock_ledger_id);
create index mfg_receipt_release_effect_inventory_source on mfg_receipt_release_effects(tenant_id,inventory_ledger_id);
create index mfg_receipt_release_effect_projection_source on mfg_receipt_release_effects(tenant_id,projection_ledger_id);
-- Admission and per-report receipt budgets must not scan every enterprise's receipts.
create index mfg_receipt_release_report_budget on mfg_receipts(tenant_id,work_order_id,report_id);
create index mfg_receipt_release_report_reversal on mfg_reports(tenant_id,work_order_id,reverses_report_id);
create index mfg_receipt_release_stock_source_line on shared_stock_ledgers(tenant_id,source_line,ledger_id);
create index mfg_receipt_release_qty_source_move on mfg_qty_ledgers(tenant_id,move_id,reason_type,ledger_id);
create index mfg_receipt_release_quality_batch on mfg_quality_items(tenant_id,work_order_id,batch_code,item_id);
create index mfg_receipt_release_quality_source on mfg_quality_items(tenant_id,source_kind,source_id,batch_code,item_id);
create index mfg_receipt_release_frozen_source on shared_inspection_freezes(tenant_id,source_kind,source_id,batch_code,freeze_id);

create function guard_mfg_receipt_release_immutable() returns trigger language plpgsql as $$
begin raise exception 'original production release record is immutable'; end $$;
create trigger mfg_receipt_release_commands_immutable before update or delete on mfg_receipt_release_commands
 for each row execute function guard_mfg_receipt_release_immutable();
create trigger mfg_receipt_release_effects_immutable before update or delete on mfg_receipt_release_effects
 for each row execute function guard_mfg_receipt_release_immutable();

create function guard_mfg_receipt_release_parent() returns trigger language plpgsql as $$
declare used boolean;
begin
 if tg_table_name='inventory_ledgers' then
  select exists(select 1 from mfg_receipt_release_commands where tenant_id=old.tenant_id
   and(source_inventory_ledger_id=old.ledger_id or source_projection_ledger_id=old.ledger_id))
   or exists(select 1 from mfg_receipt_release_effects where tenant_id=old.tenant_id
   and(inventory_ledger_id=old.ledger_id or projection_ledger_id=old.ledger_id)) into used;
 elsif tg_table_name='mfg_qty_ledgers' then
  select exists(select 1 from mfg_receipt_release_commands where tenant_id=old.tenant_id and source_qty_ledger_id=old.ledger_id)
   or exists(select 1 from mfg_receipt_release_effects where tenant_id=old.tenant_id and quantity_ledger_id=old.ledger_id) into used;
 elsif tg_table_name='shared_stock_ledgers' then
  select exists(select 1 from mfg_receipt_release_commands where tenant_id=old.tenant_id and source_stock_ledger_id=old.ledger_id)
   or exists(select 1 from mfg_receipt_release_effects where tenant_id=old.tenant_id and stock_ledger_id=old.ledger_id) into used;
 elsif tg_table_name='command_receipts' then
  select exists(select 1 from mfg_receipt_release_commands where tenant_id=old.tenant_id and request_id=old.request_id) into used;
 end if;
 if used then raise exception 'production release source or effect is immutable'; end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inventory_ledger_mfg_release_immutable before update or delete on inventory_ledgers
 for each row execute function guard_mfg_receipt_release_parent();
create trigger mfg_qty_ledger_release_immutable before update or delete on mfg_qty_ledgers
 for each row execute function guard_mfg_receipt_release_parent();
create trigger shared_stock_ledger_mfg_release_immutable before update or delete on shared_stock_ledgers
 for each row execute function guard_mfg_receipt_release_parent();
create trigger command_receipt_mfg_release_immutable before update or delete on command_receipts
 for each row execute function guard_mfg_receipt_release_parent();

create function verify_mfg_receipt_release_effects() returns trigger language plpgsql as $$
declare expected bigint; actual_count bigint; actual_delta numeric; actual_frozen numeric;
begin
 select qty_micros into expected from mfg_receipt_release_commands where tenant_id=new.tenant_id and request_id=new.request_id;
 select count(*),sum(delta_on_hand_micros::numeric),sum(delta_frozen_micros::numeric)
 into actual_count,actual_delta,actual_frozen from mfg_receipt_release_effects where tenant_id=new.tenant_id and request_id=new.request_id;
 if actual_count<>2 or actual_delta is distinct from 0::numeric or actual_frozen is distinct from -expected::numeric
  or exists(select 1 from mfg_receipt_release_effects e where e.tenant_id=new.tenant_id and e.request_id=new.request_id
   and(e.delta_on_hand_micros<>case when e.quality_status='HOLD' then -expected else expected end
   or not exists(select 1 from inventory_ledgers i where i.tenant_id=e.tenant_id and i.ledger_id=e.inventory_ledger_id
    and i.delta_on_hand_micros=e.delta_on_hand_micros and i.delta_frozen_micros=e.delta_frozen_micros
    and i.delta_reserved_micros=0 and i.delta_in_transit_micros=0 and i.ref_type='shared_stock_ledger' and i.ref_id=e.stock_ledger_id::text)
   or not exists(select 1 from inventory_ledgers i where i.tenant_id=e.tenant_id and i.ledger_id=e.projection_ledger_id
    and i.delta_on_hand_micros=e.delta_on_hand_micros and i.delta_frozen_micros=e.delta_frozen_micros
    and i.delta_reserved_micros=0 and i.delta_in_transit_micros=0 and i.ref_type='mfg_material_move' and i.ref_id=e.stock_ledger_id::text)
   or not exists(select 1 from shared_stock_ledgers s where s.tenant_id=e.tenant_id and s.ledger_id=e.stock_ledger_id
    and s.delta_micros=e.delta_on_hand_micros and s.quality_status=e.quality_status and s.bucket='ON_HAND')
   or not exists(select 1 from mfg_qty_ledgers q where q.tenant_id=e.tenant_id and q.ledger_id=e.quantity_ledger_id
    and q.delta_micros=e.delta_on_hand_micros and q.move_id=e.stock_ledger_id and q.reason_type='MFG_RECEIPT_RELEASE'))) then
  raise exception 'production release requires two conserved owned effects';
 end if;
 return null;
end $$;
create constraint trigger mfg_receipt_release_complete after insert on mfg_receipt_release_commands
 deferrable initially deferred for each row execute function verify_mfg_receipt_release_effects();
create constraint trigger mfg_receipt_release_effects_complete after insert on mfg_receipt_release_effects
 deferrable initially deferred for each row execute function verify_mfg_receipt_release_effects();
