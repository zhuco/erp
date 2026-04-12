-- 0049: Procurement order fields (warehouse, eta, remark)
alter table purchase_orders
  add column if not exists warehouse_id uuid references warehouses(warehouse_id);
alter table purchase_orders
  add column if not exists expected_at timestamptz;
alter table purchase_orders
  add column if not exists remark text;

create index if not exists idx_purchase_orders_tenant_warehouse
  on purchase_orders(tenant_id, warehouse_id, created_at desc);
create index if not exists idx_purchase_orders_tenant_expected
  on purchase_orders(tenant_id, expected_at desc);
