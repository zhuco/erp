import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { loadOpenApiDocument, validateOpenApiGate } from './openapi/lib/openapi_tools.mjs';

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..');
const openapiPath = path.join(repoRoot, 'openapi', 'erp_api_v1.yaml');

function fail(message) {
  console.error(`[openapi-gate] ${message}`);
  process.exit(1);
}

if (!fs.existsSync(openapiPath)) {
  fail(`missing OpenAPI file: ${openapiPath}`);
}

const checks = [
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/skus',
    method: 'get',
    requiredPermissions: ['shop_products.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/download',
    method: 'post',
    requiredPermissions: ['shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['shop_product_tasks', 'shop_products', 'shop_product_skus'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/tasks',
    method: 'get',
    requiredPermissions: ['shop_products.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/create-skus',
    method: 'post',
    requiredPermissions: ['shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['shop_product_tasks', 'products', 'skus', 'channel_sku_mappings'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/auto-match',
    method: 'post',
    requiredPermissions: ['shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['shop_product_tasks', 'channel_sku_mappings', 'operation_logs'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/manual-match',
    method: 'post',
    requiredPermissions: ['shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['shop_product_tasks', 'channel_sku_mappings', 'operation_logs'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/delete-match',
    method: 'post',
    requiredPermissions: ['shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['shop_product_tasks', 'channel_sku_mappings'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/parse',
    method: 'post',
    requiredPermissions: ['shop_products.read', 'shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/bundle-match/auto',
    method: 'post',
    requiredPermissions: ['shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['shop_product_tasks', 'channel_sku_mappings', 'operation_logs'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/shop-products/bundle-match/manual',
    method: 'post',
    requiredPermissions: ['shop_products.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['shop_product_tasks', 'channel_sku_mappings', 'operation_logs'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/system/integrations/shops',
    method: 'get',
    requiredPermissions: ['SYS.SETTINGS.READ'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/oms/orders/exceptions/stats',
    method: 'get',
    requiredPermissions: ['orders.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/wms/labor/tasks',
    method: 'get',
    requiredPermissions: ['wms.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/wms/slotting/suggestions',
    method: 'get',
    requiredPermissions: ['wms.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/inventory/atp',
    method: 'get',
    requiredPermissions: ['inventory.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/inventory/reconciliation-center/check',
    method: 'post',
    requiredPermissions: ['inventory.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/planning/forecasts',
    method: 'get',
    requiredPermissions: ['inventory.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/planning/purchase-plans/generate',
    method: 'post',
    requiredPermissions: ['inventory.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/after-sales/{after_sales_id}/exchange',
    method: 'post',
    requiredPermissions: ['aftersales.write', 'shipping.write', 'inventory.write'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/fin/profit',
    method: 'get',
    requiredPermissions: ['finance.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/stats/health',
    method: 'get',
    requiredPermissions: ['stats.read', 'reports.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/stats/{stats_type}',
    method: 'get',
    requiredPermissions: ['stats.read', 'reports.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/system/frontend-route-diagnostics',
    method: 'get',
    requiredPermissions: ['stats.read', 'reports.read', 'operation_logs.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/system/frontend-route-diagnostics',
    method: 'post',
    requiredPermissions: ['stats.read', 'reports.read', 'operation_logs.read'],
    requiredStatusCodes: ['200', '400', '401', '403'],
    requiredPersistenceTables: ['operation_logs'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/system/workflows',
    method: 'get',
    requiredPermissions: ['SYS.SETTINGS.READ'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
  {
    pathKey: '/v1/tenants/{tenant_id}/system/workflows',
    method: 'put',
    requiredPermissions: ['SYS.SETTINGS.WRITE'],
    requiredStatusCodes: ['200', '400', '401', '403'],
  },
];

try {
  const openapiDocument = loadOpenApiDocument(openapiPath);
  validateOpenApiGate({ openapiDocument, checks });
  console.log('[openapi-gate] OpenAPI gate verification passed');
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
