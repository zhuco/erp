import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadOpenApiDocument } from '../../scripts/openapi/lib/openapi_tools.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, '../..');
const label = process.argv[2];
if (!label || !/^[a-z0-9-]+$/.test(label)) throw new Error('An immutable contract label is required');
const output = path.join(directory, `free-erp-production-release-openapi-${label}.json`);
if (fs.existsSync(output)) throw new Error('Refusing contract evidence overwrite');
const sources = ['openapi/src/openapi.yaml', 'openapi/src/paths.yaml', 'openapi/src/components.yaml',
  'openapi/erp_api_v1.yaml', 'crates/erp-api/src/manufacturing.rs', 'crates/erp-api/src/auth_context.rs'];
const digest = name => crypto.createHash('sha256').update(fs.readFileSync(path.join(root, name))).digest('hex');
const before = Object.fromEntries(sources.map(name => [name, digest(name)]));
const paths = loadOpenApiDocument(path.join(root, sources[1]));
const components = loadOpenApiDocument(path.join(root, sources[2]));
const bundle = loadOpenApiDocument(path.join(root, sources[3]));
const route = '/v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/receipts/{receipt_id}/release';
const errors = [];
const statuses = ['200', '400', '403', '404', '409', '422', '500'];
try {
  assert.deepEqual(paths, bundle.paths, 'All current source/generated paths must match');
  assert.deepEqual(components, bundle.components, 'All current source/generated components must match');
  const operation = paths[route].post;
  assert.deepEqual(operation.security, [{ bearerAuth: [] }]);
  for (const status of statuses) assert.ok(operation.responses[status], `Real response status ${status}`);
  assert.equal(operation.requestBody.content['application/json'].schema.$ref, '#/components/schemas/ProductionReleaseRequest');
  assert.equal(operation.responses['200'].content['application/json'].schema.$ref, '#/components/schemas/ProductionReleaseResponse');
  const request = components.schemas.ProductionReleaseRequest;
  assert.equal(request.additionalProperties, false);
  assert.deepEqual(Object.keys(request.properties), ['request_id'], 'Client cannot supply trusted actor or inspection decision');
  assert.deepEqual(request.required, ['request_id']);
  const response = components.schemas.ProductionReleaseResponse;
  for (const name of ['available_micros', 'available_micros_exact', 'quantity_scale']) assert.ok(response.required.includes(name));
  assert.equal(response.properties.available_micros_exact.type, 'string');
  assert.equal(response.properties.available_micros_exact.pattern, '^[0-9]+$');
  assert.deepEqual(response.properties.quantity_scale.enum, [1_000_000]);
  const api = fs.readFileSync(path.join(root, sources[4]), 'utf8');
  const requestSource = api.match(/struct ProductionReleaseRequest\s*\{([^}]+)\}/)?.[1];
  assert.deepEqual(Array.from(requestSource?.matchAll(/(\w+)\s*:/g) ?? [], match => match[1]), ['request_id']);
  assert.ok(api.includes('ctx.production_release_actor(&request_ctx)'));
  assert.ok(api.includes('"available_micros_exact": available.to_string()'));
  assert.ok(api.includes('"quantity_scale": 1_000_000'));
  for (const term of ['original committed total', 'verified mapping', 'mandatory operation and audit', 'Net physical']) {
    assert.ok(operation.description.includes(term), `Release boundary ${term}`);
  }
} catch (error) {
  errors.push(error instanceof Error ? error.message : String(error));
}
const after = Object.fromEntries(sources.map(name => [name, digest(name)]));
for (const name of sources) if (before[name] !== after[name]) errors.push(`Input changed during contract check: ${name}`);
const report = {
  created_at: new Date().toISOString(), status: errors.length ? 'FAIL' : 'PASS',
  qualification: 'Static current source/generated release contract only; actual runtime results bind separately to their build inputs.',
  route, actual_statuses_checked: statuses,
  all_source_generated_paths_equal: JSON.stringify(paths) === JSON.stringify(bundle.paths),
  all_source_generated_components_equal: JSON.stringify(components) === JSON.stringify(bundle.components),
  source_sha256: before, after_sha256: after, errors,
};
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ status: report.status, errors }));
if (errors.length) process.exitCode = 1;
