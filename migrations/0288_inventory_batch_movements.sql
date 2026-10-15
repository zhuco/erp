-- New exact batch provenance only. Historical quantities are never inferred or rewritten.
create unique index inventory_ledgers_owned_effect_key
 on inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id);
create unique index sku_batches_owned_identity_key on sku_batches(tenant_id,sku_id,batch_id);

create table inventory_batch_movements (
 batch_movement_id uuid primary key default gen_random_uuid(),
 tenant_id uuid not null,
 warehouse_id uuid not null,
 sku_id uuid not null,
 batch_id uuid not null,
 inventory_ledger_id uuid not null,
 before_on_hand_micros bigint not null,
 after_on_hand_micros bigint not null,
 before_reserved_micros bigint not null,
 after_reserved_micros bigint not null,
 delta_on_hand_micros bigint not null,
 delta_reserved_micros bigint not null,
 decision_at timestamptz not null,
 batch_metadata_snapshot jsonb not null,
 reason_type text not null,
 ref_type text,
 ref_id text,
 created_at timestamptz not null default clock_timestamp(),
 foreign key(tenant_id,warehouse_id,sku_id,inventory_ledger_id)
  references inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id),
 foreign key(tenant_id,sku_id,batch_id) references sku_batches(tenant_id,sku_id,batch_id),
 unique(tenant_id,inventory_ledger_id,batch_id),
 check(before_on_hand_micros>=before_reserved_micros and before_reserved_micros>=0),
 check(after_on_hand_micros>=after_reserved_micros and after_reserved_micros>=0),
 check(after_on_hand_micros::numeric-before_on_hand_micros=delta_on_hand_micros),
 check(after_reserved_micros::numeric-before_reserved_micros=delta_reserved_micros),
 check(jsonb_typeof(batch_metadata_snapshot)='object')
);
create index inventory_batch_movements_history
 on inventory_batch_movements(tenant_id,warehouse_id,sku_id,batch_id,created_at,batch_movement_id);

create function guard_inventory_batch_movement_immutable() returns trigger language plpgsql as $$
begin
 raise exception 'original batch movement is immutable';
end $$;
create trigger inventory_batch_movements_immutable before update or delete on inventory_batch_movements
 for each row execute function guard_inventory_batch_movement_immutable();

-- Multi-batch FEFO inserts several children; reconcile their final sum at commit.
-- SKU reservations with no batch source do not imply ownership of batch holds.
create function verify_inventory_batch_movement_sum() returns trigger language plpgsql as $$
declare expected bigint; actual numeric;
begin
 select delta_on_hand_micros into expected from inventory_ledgers
 where tenant_id=new.tenant_id and warehouse_id=new.warehouse_id and sku_id=new.sku_id
  and ledger_id=new.inventory_ledger_id;
 select sum(delta_on_hand_micros::numeric) into actual from inventory_batch_movements
 where tenant_id=new.tenant_id and inventory_ledger_id=new.inventory_ledger_id;
 if expected is null or actual is distinct from expected::numeric then
  raise exception 'batch children differ from their exact owned inventory ledger';
 end if;
 return null;
end $$;
create constraint trigger inventory_batch_movements_sum after insert on inventory_batch_movements
 deferrable initially deferred for each row execute function verify_inventory_batch_movement_sum();
