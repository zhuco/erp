-- Formal location reservations retain their original source and append command events.
-- No historical row is assigned a source, location or invented original result.
alter table command_receipts drop constraint command_receipts_movement_context;
alter table command_receipts add constraint command_receipts_movement_context check (
 (movement_kind is null and actor_user_id is null and warehouse_id is null
  and sku_id is null and original_result is null) or
 (movement_kind is not null and movement_kind in ('INBOUND','OUTBOUND','ADJUSTMENT','RESERVE','RELEASE')
  and actor_user_id is not null and warehouse_id is not null and sku_id is not null
  and (status <> 'SUCCEEDED' or
   (original_result is not null and jsonb_typeof(original_result)='object')))
);
alter table inventory_reservations add column location_id uuid;
alter table inventory_reservations add column batch_code text;
alter table inventory_reservations add column quality_status text;
alter table inventory_reservations add column source_context jsonb;
alter table inventory_reservations add column source_request_id text;
alter table inventory_reservations add column released_micros bigint;
alter table inventory_reservations add constraint inventory_location_reservation_context check (
 source_type is distinct from 'LOCATION_COMMAND' or
 (source_id is not null and source_id<>'' and location_id is not null
  and batch_code is not null and batch_code<>'' and quality_status='GOOD'
  and source_context is not null and source_request_id is not null
  and order_id is null and order_item_id is null
  and original_qty_micros is not null and qty_micros is not null and released_micros is not null
  and original_qty_micros>0 and qty_micros>=0 and released_micros>=0
  and original_qty_micros=qty_micros+released_micros
  and ((qty_micros>0 and status='RESERVED') or (qty_micros=0 and status='RELEASED')))
);
create unique index inventory_location_reservation_source_once
 on inventory_reservations(tenant_id,source_id) where source_type='LOCATION_COMMAND';
create table inventory_location_reservation_events (
 tenant_id uuid not null references tenants(tenant_id),
 request_id text not null,
 reservation_id uuid not null references inventory_reservations(reservation_id),
 ledger_id uuid not null,
 delta_micros bigint not null check(delta_micros<>0),
 remaining_micros bigint not null check(remaining_micros>=0),
 released_micros bigint not null check(released_micros>=0),
 location_reserved_micros bigint not null check(location_reserved_micros>=0),
 created_at timestamptz not null,
 primary key(tenant_id,request_id),
 unique(tenant_id,ledger_id),
 foreign key(tenant_id,ledger_id) references shared_stock_ledgers(tenant_id,ledger_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id)
);
create index inventory_location_reservation_events_source
 on inventory_location_reservation_events(tenant_id,reservation_id);

create function guard_inventory_location_reservation() returns trigger language plpgsql as $$
begin
 if old.source_type is distinct from 'LOCATION_COMMAND' and tg_op='UPDATE'
  and new.source_type='LOCATION_COMMAND' then
  raise exception 'historical reservation cannot acquire invented location source evidence';
 end if;
 if old.source_type='LOCATION_COMMAND' then
  if tg_op='DELETE' then raise exception 'location reservation source is immutable'; end if;
  if (to_jsonb(new)-array['qty','qty_micros','released_micros','status','updated_at'])
   is distinct from
   (to_jsonb(old)-array['qty','qty_micros','released_micros','status','updated_at']) then
   raise exception 'location reservation source is immutable';
  end if;
  if new.qty_micros>old.qty_micros or new.released_micros<old.released_micros then
   raise exception 'location reservation cannot increase or undo a release';
  end if;
 end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inventory_location_reservation_immutable before update or delete
 on inventory_reservations for each row execute function guard_inventory_location_reservation();
create function guard_inventory_location_reservation_event() returns trigger language plpgsql as $$
begin raise exception 'location reservation event is immutable'; end $$;
create trigger inventory_location_reservation_event_immutable before update or delete
 on inventory_location_reservation_events for each row
 execute function guard_inventory_location_reservation_event();

-- A committed source update must equal its immutable canonical command events.
-- This also fences old reservation writers; they cannot mutate these sources independently.
create function verify_inventory_location_reservation_commit() returns trigger language plpgsql as $$
declare r inventory_reservations%rowtype; total numeric; first_delta numeric;
begin
 select * into r from inventory_reservations where reservation_id=new.reservation_id;
 if r.source_type is distinct from 'LOCATION_COMMAND' then return null; end if;
 select coalesce(sum(e.delta_micros),0),
  coalesce(sum(e.delta_micros) filter(where e.delta_micros>0),0)
 into total,first_delta from inventory_location_reservation_events e
 where e.tenant_id=r.tenant_id and e.reservation_id=r.reservation_id;
 if total<>r.qty_micros or first_delta<>r.original_qty_micros or exists(
  select 1 from inventory_location_reservation_events e
  left join command_receipts c on c.tenant_id=e.tenant_id and c.request_id=e.request_id
  left join shared_stock_ledgers s on s.ledger_id=e.ledger_id and s.tenant_id=e.tenant_id
  left join inventory_ledgers l on l.tenant_id=e.tenant_id
   and l.ref_type='shared_stock_ledger' and l.ref_id=e.ledger_id::text
  where e.tenant_id=r.tenant_id and e.reservation_id=r.reservation_id and
   (c.status is distinct from 'SUCCEEDED'
    or c.movement_kind is distinct from case when e.delta_micros>0 then 'RESERVE' else 'RELEASE' end
    or c.effect_id is distinct from l.ledger_id::text
    or s.bucket is distinct from 'RESERVED' or s.delta_micros is distinct from e.delta_micros
    or s.request_id is distinct from e.request_id or s.source_line is distinct from r.source_id
    or s.warehouse_id is distinct from r.warehouse_id or s.sku_id is distinct from r.sku_id
    or s.location_id is distinct from r.location_id or s.batch_code is distinct from r.batch_code
    or s.quality_status is distinct from r.quality_status
    or l.delta_reserved_micros is distinct from e.delta_micros
    or l.delta_on_hand_micros is distinct from 0::bigint
    or l.delta_frozen_micros is distinct from 0::bigint
    or l.delta_in_transit_micros is distinct from 0::bigint)) then
  raise exception 'location reservation requires matching committed canonical command events';
 end if;
 return null;
end $$;
create constraint trigger inventory_location_reservation_commit
 after insert or update on inventory_reservations deferrable initially deferred
 for each row execute function verify_inventory_location_reservation_commit();
create constraint trigger inventory_location_reservation_event_commit
 after insert on inventory_location_reservation_events deferrable initially deferred
 for each row execute function verify_inventory_location_reservation_commit();
