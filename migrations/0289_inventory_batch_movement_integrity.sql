-- Preserve existing original effects; add guards without rewriting any row.
alter table inventory_batch_movements add constraint inventory_batch_movements_metadata_identity
 check(coalesce(batch_metadata_snapshot->>'tenant_id'=tenant_id::text
  and batch_metadata_snapshot->>'sku_id'=sku_id::text
  and batch_metadata_snapshot->>'batch_id'=batch_id::text,false));

create function guard_inventory_ledger_batch_source() returns trigger language plpgsql as $$
begin
 if exists(select 1 from inventory_batch_movements
  where tenant_id=old.tenant_id and inventory_ledger_id=old.ledger_id) then
  raise exception 'inventory ledger with original batch effects is immutable';
 end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inventory_ledger_batch_source_immutable before update or delete on inventory_ledgers
 for each row execute function guard_inventory_ledger_batch_source();
