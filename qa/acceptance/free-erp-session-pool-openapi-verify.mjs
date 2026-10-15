// Read source and bundled public login capacity contract; no business calls.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { loadOpenApiDocument } from '../../scripts/openapi/lib/openapi_tools.mjs';

const root = process.cwd();
const destination = path.resolve(root, 'qa/acceptance/free-erp-session-pool-oct7-openapi.json');
if (fs.existsSync(destination)) throw new Error('Refusing to replace contract evidence');
const sourcePath = path.join(root, 'openapi/src/paths.yaml');
const bundlePath = path.join(root, 'openapi/erp_api_v1.yaml');
const handlerPath = path.join(root, 'crates/erp-api/src/auth_handlers.rs');
const source = loadOpenApiDocument(sourcePath)['/auth/login'].post.responses['503'];
const bundle = loadOpenApiDocument(bundlePath).paths['/auth/login'].post.responses['503'];
const schema = source?.content?.['application/json']?.schema;
const handler = fs.readFileSync(handlerPath, 'utf8');
const checks = {
  source_and_bundle_identical: JSON.stringify(source) === JSON.stringify(bundle),
  response_object: schema?.type === 'object',
  required_error_fields: schema?.required?.length === 2 && schema.required.includes('code') && schema.required.includes('message'),
  public_busy_code: schema?.properties?.code?.type === 'string' && JSON.stringify(schema.properties.code.enum) === JSON.stringify(['login_busy']),
  message_is_string: schema?.properties?.message?.type === 'string',
  no_session_on_overload: /no session is opened/.test(source?.description ?? ''),
  actual_handler_mapping: /Err\(PasswordVerificationError::Busy\)[\s\S]{0,450}StatusCode::SERVICE_UNAVAILABLE[\s\S]{0,250}"code"\s*:\s*"login_busy"/.test(handler),
};
const sha = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const report = {
  utc: new Date().toISOString(),
  status: Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL',
  scope: 'Public login overload schema and source mapping only; no runtime request or performance qualification',
  checks,
  sha256: Object.fromEntries([sourcePath, bundlePath, handlerPath].map(file => [path.relative(root, file).replaceAll('\\', '/'), sha(file)])),
};
fs.writeFileSync(destination, `${JSON.stringify(report, null, 2)}\n`, {flag: 'wx'});
console.log(JSON.stringify({status: report.status, checks}));
if (report.status !== 'PASS') process.exitCode = 1;
