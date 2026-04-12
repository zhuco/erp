-- 0026: Inventory balances pagination index

create index if not exists idx_inventory_balances_tenant_updated
  on inventory_balances(tenant_id, updated_at desc, warehouse_id desc, sku_id desc);
