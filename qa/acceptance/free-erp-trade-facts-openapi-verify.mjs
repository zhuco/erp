// Check the receipt snapshot schema against its Rust public fields.
import fs from 'node:fs';
import crypto from 'node:crypto';
import path from 'node:path';
import { loadOpenApiDocument } from '../../scripts/openapi/lib/openapi_tools.mjs';

const root = process.cwd();
const out = path.join(root,'qa/acceptance/free-erp-trade-facts-openapi.json');
if (fs.existsSync(out)) throw new Error('Refusing to replace contract evidence');
const componentPath = path.join(root,'openapi/src/components.yaml');
const pathPath = path.join(root,'openapi/src/paths.yaml');
const bundlePath = path.join(root,'openapi/erp_api_v1.yaml');
const rustPath = path.join(root,'crates/erp-repo/src/shared_contract/receipt_offsets.rs');
const component = loadOpenApiDocument(componentPath).schemas.PurchaseReceiptOffsetResponse;
const source = loadOpenApiDocument(pathPath)['/v1/tenants/{tenant_id}/purchase-receipt-offsets'].get;
const bundle = loadOpenApiDocument(bundlePath);
const compiled = bundle.components.schemas.PurchaseReceiptOffsetResponse;
const route = bundle.paths['/v1/tenants/{tenant_id}/purchase-receipt-offsets'].get;
const rust = fs.readFileSync(rustPath,'utf8');
const body = /pub struct PurchaseReceiptOffset\s*\{([^}]+)\}/s.exec(rust)?.[1] ?? '';
const fields = [...body.matchAll(/pub (\w+):\s*(\w+)/g)].map(x=>[x[1],x[2]]);
const checks = {
  fields_match: JSON.stringify(fields.map(x=>x[0]).sort())===JSON.stringify(Object.keys(component.properties).sort()),
  required_fields_match: JSON.stringify(fields.map(x=>x[0]).sort())===JSON.stringify([...component.required].sort()),
  integer_fields_are_int64: fields.filter(x=>x[1]==='i64').every(([name])=>component.properties[name].type==='integer'&&component.properties[name].format==='int64'),
  actual_statuses: JSON.stringify(component.properties.status.enum)===JSON.stringify(['OFFSET','NO_OFFSET']),
  source_and_bundle_schema: JSON.stringify(component)===JSON.stringify(compiled),
  source_and_bundle_route: JSON.stringify(source)===JSON.stringify(route),
  public_failure_codes: ['400','401','403','404','409','500'].every(code=>source.responses[code]),
  only_source_identity_query: source.parameters.length===2 && source.parameters.map(x=>x.name).sort().join(',')==='quality_id,tenant_id',
  no_client_quantity_or_cost: !source.requestBody,
};
const sha = file=>crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const report = {utc:new Date().toISOString(),status:Object.values(checks).every(Boolean)?'PASS':'FAIL',
  scope:'DTO fields, source/bundle and public failure contract only; no runtime or Release performance inference',
  checks,sha256:Object.fromEntries([componentPath,pathPath,bundlePath,rustPath].map(file=>[path.relative(root,file).replaceAll('\\','/'),sha(file)]))};
fs.writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({status:report.status,checks}));
if(report.status!=='PASS')process.exitCode=1;
