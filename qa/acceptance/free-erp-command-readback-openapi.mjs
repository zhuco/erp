// Validate actual command HTTP requests and successful bodies against the saved contract.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { parse } from 'yaml';

const base = path.resolve('qa/acceptance');
const stage = process.argv[2];
if (!/^[a-z0-9-]+$/.test(stage ?? '')) throw new Error('immutable stage required');
const destination = path.join(base, `free-erp-command-readback-${stage}-openapi.json`);
if (fs.existsSync(destination)) throw new Error('refusing to replace historical contract evidence');
const contractBytes = fs.readFileSync('openapi/erp_api_v1.yaml');
const contract = parse(contractBytes.toString('utf8'), {uniqueKeys: true});
const schemas = contract.components.schemas;
const errors = [];
let requests = 0, successes = 0, refusals = 0;
function valid(schema, value) {
  if (schema.$ref) return valid(schemas[schema.$ref.split('/').at(-1)], value);
  if (schema.oneOf) return schema.oneOf.filter(item => valid(item, value)).length === 1;
  if (schema.anyOf) return schema.anyOf.some(item => valid(item, value));
  if (schema.nullable && value === null) return true;
  if (schema.enum && !schema.enum.some(item => Object.is(item, value))) return false;
  if (schema.type === 'object') {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
    if ((schema.required ?? []).some(key => !(key in value))) return false;
    if (schema.additionalProperties === false && Object.keys(value).some(key => !(key in (schema.properties ?? {})))) return false;
    return Object.entries(schema.properties ?? {}).every(([key, item]) => !(key in value) || valid(item, value[key]));
  }
  if (schema.type === 'array') return Array.isArray(value) && value.every(item => valid(schema.items, item));
  if (schema.type === 'string') return typeof value === 'string' && value.length >= (schema.minLength ?? 0);
  if (schema.type === 'boolean') return typeof value === 'boolean';
  if (schema.type === 'integer') return Number.isInteger(value) && value >= (schema.minimum ?? -Infinity);
  if (schema.type === 'number') return typeof value === 'number';
  return true;
}
const logPath = path.join(base, `free-erp-command-readback-${stage}-test.log`);
const logBytes = fs.readFileSync(logPath);
for (const line of logBytes.toString('utf8').split(/\r?\n/)) {
  const match = line.match(/QA command_readback_http tenant=([a-f0-9-]+) method=(GET|POST) path=([^ ]+) input=(.*?) status=(\d+) response=(.*)$/);
  if (!match) continue;
  const [,tenant,method,route,inputText,statusText,responseText] = match;
  const url = new URL(`https://qa.invalid/${route}`);
  const actualPath = `/v1/tenants/{tenant_id}/${url.pathname.slice(1).replace(/commands\/[a-f0-9-]+/, 'commands/{request_id}')}`;
  const operation = contract.paths[actualPath]?.[method.toLowerCase()];
  if (!operation) { errors.push(`unregistered operation: ${method} ${actualPath}`); continue; }
  const input = JSON.parse(inputText), response = JSON.parse(responseText), status = Number(statusText);
  const requestSchema = operation.requestBody?.content?.['application/json']?.schema;
  if (requestSchema) { requests++; if (!valid(requestSchema,input)) errors.push(`request mismatch: ${tenant} ${route}`); }
  if (!operation.responses[String(status)]) errors.push(`undeclared actual status ${status}: ${route}`);
  if (status === 200) {
    successes++;
    const schema = operation.responses['200']?.content?.['application/json']?.schema;
    if (!schema || !valid(schema,response)) errors.push(`success body mismatch: ${tenant} ${route}`);
  } else refusals++;
}
if (successes < 20 || refusals < 10) errors.push('expected the complete actual positive/refusal samples');
const report = {utc:new Date().toISOString(),status:errors.length ? 'FAIL' : 'PASS',requests,successes,refusals,errors,
  scope:'Actual command requests and successful response shapes; status declarations for refusals. No performance, ordinary login or full business-source authorization certification.',
  contract_sha256:crypto.createHash('sha256').update(contractBytes).digest('hex'),
  log_sha256:crypto.createHash('sha256').update(logBytes).digest('hex')};
fs.writeFileSync(destination,JSON.stringify(report,null,2)+'\n');
process.stdout.write(JSON.stringify({status:report.status,requests,successes,refusals,errors})+'\n');
process.exitCode = errors.length ? 1 : 0;
