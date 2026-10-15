-- Exact quantities live in the original inventory tables. NULL means that the
-- historical key has not yet passed the locked, source-based activation check.
-- Old integer 1 continues to mean one whole base unit, never one micro-unit.
alter table inventory_balances add column if not exists on_hand_micros bigint;
alter table inventory_balances add column if not exists reserved_micros bigint;
alter table inventory_balances add column if not exists frozen_micros bigint;
alter table inventory_balances add column if not exists in_transit_micros bigint;
alter table inventory_ledgers add column if not exists delta_on_hand_micros bigint;
alter table inventory_ledgers add column if not exists delta_reserved_micros bigint;
alter table inventory_ledgers add column if not exists delta_frozen_micros bigint;
alter table inventory_ledgers add column if not exists delta_in_transit_micros bigint;
alter table inventory_reservations add column if not exists qty_micros bigint;
alter table inventory_reservations add column if not exists original_qty_micros bigint;
alter table inventory_reservations add column if not exists source_type text;
alter table inventory_reservations add column if not exists source_id text;
alter table inventory_batch_balances add column if not exists on_hand_micros bigint;
alter table inventory_batch_balances add column if not exists reserved_micros bigint;

create index if not exists inventory_quantity_source_lookup
 on inventory_ledgers(tenant_id,warehouse_id,sku_id,ref_id);
create index if not exists mfg_quantity_source_lookup
 on mfg_qty_ledgers(tenant_id,warehouse_id,sku_id);
create index if not exists inventory_reservation_source_lookup
 on inventory_reservations(tenant_id,source_type,source_id,warehouse_id,sku_id);

-- Activation records and source maps are audit facts, not additional balances.
create table if not exists inventory_quantity_activations (
 tenant_id uuid not null,
 warehouse_id uuid not null,
 sku_id uuid not null,
 legacy_on_hand bigint not null,
 legacy_reserved bigint not null,
 legacy_frozen bigint not null,
 legacy_in_transit bigint not null,
 activated_at timestamptz not null default now(),
 primary key(tenant_id,warehouse_id,sku_id)
);
create table if not exists inventory_quantity_source_maps (
 tenant_id uuid not null,
 manufacturing_ledger_id uuid not null,
 inventory_ledger_id uuid not null references inventory_ledgers(ledger_id),
 qty_micros bigint not null,
 mapped_at timestamptz not null default now(),
 primary key(tenant_id,manufacturing_ledger_id),
 unique(tenant_id,inventory_ledger_id)
);

create or replace function guard_inventory_exact_balance() returns trigger
language plpgsql as $$
begin
 if new.on_hand_micros is null and new.reserved_micros is null
    and new.frozen_micros is null and new.in_transit_micros is null then
  if tg_op='UPDATE' and old.on_hand_micros is not null then
   raise exception 'exact inventory cannot return to an integer-only balance';
  end if;
  return new;
 end if;
 if new.on_hand_micros is null or new.reserved_micros is null
    or new.frozen_micros is null or new.in_transit_micros is null then
  raise exception 'all exact inventory quantity fields are required';
 end if;
 if not exists(select 1 from warehouses where tenant_id=new.tenant_id and warehouse_id=new.warehouse_id)
    or not exists(select 1 from skus where tenant_id=new.tenant_id and sku_id=new.sku_id) then
  raise exception 'exact inventory warehouse or SKU is outside the enterprise';
 end if;
 if tg_op='UPDATE' and old.on_hand_micros is not null
    and row(new.on_hand_micros,new.reserved_micros,new.frozen_micros,new.in_transit_micros)
        =row(old.on_hand_micros,old.reserved_micros,old.frozen_micros,old.in_transit_micros)
    and row(new.on_hand,new.reserved,new.frozen,new.in_transit)
        is distinct from row(old.on_hand,old.reserved,old.frozen,old.in_transit) then
  raise exception 'integer-only inventory write requires the exact quantity service';
 end if;
 if new.on_hand_micros<0 or new.reserved_micros<0 or new.frozen_micros<0
    or new.in_transit_micros<0
    or new.on_hand_micros::numeric<new.reserved_micros::numeric+new.frozen_micros then
  raise exception 'exact inventory has negative physical, reserved, frozen, transit or available quantity';
 end if;
 -- These columns are explicitly whole-unit compatibility projections. Admission
 -- and exact responses must use the *_micros columns and subtract before rounding.
 new.on_hand:=new.on_hand_micros/1000000;
 new.reserved:=new.reserved_micros/1000000;
 new.frozen:=new.frozen_micros/1000000;
 new.in_transit:=new.in_transit_micros/1000000;
 return new;
end $$;
drop trigger if exists inventory_exact_balance_guard on inventory_balances;
create trigger inventory_exact_balance_guard before insert or update on inventory_balances
 for each row execute function guard_inventory_exact_balance();

create or replace function guard_inventory_exact_ledger() returns trigger
language plpgsql as $$
begin
 perform pg_advisory_xact_lock(hashtext(new.tenant_id::text),
  hashtext('inventory:'||new.warehouse_id::text||':'||new.sku_id::text));
 if not exists(select 1 from warehouses where tenant_id=new.tenant_id and warehouse_id=new.warehouse_id)
    or not exists(select 1 from skus where tenant_id=new.tenant_id and sku_id=new.sku_id) then
  raise exception 'inventory ledger warehouse or SKU is outside the enterprise';
 end if;
 if new.delta_on_hand_micros is null and new.delta_reserved_micros is null
    and new.delta_frozen_micros is null and new.delta_in_transit_micros is null then
  new.delta_on_hand_micros:=(new.delta_on_hand::numeric*1000000)::bigint;
  new.delta_reserved_micros:=(new.delta_reserved::numeric*1000000)::bigint;
  new.delta_frozen_micros:=(new.delta_frozen::numeric*1000000)::bigint;
  new.delta_in_transit_micros:=(new.delta_in_transit::numeric*1000000)::bigint;
 elsif new.delta_on_hand_micros is null or new.delta_reserved_micros is null
    or new.delta_frozen_micros is null or new.delta_in_transit_micros is null then
  raise exception 'all exact inventory ledger fields are required';
 end if;
 if row(new.delta_on_hand,new.delta_reserved,new.delta_frozen,new.delta_in_transit)
    is distinct from row(new.delta_on_hand_micros/1000000,new.delta_reserved_micros/1000000,
                         new.delta_frozen_micros/1000000,new.delta_in_transit_micros/1000000) then
  raise exception 'inventory ledger whole-unit projection does not match exact quantity';
 end if;
 return new;
end $$;
drop trigger if exists inventory_exact_ledger_guard on inventory_ledgers;
create trigger inventory_exact_ledger_guard before insert on inventory_ledgers
 for each row execute function guard_inventory_exact_ledger();

-- Historical source facts are immutable. Activation may fill only previously
-- absent precision columns, and only with the checked whole-unit equivalent.
create or replace function guard_inventory_ledger_update() returns trigger
language plpgsql as $$
begin
 if (to_jsonb(new)-array['delta_on_hand_micros','delta_reserved_micros','delta_frozen_micros','delta_in_transit_micros'])
    is distinct from
    (to_jsonb(old)-array['delta_on_hand_micros','delta_reserved_micros','delta_frozen_micros','delta_in_transit_micros']) then
  raise exception 'inventory ledger source facts are immutable';
 end if;
 if new.delta_on_hand_micros is distinct from coalesce(old.delta_on_hand_micros,(old.delta_on_hand::numeric*1000000)::bigint)
    or new.delta_reserved_micros is distinct from coalesce(old.delta_reserved_micros,(old.delta_reserved::numeric*1000000)::bigint)
    or new.delta_frozen_micros is distinct from coalesce(old.delta_frozen_micros,(old.delta_frozen::numeric*1000000)::bigint)
    or new.delta_in_transit_micros is distinct from coalesce(old.delta_in_transit_micros,(old.delta_in_transit::numeric*1000000)::bigint) then
  raise exception 'inventory ledger exact quantities are immutable after proven activation';
 end if;
 return new;
end $$;
drop trigger if exists inventory_ledger_update_guard on inventory_ledgers;
create trigger inventory_ledger_update_guard before update on inventory_ledgers
 for each row execute function guard_inventory_ledger_update();

-- A posted quantity effect remains a historical fact. Corrections append a
-- compensating source; deleting an effect would disconnect the balance and audit.
create or replace function guard_inventory_ledger_delete() returns trigger
language plpgsql as $$
begin
 raise exception 'inventory ledger facts are immutable; append a correction instead';
end $$;
drop trigger if exists inventory_ledger_delete_guard on inventory_ledgers;
create trigger inventory_ledger_delete_guard before delete on inventory_ledgers
 for each row execute function guard_inventory_ledger_delete();

-- During writer migration, an activated key cannot still be sold by an
-- independent shared-stock ledger. A source projection must cite its canonical
-- effect; this guard is a fence, not completion of the remaining shared writers.
create or replace function guard_shared_stock_quantity() returns trigger
language plpgsql as $$
begin
 perform pg_advisory_xact_lock(hashtext(new.tenant_id::text),
  hashtext('inventory:'||new.warehouse_id::text||':'||new.sku_id::text));
 if new.delta_micros<>0 and exists(
   select 1 from inventory_balances b where b.tenant_id=new.tenant_id
    and b.warehouse_id=new.warehouse_id and b.sku_id=new.sku_id
    and b.on_hand_micros is not null
 ) and not exists(
   select 1 from inventory_ledgers i where i.tenant_id=new.tenant_id
    and i.warehouse_id=new.warehouse_id and i.sku_id=new.sku_id
    and i.ref_type='shared_stock_ledger' and i.ref_id=new.ledger_id::text
    and case new.bucket
      when 'ON_HAND' then i.delta_on_hand_micros
      when 'RESERVED' then i.delta_reserved_micros
      when 'FROZEN' then i.delta_frozen_micros
      when 'IN_TRANSIT' then i.delta_in_transit_micros
      else null
    end=new.delta_micros
 ) then
  raise exception 'shared stock writer requires the canonical precision service';
 end if;
 return new;
end $$;
drop trigger if exists shared_stock_quantity_guard on shared_stock_ledgers;
create trigger shared_stock_quantity_guard before insert on shared_stock_ledgers
 for each row execute function guard_shared_stock_quantity();

create or replace function guard_manufacturing_shadow_write() returns trigger
language plpgsql as $$
begin
 if exists(select 1 from inventory_balances i where i.tenant_id=new.tenant_id
     and i.warehouse_id=new.warehouse_id and i.sku_id=new.sku_id and i.on_hand_micros is not null) then
  raise exception 'manufacturing shadow balance is historical; use the exact inventory service';
 end if;
 return new;
end $$;
drop trigger if exists mfg_shadow_quantity_guard on mfg_qty_balances;
create trigger mfg_shadow_quantity_guard before insert or update on mfg_qty_balances
 for each row execute function guard_manufacturing_shadow_write();

create or replace function guard_material_quantity_source() returns trigger
language plpgsql as $$
begin
 perform pg_advisory_xact_lock(hashtext(new.tenant_id::text),
  hashtext('inventory:'||new.warehouse_id::text||':'||new.sku_id::text));
 if new.delta_micros <> 0
    and exists(select 1 from inventory_balances i where i.tenant_id=new.tenant_id
     and i.warehouse_id=new.warehouse_id and i.sku_id=new.sku_id and i.on_hand_micros is not null)
    and not exists(select 1 from inventory_ledgers i where i.tenant_id=new.tenant_id
     and i.warehouse_id=new.warehouse_id and i.sku_id=new.sku_id
     and i.ref_type='mfg_material_move' and i.ref_id=new.move_id::text
     and i.delta_on_hand_micros=new.delta_micros) then
  raise exception 'manufacturing quantity detail requires its original exact inventory effect';
 end if;
 return new;
end $$;
drop trigger if exists mfg_quantity_source_guard on mfg_qty_ledgers;
create trigger mfg_quantity_source_guard before insert on mfg_qty_ledgers
 for each row execute function guard_material_quantity_source();

create or replace function guard_inventory_exact_batch() returns trigger
language plpgsql as $$
begin
 if new.on_hand_micros is null and new.reserved_micros is null then
  if tg_op='UPDATE' and old.on_hand_micros is not null then
   raise exception 'exact batch cannot return to integer-only quantities';
  end if;
  return new;
 end if;
 if new.on_hand_micros is null or new.reserved_micros is null then
  raise exception 'both exact batch quantities are required';
 end if;
 if tg_op='UPDATE' and old.on_hand_micros is not null
    and row(new.on_hand_micros,new.reserved_micros)=row(old.on_hand_micros,old.reserved_micros)
    and row(new.on_hand,new.reserved) is distinct from row(old.on_hand,old.reserved) then
  raise exception 'integer-only batch write requires the exact quantity service';
 end if;
 if new.on_hand_micros<0 or new.reserved_micros<0 or new.on_hand_micros<new.reserved_micros then
  raise exception 'exact batch has negative or over-reserved quantity';
 end if;
 new.on_hand:=new.on_hand_micros/1000000;
 new.reserved:=new.reserved_micros/1000000;
 return new;
end $$;
drop trigger if exists inventory_exact_batch_guard on inventory_batch_balances;
create trigger inventory_exact_batch_guard before insert or update on inventory_batch_balances
 for each row execute function guard_inventory_exact_batch();

create or replace function guard_inventory_exact_reservation() returns trigger
language plpgsql as $$
begin
 if new.qty_micros is null then
  new.qty_micros:=(new.qty::numeric*1000000)::bigint;
 elsif tg_op='UPDATE' and old.qty_micros is not null and new.qty_micros=old.qty_micros
    and new.qty<>old.qty then
  raise exception 'integer-only reservation write requires the exact quantity service';
 end if;
 if new.qty_micros<0 then raise exception 'negative exact source reservation'; end if;
 if tg_op='INSERT' and new.original_qty_micros is null then
  new.original_qty_micros:=new.qty_micros;
 end if;
 if tg_op='UPDATE' and old.original_qty_micros is not null
    and new.original_qty_micros is distinct from old.original_qty_micros then
  raise exception 'original reservation quantity is immutable';
 end if;
 if new.original_qty_micros is not null
    and (new.original_qty_micros<0 or new.qty_micros>new.original_qty_micros) then
  raise exception 'source reservation exceeds its original quantity';
 end if;
 new.qty:=new.qty_micros/1000000;
 return new;
end $$;
drop trigger if exists inventory_exact_reservation_guard on inventory_reservations;
create trigger inventory_exact_reservation_guard before insert or update on inventory_reservations
 for each row execute function guard_inventory_exact_reservation();
