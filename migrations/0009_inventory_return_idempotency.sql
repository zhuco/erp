-- 0009: Inventory return ledger idempotency

create unique index if not exists idx_inventory_ledgers_return_unique
  on inventory_ledgers(tenant_id, ref_type, ref_id, sku_id)
  where ref_type = 'AFTER_SALE_CASE';
