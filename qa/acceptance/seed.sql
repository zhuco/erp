\set ON_ERROR_STOP on

-- Deterministic QA-only fixture for erp_grid_acceptance.
-- Login: qa.acceptance@example.invalid / ErpQa!2026
begin;

insert into tenants (tenant_id, name, status)
values ('a1100000-0000-4000-8000-000000000001', '易格 REAL API 验收租户', 'ACTIVE')
on conflict (tenant_id) do update set name = excluded.name, status = excluded.status, updated_at = now();

insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name, name, status)
values ('a1100000-0000-4000-8000-000000000010', 'a1100000-0000-4000-8000-000000000001',
        'qa', 'qa_acceptance_shop', 'QA 验收店铺', 'QA 验收店铺', 'ACTIVE')
on conflict (shop_id) do update set shop_name = excluded.shop_name, name = excluded.name,
  status = excluded.status, updated_at = now();

insert into roles (role_id, tenant_id, name)
values ('a1100000-0000-4000-8000-000000000020', 'a1100000-0000-4000-8000-000000000001', 'QA acceptance operator')
on conflict (role_id) do update set name = excluded.name;

insert into users (user_id, tenant_id, email, password_hash, status)
values ('a1100000-0000-4000-8000-000000000030', 'a1100000-0000-4000-8000-000000000001',
        'qa.acceptance@example.invalid',
        '$argon2id$v=19$m=65536,t=3,p=4$e2sG+56d5ad9Zqhn3AyI0Q$+V/BdTqdWryDDwRw5r1Jz3knecMc7FwWB5FiPk5RDvw',
        'ACTIVE')
on conflict (user_id) do update set email = excluded.email, password_hash = excluded.password_hash,
  status = excluded.status, updated_at = now();

insert into user_roles (tenant_id, user_id, role_id)
values ('a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000030',
        'a1100000-0000-4000-8000-000000000020')
on conflict do nothing;

insert into role_permissions (tenant_id, role_id, perm_code)
select 'a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000020', code
from (values
  ('BASIC.SKU.READ'), ('BASIC.SKU.WRITE'), ('BASIC.WAREHOUSE.READ'),
  ('WMS.INVENTORY.READ'), ('WMS.INVENTORY.WRITE'), ('OPS.REPORT.READ'),
  ('OPS.EXPORT.READ'), ('OPS.EXPORT.WRITE')
) as p(code)
on conflict do nothing;

insert into permissions (tenant_id, code, name, module, description)
select 'a1100000-0000-4000-8000-000000000001', code, code, split_part(code, '.', 1), 'QA acceptance permission'
from (values
  ('BASIC.SKU.READ'), ('BASIC.SKU.WRITE'), ('BASIC.WAREHOUSE.READ'),
  ('WMS.INVENTORY.READ'), ('WMS.INVENTORY.WRITE'), ('OPS.REPORT.READ'),
  ('OPS.EXPORT.READ'), ('OPS.EXPORT.WRITE')
) as p(code)
on conflict (tenant_id, code) do nothing;

insert into warehouses (warehouse_id, tenant_id, name, code, is_default, status)
values ('a1100000-0000-4000-8000-000000000040', 'a1100000-0000-4000-8000-000000000001',
        'QA 主仓', 'QA-WH-01', true, 'ACTIVE')
on conflict (warehouse_id) do update set name = excluded.name, code = excluded.code,
  is_default = excluded.is_default, status = excluded.status, updated_at = now();

insert into warehouse_locations (location_id, tenant_id, warehouse_id, code, name, priority, pick_zone, barcode)
values ('a1100000-0000-4000-8000-000000000041', 'a1100000-0000-4000-8000-000000000001',
        'a1100000-0000-4000-8000-000000000040', 'A-01-01', 'QA 拣货位', 10, 'A', 'QA-A-01-01')
on conflict (location_id) do update set name = excluded.name, priority = excluded.priority,
  pick_zone = excluded.pick_zone, barcode = excluded.barcode, updated_at = now();

insert into user_shop_access (tenant_id, user_id, shop_id)
values ('a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000030',
        'a1100000-0000-4000-8000-000000000010') on conflict do nothing;
insert into user_warehouse_access (tenant_id, user_id, warehouse_id)
values ('a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000030',
        'a1100000-0000-4000-8000-000000000040') on conflict do nothing;

insert into products (product_id, tenant_id, product_code, name, brand, category, base_unit,
                      primary_barcode, status, default_warehouse_id, attrs, product_tags)
values
 ('a1100000-0000-4000-8000-000000000101', 'a1100000-0000-4000-8000-000000000001', 'QA-P-001', 'QA 蓝色保温杯', '易格 QA', '杯具', '个', 'QA-P-001-BC', 'ACTIVE', 'a1100000-0000-4000-8000-000000000040', '{"color":"blue"}', array['QA','日用']),
 ('a1100000-0000-4000-8000-000000000102', 'a1100000-0000-4000-8000-000000000001', 'QA-P-002', 'QA 红色保温杯', '易格 QA', '杯具', '个', 'QA-P-002-BC', 'ACTIVE', 'a1100000-0000-4000-8000-000000000040', '{"color":"red"}', array['QA','日用']),
 ('a1100000-0000-4000-8000-000000000103', 'a1100000-0000-4000-8000-000000000001', 'QA-P-003', 'QA 帆布袋', '易格 QA', '箱包', '个', 'QA-P-003-BC', 'ACTIVE', 'a1100000-0000-4000-8000-000000000040', '{"color":"natural"}', array['QA','服饰']),
 ('a1100000-0000-4000-8000-000000000104', 'a1100000-0000-4000-8000-000000000001', 'QA-P-004', 'QA 停用样品', '易格 QA', '样品', '个', 'QA-P-004-BC', 'INACTIVE', 'a1100000-0000-4000-8000-000000000040', '{}', array['QA'])
on conflict (product_id) do update set product_code=excluded.product_code, name=excluded.name, brand=excluded.brand,
 category=excluded.category, base_unit=excluded.base_unit, primary_barcode=excluded.primary_barcode,
 status=excluded.status, default_warehouse_id=excluded.default_warehouse_id, attrs=excluded.attrs,
 product_tags=excluded.product_tags, updated_at=now();

insert into skus (sku_id, tenant_id, product_id, sku_code, barcode, name, spec, cost_cent, status, is_active)
values
 ('a1100000-0000-4000-8000-000000000201', 'a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000101', 'QA-SKU-001', 'QA-SKU-001-BC', '蓝杯 500ml', '{"容量":"500ml"}', 2599, 'ACTIVE', true),
 ('a1100000-0000-4000-8000-000000000202', 'a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000102', 'QA-SKU-002', 'QA-SKU-002-BC', '红杯 350ml', '{"容量":"350ml"}', 2199, 'ACTIVE', true),
 ('a1100000-0000-4000-8000-000000000203', 'a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000103', 'QA-SKU-003', 'QA-SKU-003-BC', '帆布袋 标准款', '{"尺寸":"标准"}', 1299, 'ACTIVE', true),
 ('a1100000-0000-4000-8000-000000000204', 'a1100000-0000-4000-8000-000000000001', 'a1100000-0000-4000-8000-000000000104', 'QA-SKU-004', 'QA-SKU-004-BC', '停用样品', '{}', 99, 'INACTIVE', false)
on conflict (sku_id) do update set sku_code=excluded.sku_code, barcode=excluded.barcode, name=excluded.name,
 spec=excluded.spec, cost_cent=excluded.cost_cent, status=excluded.status, is_active=excluded.is_active, updated_at=now();

insert into product_warehouses (tenant_id, product_id, warehouse_id, is_default)
select 'a1100000-0000-4000-8000-000000000001', product_id,
       'a1100000-0000-4000-8000-000000000040', true
from products where tenant_id='a1100000-0000-4000-8000-000000000001'
on conflict (tenant_id, product_id, warehouse_id) do update set is_default=true, updated_at=now();

insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand, reserved, frozen, in_transit)
values
 ('a1100000-0000-4000-8000-000000000001','a1100000-0000-4000-8000-000000000040','a1100000-0000-4000-8000-000000000201',120,5,2,10),
 ('a1100000-0000-4000-8000-000000000001','a1100000-0000-4000-8000-000000000040','a1100000-0000-4000-8000-000000000202',60,8,0,0),
 ('a1100000-0000-4000-8000-000000000001','a1100000-0000-4000-8000-000000000040','a1100000-0000-4000-8000-000000000203',15,0,0,25),
 ('a1100000-0000-4000-8000-000000000001','a1100000-0000-4000-8000-000000000040','a1100000-0000-4000-8000-000000000204',0,0,0,0)
on conflict (tenant_id, warehouse_id, sku_id) do update set on_hand=excluded.on_hand,
 reserved=excluded.reserved, frozen=excluded.frozen, in_transit=excluded.in_transit, updated_at=now();

insert into inventory_location_balances (tenant_id, warehouse_id, location_id, sku_id, on_hand)
select tenant_id, warehouse_id, 'a1100000-0000-4000-8000-000000000041', sku_id, on_hand
from inventory_balances where tenant_id='a1100000-0000-4000-8000-000000000001'
on conflict (tenant_id, warehouse_id, location_id, sku_id) do update set on_hand=excluded.on_hand, updated_at=now();

commit;
