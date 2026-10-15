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
const output = path.join(directory, `free-erp-batch-writes-openapi${label ? `-${label}` : ''}.json`);
if (fs.existsSync(output)) throw new Error('Refusing contract evidence overwrite');
const sources = ['openapi/src/openapi.yaml', 'openapi/src/paths.yaml', 'openapi/src/components.yaml', 'openapi/erp_api_v1.yaml'];
const digest = name => crypto.createHash('sha256').update(fs.readFileSync(path.join(root, name))).digest('hex');
const before = Object.fromEntries(sources.map(name => [name, digest(name)]));
const paths = loadOpenApiDocument(path.join(root, sources[1]));
const components = loadOpenApiDocument(path.join(root, sources[2]));
const bundled = loadOpenApiDocument(path.join(root, sources[3]));
const routes = ['inbounds', 'outbounds', 'adjustments'].map(action => `/v1/tenants/{tenant_id}/inventory/${action}`);
const quantities = ['before_on_hand_micros', 'after_on_hand_micros', 'before_reserved_micros', 'after_reserved_micros', 'delta_on_hand_micros', 'delta_reserved_micros'];
const errors = [];
try {
  assert.deepEqual(paths, bundled.paths, 'All source/generated paths must match');
  assert.deepEqual(components, bundled.components, 'All source/generated components must match');
  validateOpenApiGate({ openapiDocument: bundled, checks: routes.map(pathKey => ({
    pathKey, method: 'post', requiredPermissions: ['inventory.write'],
    requiredStatusCodes: ['200', '400', '401', '403', '404', '409', '422'],
  })) });
  for (const route of routes) {
    const operation = paths[route].post;
    assert.ok(!/batch_id.{0,30}(?:unsupported|not supported)/i.test(operation.description ?? ''), `${route}: stale UUID support description`);
    assert.ok(operation.responses['404'].description.includes('UUID'), `${route}: owned batch error`);
    assert.ok(operation.responses['422'].description.includes('inventory_batch_limit'), `${route}: capacity rejection`);
    assert.ok(operation.responses['422'].description.includes('1000'), `${route}: capacity budget`);
    assert.ok(JSON.stringify(operation.responses['200']).includes('InventoryCommandResponse'), `${route}: original response`);
  }
  for (const name of ['InventoryMovementRequest', 'InventoryAdjustmentRequest']) {
    const batch = components.schemas[name].properties.batch_id;
    assert.equal(batch.type, 'string');
    assert.equal(batch.format, 'uuid');
  }
  const response = components.schemas.InventoryCommandResponse.properties;
  assert.equal(response.batch_id.format, 'uuid');
  assert.equal(response.batch_effects.type, 'array');
  assert.equal(response.batch_effects.items.$ref, '#/components/schemas/InventoryBatchEffect');
  const effect = components.schemas.InventoryBatchEffect;
  for (const name of quantities) {
    assert.ok(effect.required.includes(name), `Missing required exact quantity ${name}`);
    assert.equal(effect.properties[name].type, 'string', `${name}: exact decimal text`);
    assert.ok(effect.properties[name].pattern, `${name}: canonical text validation`);
  }
  for (const name of ['batch_movement_id', 'inventory_ledger_id', 'batch_id']) {
    assert.ok(effect.required.includes(name));
    assert.equal(effect.properties[name].format, 'uuid');
  }
  for (const name of ['decision_at', 'created_at']) assert.equal(effect.properties[name].format, 'date-time');
  for (const name of ['tenant_id', 'sku_id', 'batch_id', 'batch_no']) {
    assert.ok(effect.properties.batch_metadata_snapshot.required.includes(name), `Owned snapshot ${name}`);
  }
} catch (error) {
  errors.push(error instanceof Error ? error.message : String(error));
}
const after = Object.fromEntries(sources.map(name => [name, digest(name)]));
for (const name of sources) if (before[name] !== after[name]) errors.push(`Input changed during contract check: ${name}`);
const report = {
  created_at: new Date().toISOString(), status: errors.length ? 'FAIL' : 'PASS',
  qualification: 'Static current source/generated contract only. Runtime evidence binds separately to each recorded build input.',
  routes, quantities, source_sha256: before, after_sha256: after,
  all_source_generated_paths_equal: JSON.stringify(paths) === JSON.stringify(bundled.paths),
  all_source_generated_components_equal: JSON.stringify(components) === JSON.stringify(bundled.components),
  errors,
};
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ status: report.status, errors }));
if (errors.length) process.exitCode = 1;
