-- A shared projection that cites a canonical inventory effect is a historical fact.
-- Unlinked legacy rows remain a separate reconciliation scope.
create or replace function guard_inventory_linked_shared_ledger() returns trigger
language plpgsql as $$
begin
 if exists(select 1 from inventory_ledgers i where i.tenant_id=old.tenant_id
   and i.warehouse_id=old.warehouse_id and i.sku_id=old.sku_id
   and i.ref_type='shared_stock_ledger' and i.ref_id=old.ledger_id::text) then
  if tg_op='DELETE' then
   raise exception 'linked shared inventory projection is immutable';
  end if;
  if new is distinct from old then
   raise exception 'linked shared inventory projection is immutable';
  end if;
 end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inventory_linked_shared_ledger_guard before update or delete
 on shared_stock_ledgers for each row execute function guard_inventory_linked_shared_ledger();

-- Exact location/batch/quality sums use the full owned key, not a tenant-wide scan.
create index if not exists shared_stock_location_quantity_idx
 on shared_stock_ledgers(tenant_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket)
 include(delta_micros);
