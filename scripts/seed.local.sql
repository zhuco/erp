-- 本地联调演示租户、店铺、角色和权限种子。
-- 执行前请先运行数据库迁移，并把下方占位 UUID 替换为实际返回值。

insert into tenants(name) values ('demo') returning tenant_id;

-- 用上一步返回的 tenant_id 替换 <TENANT_UUID>。
insert into shops(tenant_id, platform, platform_shop_id, shop_name)
values ('<TENANT_UUID>', 'taobao', 'demo_shop_001', '淘宝演示店') returning shop_id;

-- 创建管理员角色。
insert into roles(tenant_id, name) values ('<TENANT_UUID>', 'admin') returning role_id;

-- 用上一步返回的 role_id 替换 <ROLE_UUID>。
insert into role_permissions(tenant_id, role_id, perm_code) values
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.PERMISSION.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.ROLE.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.ROLE.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.USER.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.USER.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.AUDIT.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.SETTINGS.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.SETTINGS.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.DLQ.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'SYS.DLQ.REPLAY'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.ORDER.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.ORDER.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.ORDER.RAW'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.ORDER.REVIEW'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.SHIPPING.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.SHIPPING.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.AFTERSALES.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.AFTERSALES.RAW'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OMS.AFTERSALES.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'WMS.INVENTORY.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'WMS.INVENTORY.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.SKU.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.SKU.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.CHANNEL.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.CHANNEL.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.MAPPING.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.MAPPING.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.WAREHOUSE.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.WAREHOUSE.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.CARRIER.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.CARRIER.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.LOGISTICS_PRODUCT.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.LOGISTICS_PRODUCT.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.LOGISTICS_PRICE.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.LOGISTICS_PRICE.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.LOGISTICS_MATCH_RULE.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'BASIC.LOGISTICS_MATCH_RULE.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.DEVICE.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.DEVICE.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.TEMPLATE.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.TEMPLATE.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.JOB.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.JOB.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.MAPPING.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'PRINT.MAPPING.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OPS.OPERATION_LOG.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OPS.REPORT.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OPS.EXPORT.READ'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'OPS.EXPORT.WRITE'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'finance.read'),
  ('<TENANT_UUID>', '<ROLE_UUID>', 'finance.write');

-- 管理员用户密码哈希需通过 erp-api 启动端点或本地辅助工具生成。
-- insert into users(tenant_id, email, password_hash) values ('<TENANT_UUID>', 'admin@example.com', '<HASH>') returning user_id;

-- 将管理员用户绑定到管理员角色。
-- insert into user_roles(tenant_id, user_id, role_id) values ('<TENANT_UUID>', '<USER_UUID>', '<ROLE_UUID>');

