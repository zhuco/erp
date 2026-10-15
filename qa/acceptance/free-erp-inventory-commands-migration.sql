\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory command QA identity required';
 end if;
end $$;
-- Add original-result evidence without inventing snapshots for legacy commands.
alter table command_receipts add column if not exists movement_kind text;
alter table command_receipts add column if not exists actor_user_id uuid;
alter table command_receipts add column if not exists warehouse_id uuid;
alter table command_receipts add column if not exists sku_id uuid;
alter table command_receipts add column if not exists original_result jsonb;

alter table command_receipts add constraint command_receipts_movement_context check (
 (movement_kind is null and actor_user_id is null and warehouse_id is null
  and sku_id is null and original_result is null)
 or
 (movement_kind is not null and movement_kind in ('INBOUND','OUTBOUND','ADJUSTMENT')
  and actor_user_id is not null and warehouse_id is not null and sku_id is not null
  and (status <> 'SUCCEEDED' or (original_result is not null
    and jsonb_typeof(original_result) = 'object')))
);

-- A committed original result cannot become a later balance or another command.
create or replace function guard_inventory_movement_command_receipt() returns trigger
language plpgsql as $$
begin
 if old.movement_kind is not null and old.status = 'SUCCEEDED' then
  if tg_op = 'DELETE' then
   raise exception 'committed inventory movement receipt is immutable';
  end if;
  if new is distinct from old then
   raise exception 'committed inventory movement receipt is immutable';
  end if;
 end if;
 if tg_op = 'DELETE' then return old; end if;
 return new;
end $$;
create trigger inventory_movement_receipt_immutable before update or delete
 on command_receipts for each row execute function guard_inventory_movement_command_receipt();

commit;
