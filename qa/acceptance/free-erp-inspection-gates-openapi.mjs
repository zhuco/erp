import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadOpenApiDocument } from '../../scripts/openapi/lib/openapi_tools.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const stage = process.argv[2];
if (!stage || !/^[a-z0-9-]+$/.test(stage)) throw new Error('Immutable stage required');
const output = path.join(root, `qa/acceptance/free-erp-inspection-gates-openapi-${stage}.json`);
if (fs.existsSync(output)) throw new Error('Historical evidence exists');
const inputs = ['openapi/src/paths.yaml', 'openapi/src/components.yaml', 'openapi/src/openapi.yaml',
  'openapi/erp_api_v1.yaml', 'crates/erp-api/src/manufacturing.rs', 'crates/erp-api/src/sales_orders.rs',
  'crates/erp-api/src/manufacturing/inspection_outcomes.rs', 'qa/acceptance/free-erp-inspection-gates-full-test.log'];
const sha = name => crypto.createHash('sha256').update(fs.readFileSync(path.join(root, name))).digest('hex');
const before = Object.fromEntries(inputs.map(name => [name, sha(name)]));
const paths = loadOpenApiDocument(path.join(root, inputs[0]));
const components = loadOpenApiDocument(path.join(root, inputs[1]));
const bundle = loadOpenApiDocument(path.join(root, inputs[3]));
const schemas = components.schemas;
const errors = [];
let checked = 0, rejected = 0;
const routes = [
  ['/v1/tenants/{tenant_id}/sales-orders/{sales_order_id}/deliveries','SalesDeliveryRequest','SalesDeliveryResponse',inputs[5]],
  ['/v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/materials','MaterialMoveRequest','MaterialMoveResponse',inputs[4]],
  ['/v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/receipts','ProductionReceiptRequest','ProductionReceiptResponse',inputs[4]],
  ['/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings','InspectionReadingRequest','InspectionReadingResponse',inputs[4]],
  ['/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/reinspections','InspectionReinspectionRequest','InspectionReadingResponse',inputs[6]],
  ['/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings/{item_id}/reviews','InspectionReadingReviewRequest','InspectionReadingReviewResponse',inputs[6]],
];
const counts = {};

// This validator covers the keywords used by the six bounded schemas above.
// The project's OpenAPI gate is a separate, independently recorded check.
function validate(value, schema, location) {
  if (!schema) return [`${location}: missing schema`];
  if (schema.$ref) return validate(value, schemas[schema.$ref.split('/').at(-1)], location);
  if (schema.oneOf) {
    const matches = schema.oneOf.filter(item => validate(value,item,location).length === 0).length;
    return matches === 1 ? [] : [`${location}: oneOf matched ${matches}`];
  }
  if (schema.allOf) return schema.allOf.flatMap(item => validate(value,item,location));
  const issues = [];
  if (value === null) return schema.nullable ? [] : [`${location}: unexpected null`];
  const type = Array.isArray(value) ? 'array' : typeof value;
  if (schema.type === 'integer' ? !Number.isInteger(value) : schema.type && schema.type !== type) return [`${location}: type ${type}`];
  if (schema.enum && !schema.enum.some(item => JSON.stringify(item) === JSON.stringify(value))) issues.push(`${location}: enum`);
  if (type === 'object') {
    for (const key of schema.required ?? []) if (!Object.hasOwn(value,key)) issues.push(`${location}.${key}: missing`);
    for (const [key,item] of Object.entries(value)) {
      if (schema.properties?.[key]) issues.push(...validate(item,schema.properties[key],`${location}.${key}`));
      else if (schema.additionalProperties === false) issues.push(`${location}.${key}: unexpected`);
    }
  } else if (type === 'array') {
    if (schema.maxItems !== undefined && value.length > schema.maxItems) issues.push(`${location}: array budget`);
    for (let i=0;i<value.length;i++) issues.push(...validate(value[i],schema.items,`${location}[${i}]`));
  } else if (type === 'number') {
    if (!Number.isSafeInteger(value) && schema.type === 'integer') issues.push(`${location}: numeric sample exceeds JS exact integer range`);
    if (schema.minimum !== undefined && value < schema.minimum) issues.push(`${location}: minimum`);
    if (schema.maximum !== undefined && value > schema.maximum) issues.push(`${location}: maximum`);
  } else if (type === 'string') {
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) issues.push(`${location}: pattern`);
    if (schema.format === 'uuid' && !/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i.test(value)) issues.push(`${location}: uuid`);
    if (schema.format === 'date-time' && Number.isNaN(Date.parse(value))) issues.push(`${location}: date-time`);
  }
  return issues;
}

try {
  assert.deepEqual(paths,bundle.paths,'Current source and generated paths');
  assert.deepEqual(components,bundle.components,'Current source and generated components');
  for (const [url,request,response,sourceFile] of routes) {
    const operation = paths[url]?.post;
    assert.ok(operation, `Current registered consumer contract missing: ${url}`);
    assert.deepEqual(operation.security,[{bearerAuth:[]}]);
    assert.equal(operation.requestBody.content['application/json'].schema.$ref,`#/components/schemas/${request}`);
    assert.equal(operation.responses['200'].content['application/json'].schema.$ref,`#/components/schemas/${response}`);
    assert.ok(operation.responses['409'],`${url}: missing conflict contract`);
    const source = fs.readFileSync(path.join(root,sourceFile),'utf8');
    const body = source.match(new RegExp(`struct ${request}\\s*\\{([\\s\\S]*?)^\\}`,'m'))?.[1];
    assert.ok(body,`Current Rust DTO missing: ${request}`);
    const fields = [...body.matchAll(/^\s*(\w+)\s*:\s*([^,]+),/gm)];
    assert.deepEqual(Object.keys(schemas[request].properties).sort(),fields.map(item=>item[1]).sort());
    assert.deepEqual([...schemas[request].required].sort(),fields.filter(item=>!item[2].includes('Option<')).map(item=>item[1]).sort());
    counts[request] = {success:0,rejected:0};
  }
  const log = fs.readFileSync(path.join(root,inputs.at(-1)),'utf8').replace(/^\uFEFF/,'');
  assert.match(log,/test result: ok\. 332 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;/);
  const owned = new Set([...log.matchAll(/QA inspection_gate_fixture tenant=([a-f0-9-]+) /g)].map(item=>item[1]));
  assert.equal(owned.size,14);
  for (const match of log.matchAll(/QA (?:inspection_gate|production_release)_http tenant=([a-f0-9-]+) path=(\S+) input=(.*?) status=(\d+) response=(.+)/g)) {
    if (!owned.has(match[1])) continue;
    const actual = match[2];
    let index;
    if (/^sales-orders\/[^/]+\/deliveries$/.test(actual)) index=0;
    else if (/^work-orders\/[^/]+\/materials$/.test(actual)) index=1;
    else if (/^work-orders\/[^/]+\/receipts$/.test(actual)) index=2;
    else if (/^quality\/inspection-freezes\/[^/]+\/readings\/[^/]+\/reviews$/.test(actual)) index=5;
    else if (/^quality\/inspection-freezes\/[^/]+\/reinspections$/.test(actual)) index=4;
    else if (/^quality\/inspection-freezes\/[^/]+\/readings$/.test(actual)) index=3;
    else continue;
    const [url,request,response] = routes[index];
    errors.push(...validate(JSON.parse(match[3]),schemas[request],`${actual} input`));
    if (match[4] === '200') {
      errors.push(...validate(JSON.parse(match[5]),schemas[response],`${actual} response`));
      counts[request].success += 1;
      checked += 1;
    } else {
      assert.ok(paths[url].post.responses[match[4]],`${actual}: actual rejection ${match[4]} not documented`);
      counts[request].rejected += 1;
      rejected += 1;
    }
  }
  assert.ok(checked>=30 && rejected>=25,'Actual successful consumers, retakes/reviews and twenty race rejects required');
  assert.ok(Object.values(counts).every(item=>item.success>0),'Every stated contract needs an actual success body');
} catch (error) {
  errors.push(error instanceof Error ? error.message : String(error));
}
const after = Object.fromEntries(inputs.map(name=>[name,sha(name)]));
for (const name of inputs) if (before[name] !== after[name]) errors.push(`Source changed during validation: ${name}`);
const result = {created_at:new Date().toISOString(),status:errors.length?'FAIL':'PASS',
  scope:'Six current consumer/measurement POST contracts, their current Rust DTOs and actual bodies from fourteen owned QA fixture groups. Planning/lookup/cancel response variants are schema-defined but not exercised by these gates. Not normal authentication, roles, formal clients or release performance.',
  routes,successful_bodies:checked,rejected_responses:rejected,per_contract:counts,inputs_before:before,inputs_after:after,errors};
fs.writeFileSync(output,JSON.stringify(result,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({status:result.status,checked,rejected,errors}));
if (errors.length) process.exitCode=1;
