import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadOpenApiDocument, validateOpenApiGate } from '../../scripts/openapi/lib/openapi_tools.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, '../..');
const label = process.argv[2];
if (label !== undefined && !/^[a-z0-9-]+$/.test(label)) throw new Error('Invalid immutable contract label');
const output = path.join(directory, `free-erp-fulfillment-cost-batch-openapi${label ? `-${label}` : ''}.json`);
if (fs.existsSync(output)) throw new Error('Refusing contract evidence overwrite');
const route = '/v1/tenants/{tenant_id}/oms/shipments/{shipment_id}/ship';
const sources = ['openapi/src/paths.yaml', 'openapi/erp_api_v1.yaml'];
const raw = sources.map(name => fs.readFileSync(path.join(root, name)));
const source = loadOpenApiDocument(path.join(root, sources[0]));
const bundled = loadOpenApiDocument(path.join(root, sources[1]));
const errors = [];
try {
  assert.deepEqual(source[route].post, bundled.paths[route].post);
  validateOpenApiGate({ openapiDocument: bundled, checks: [{
    pathKey: route, method: 'post', requiredPermissions: ['shipping.write', 'inventory.write'],
    requiredStatusCodes: ['200', '400', '401', '403', '404', '409', '422', '500'],
  }] });
  const conflict = bundled.paths[route].post.responses['409'].description;
  const invalid = bundled.paths[route].post.responses['422'].description;
  for (const code of ['fulfillment_cost_unavailable', 'fulfillment_cost_invalid', 'fulfillment_cost_conflict', 'inventory_reconciliation_required', 'inventory_insufficient']) assert.ok(conflict.includes(code), code);
  for (const code of ['fulfillment_cost_overflow', 'inventory_batch_limit', 'fulfillment_scope_limit']) assert.ok(invalid.includes(code), code);
} catch (error) {
  errors.push(error instanceof Error ? error.message : String(error));
}
const report = {
  status: errors.length ? 'FAIL' : 'PASS', route, source_bundle_operation_equal: errors.length === 0,
  permissions: ['shipping.write', 'inventory.write'], status_codes: ['200', '400', '401', '403', '404', '409', '422', '500'],
  source_sha256: Object.fromEntries(sources.map((name, i) => [name, crypto.createHash('sha256').update(raw[i]).digest('hex')])),
  operation: bundled.paths[route].post,
  qualification: 'Static source/generated contract only; actual authenticated route cases are separate.', errors,
};
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, errors }));
if (errors.length) process.exitCode = 1;
