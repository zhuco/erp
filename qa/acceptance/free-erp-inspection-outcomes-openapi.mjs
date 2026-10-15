import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadOpenApiDocument } from '../../scripts/openapi/lib/openapi_tools.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const stage = process.argv[2];
const testStage = process.argv[3] ?? stage;
if (![stage, testStage].every(value => value && /^[a-z0-9-]+$/.test(value))) throw new Error('Immutable evidence labels required');
const prefix = 'qa/acceptance/free-erp-inspection-outcomes-';
const output = path.join(root, `${prefix}openapi-${stage}.json`);
if (fs.existsSync(output)) throw new Error('Refusing evidence overwrite');
const inputs = ['openapi/src/openapi.yaml', 'openapi/src/paths.yaml', 'openapi/src/components.yaml',
  'openapi/erp_api_v1.yaml', 'crates/erp-api/src/manufacturing.rs',
  'crates/erp-api/src/manufacturing/inspection_outcomes.rs', `${prefix}${testStage}-test.log`];
const sha = name => crypto.createHash('sha256').update(fs.readFileSync(path.join(root, name))).digest('hex');
const before = Object.fromEntries(inputs.map(name => [name, sha(name)]));
const paths = loadOpenApiDocument(path.join(root, inputs[1]));
const components = loadOpenApiDocument(path.join(root, inputs[2]));
const bundle = loadOpenApiDocument(path.join(root, inputs[3]));
const errors = [];
let checkedBodies = 0;
const schemas = components.schemas;
const routes = [
  ['post', '/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings', 'InspectionReadingRequest', 'InspectionReadingResponse'],
  ['post', '/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/reinspections', 'InspectionReinspectionRequest', 'InspectionReadingResponse'],
  ['post', '/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings/{item_id}/reviews', 'InspectionReadingReviewRequest', 'InspectionReadingReviewResponse'],
  ['get', '/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings', null, 'InspectionReadingHistoryResponse'],
];

// This bounded validator covers exactly the keywords in the measurement schemas.
// General OpenAPI validation is still provided by the registered project gates.
function validate(value, schema, location) {
  if (schema.$ref) return validate(value, schemas[schema.$ref.split('/').at(-1)], location);
  if (schema.oneOf) {
    const matched = schema.oneOf.filter(branch => validate(value, branch, location).length === 0).length;
    return matched === 1 ? [] : [`${location}: expected exactly one schema, matched ${matched}`];
  }
  if (schema.allOf) return schema.allOf.flatMap(branch => validate(value, branch, location));
  const problems = [];
  if (schema.enum && !schema.enum.some(item => JSON.stringify(item) === JSON.stringify(value))) problems.push(`${location}: enum`);
  if (value === null) return schema.nullable ? problems : [...problems, `${location}: null`];
  const type = Array.isArray(value) ? 'array' : typeof value;
  if (schema.type === 'integer' ? !Number.isInteger(value) : schema.type && schema.type !== type) return [...problems, `${location}: type ${type}`];
  if (type === 'object') {
    for (const key of schema.required ?? []) if (!Object.hasOwn(value, key)) problems.push(`${location}.${key}: missing`);
    for (const [key, entry] of Object.entries(value)) {
      if (schema.properties?.[key]) problems.push(...validate(entry, schema.properties[key], `${location}.${key}`));
      else if (schema.additionalProperties === false) problems.push(`${location}.${key}: unexpected`);
    }
  }
  if (type === 'array') {
    if (schema.maxItems !== undefined && value.length > schema.maxItems) problems.push(`${location}: array budget`);
    value.forEach((entry, index) => problems.push(...validate(entry, schema.items, `${location}[${index}]`)));
  }
  if (type === 'number') {
    if (schema.minimum !== undefined && value < schema.minimum) problems.push(`${location}: minimum`);
    if (schema.maximum !== undefined && value > schema.maximum) problems.push(`${location}: maximum`);
  }
  if (type === 'string') {
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) problems.push(`${location}: pattern`);
    if (schema.format === 'uuid' && !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)) problems.push(`${location}: uuid`);
    if (schema.format === 'date-time' && Number.isNaN(Date.parse(value))) problems.push(`${location}: date-time`);
  }
  return problems;
}

try {
  assert.deepEqual(paths, bundle.paths, 'Current source and generated paths');
  assert.deepEqual(components, bundle.components, 'Current source and generated schemas');
  for (const [method, url, request, response] of routes) {
    const operation = paths[url][method];
    assert.deepEqual(operation.security, [{ bearerAuth: [] }]);
    assert.equal(operation.responses['200'].content['application/json'].schema.$ref, `#/components/schemas/${response}`);
    for (const status of method === 'post' ? ['400', '403', '409', '422', '500'] : ['400', '403', '422', '500']) assert.ok(operation.responses[status], `${method} ${url}: ${status}`);
    if (request) {
      assert.equal(operation.requestBody.content['application/json'].schema.$ref, `#/components/schemas/${request}`);
      const source = fs.readFileSync(path.join(root, request === 'InspectionReadingRequest' ? inputs[4] : inputs[5]), 'utf8');
      const body = source.match(new RegExp(`struct ${request}\\s*\\{([\\s\\S]*?)^\\}`, 'm'))?.[1];
      assert.ok(body, `Current Rust DTO ${request}`);
      const declarations = [...body.matchAll(/^\s*(\w+)\s*:\s*([^,]+),/gm)];
      assert.deepEqual(Object.keys(schemas[request].properties).sort(), declarations.map(match => match[1]).sort());
      assert.deepEqual([...schemas[request].required].sort(), declarations.filter(match => !match[2].includes('Option<')).map(match => match[1]).sort());
      assert.equal(schemas[request].additionalProperties, false);
    }
  }
  assert.equal(schemas.InspectionReadingSnapshot.required.length, 23);
  assert.equal(schemas.InspectionReadingHistoryResponse.properties.history.maxItems, 20);
  assert.equal(schemas.InspectionReadingHistoryResponse.properties.legacy_reading_count.maximum, 1000);
  const raw = fs.readFileSync(path.join(root, inputs.at(-1)));
  const log = raw.toString(raw[0] === 255 && raw[1] === 254 ? 'utf16le' : 'utf8');
  assert.match(log, /QA controlled_test_exit=0/);
  for (const line of log.split(/\r?\n/)) {
    const match = line.match(/QA production_release_http tenant=\S+ path=(\S+) input=(.*) status=(\d+) response=(.*)/);
    if (!match || match[3] !== '200' || !match[1].startsWith('quality/inspection-freezes/')) continue;
    let schema;
    if (match[1].includes('/readings?')) schema = schemas.InspectionReadingHistoryResponse;
    else if (match[1].endsWith('/reviews')) schema = schemas.InspectionReadingReviewResponse;
    else if (/\/(readings|reinspections)$/.test(match[1])) schema = schemas.InspectionReadingResponse;
    else continue;
    errors.push(...validate(JSON.parse(match[4]), schema, match[1]));
    checkedBodies += 1;
  }
  assert.ok(checkedBodies >= 27, 'Actual successful measurement, review and history bodies required');
} catch (error) {
  errors.push(error instanceof Error ? error.message : String(error));
}
const after = Object.fromEntries(inputs.map(name => [name, sha(name)]));
for (const name of inputs) if (before[name] !== after[name]) errors.push(`Input changed during check: ${name}`);
fs.writeFileSync(output, JSON.stringify({created_at: new Date().toISOString(), status: errors.length ? 'FAIL' : 'PASS',
  scope: 'Four current measurement contracts and actual successful HTTP bodies; not real login, public networking or performance acceptance',
  routes, checked_successful_http_bodies: checkedBodies, inputs_before: before, inputs_after: after, errors}, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify({status: errors.length ? 'FAIL' : 'PASS', checked_successful_http_bodies: checkedBodies, errors}));
if (errors.length) process.exitCode = 1;
