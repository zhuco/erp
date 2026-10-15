import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadOpenApiDocument } from '../../scripts/openapi/lib/openapi_tools.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const stage = process.argv[2];
const logName = process.argv[3];
assert.ok(stage && /^[a-z0-9-]+$/.test(stage) && logName);
const target = path.join(root, `qa/acceptance/free-erp-atomic-effects-${stage}-contract.json`);
assert.ok(!fs.existsSync(target), 'Keep historical evidence');
const names = ['openapi/src/openapi.yaml', 'openapi/src/paths.yaml', 'openapi/src/components.yaml', 'openapi/erp_api_v1.yaml',
  'crates/erp-api/src/manufacturing/atomic_effects.rs', 'crates/erp-repo/src/shared_contract/atomic_effects.rs', logName];
const sha = name => crypto.createHash('sha256').update(fs.readFileSync(path.join(root, name))).digest('hex');
const before = Object.fromEntries(names.map(name => [name, sha(name)]));
const bundle = loadOpenApiDocument(path.join(root, names[3]));
const paths = loadOpenApiDocument(path.join(root, names[1]));
const components = loadOpenApiDocument(path.join(root, names[2]));
const route = '/v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/atomic-effects';
const errors = [];
let success = 0, refused = 0;
function validate(schema, value, label) {
  if (schema.$ref) return validate(components.schemas[schema.$ref.split('/').at(-1)], value, label);
  if (schema.nullable && value === null) return;
  if (schema.oneOf) {
    assert.equal(schema.oneOf.filter(candidate => {
      try { validate(candidate, value, label); return true; } catch { return false; }
    }).length, 1, label);
    return;
  }
  if (schema.enum) assert.ok(schema.enum.includes(value), label);
  if (schema.type === 'object') {
    assert.ok(value && typeof value === 'object' && !Array.isArray(value), label);
    for (const field of schema.required ?? []) assert.ok(Object.hasOwn(value, field), `${label}.${field}`);
    if (schema.additionalProperties === false) for (const field of Object.keys(value)) assert.ok(schema.properties[field], `${label}.${field}`);
    for (const [field, item] of Object.entries(value)) if (schema.properties?.[field]) validate(schema.properties[field], item, `${label}.${field}`);
  } else if (schema.type === 'array') {
    assert.ok(Array.isArray(value), label);
    if (schema.minItems !== undefined) assert.ok(value.length >= schema.minItems, label);
    if (schema.maxItems !== undefined) assert.ok(value.length <= schema.maxItems, label);
    for (const item of value) validate(schema.items, item, label);
  } else if (schema.type === 'string') {
    assert.equal(typeof value, 'string', label);
    if (schema.minLength !== undefined) assert.ok(value.length >= schema.minLength, label);
    if (schema.pattern) assert.ok(new RegExp(schema.pattern).test(value), label);
    if (schema.format === 'uuid') assert.ok(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(value), label);
  } else if (schema.type === 'integer') {
    assert.ok(Number.isSafeInteger(value), label);
    if (schema.minimum !== undefined) assert.ok(value >= schema.minimum, label);
  } else if (schema.type === 'boolean') assert.equal(typeof value, 'boolean', label);
}
try {
  assert.deepEqual(bundle.paths, paths);
  assert.deepEqual(bundle.components, components);
  const operation = paths[route].post;
  assert.deepEqual(Object.keys(operation.responses).sort(), ['200', '400', '403', '404', '409', '422', '500']);
  const api = fs.readFileSync(path.join(root, names[4]), 'utf8');
  const fields = [...api.match(/struct AtomicEffectRequest\s*\{([^}]+)\}/)[1].matchAll(/^\s*(\w+)\s*:/gm)].map(row => row[1]).sort();
  assert.deepEqual(fields, Object.keys(components.schemas.AtomicEffectRequest.properties).sort());
  assert.ok(api.includes('ctx.production_release_actor(&rctx)'));
  const log = fs.readFileSync(path.join(root, logName), 'utf8');
  for (const match of log.matchAll(/QA production_release_http tenant=([a-f0-9-]+) path=([^ ]+) input=(.+?) status=(\d+) response=([^\r\n]+)/g)) {
    if (!match[2].endsWith('/atomic-effects')) continue;
    assert.ok(operation.responses[match[4]], `Actual status ${match[4]}`);
    if (match[4] === '200') {
      validate(components.schemas.AtomicEffectRequest, JSON.parse(match[3]), 'actual request');
      validate(components.schemas.AtomicEffectResponse, JSON.parse(match[5]), 'actual response');
      success++;
    } else refused++;
  }
  assert.ok(success >= 20 && refused >= 10, 'Real success and refusal samples');
} catch (error) { errors.push(error.message); }
const after = Object.fromEntries(names.map(name => [name, sha(name)]));
for (const name of names) if (before[name] !== after[name]) errors.push(`Input changed: ${name}`);
const report = { utc: new Date().toISOString(), status: errors.length ? 'FAIL' : 'PASS', route,
  successful_responses: success, actual_refusals: refused, source_sha256: before, after_sha256: after, errors,
  qualification: 'Actual single-line ISSUE samples and exact current source/generated contract; no normal login, dual-client, whole-document or Release acceptance.' };
fs.writeFileSync(target, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ status: report.status, successful_responses: success, actual_refusals: refused, errors }));
if (errors.length) process.exitCode = 1;
