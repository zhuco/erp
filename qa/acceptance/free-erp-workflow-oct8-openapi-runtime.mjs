// Validate observed purchase handler responses against the source and bundled contract.
import fs from 'node:fs';
import crypto from 'node:crypto';
import { loadOpenApiDocument } from '../../scripts/openapi/lib/openapi_tools.mjs';

const bundlePath='openapi/erp_api_v1.yaml';
const sourcePath='openapi/src/paths.yaml';
const componentPath='openapi/src/components.yaml';
const logPath='qa/acceptance/free-erp-workflow-oct8-arrival-proof-fixed-test.log';
const stage=process.argv[2]??'';
if(stage&&!/^[a-z0-9-]+$/.test(stage))throw new Error('Invalid evidence stage');
const out=`qa/acceptance/free-erp-workflow-oct8-openapi-runtime${stage?'-'+stage:''}.json`;
if(fs.existsSync(out))throw new Error('Refusing to replace runtime contract evidence');
const bundle=loadOpenApiDocument(bundlePath);
const source=loadOpenApiDocument(sourcePath);
const components=loadOpenApiDocument(componentPath);
const route='/v1/tenants/{tenant_id}/purchase-orders';
const errors=[];
const checks={source_route_matches_bundle:JSON.stringify(source[route].post)===JSON.stringify(bundle.paths[route].post)};
const selected=['PurchaseOrderRevisionSavedResponse','PurchaseOrderArrivalResponse','PurchaseArrivalReconciliationResponse'];
checks.source_shapes_match_bundle=selected.every(name=>JSON.stringify(components.schemas[name])===JSON.stringify(bundle.components.schemas[name]));

function conforms(value, schema, depth=0){
  if(!schema||depth>40)return false;
  if(schema.$ref){
    const name=schema.$ref.split('/').at(-1);
    return conforms(value,bundle.components.schemas[name],depth+1);
  }
  if(value===null)return schema.nullable===true;
  if(schema.oneOf&&schema.oneOf.filter(s=>conforms(value,s,depth+1)).length!==1)return false;
  if(schema.anyOf&&!schema.anyOf.some(s=>conforms(value,s,depth+1)))return false;
  if(schema.enum&&!schema.enum.includes(value))return false;
  if(schema.type==='object'){
    if(!value||Array.isArray(value)||typeof value!=='object')return false;
    if((schema.required??[]).some(key=>!(key in value)))return false;
    if(schema.additionalProperties===false&&Object.keys(value).some(key=>!(key in (schema.properties??{}))))return false;
    if(Object.entries(value).some(([key,v])=>schema.properties?.[key]&&!conforms(v,schema.properties[key],depth+1)))return false;
  }
  if(schema.type==='array'&&(!Array.isArray(value)||value.some(v=>!conforms(v,schema.items,depth+1))))return false;
  if(schema.type==='integer'&&!Number.isInteger(value))return false;
  if(schema.type==='number'&&(typeof value!=='number'||!Number.isFinite(value)))return false;
  if(schema.type==='string'&&typeof value!=='string')return false;
  if(schema.type==='boolean'&&typeof value!=='boolean')return false;
  if(schema.format==='uuid'&&!/^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i.test(value))return false;
  if(schema.minimum!==undefined&&value<schema.minimum)return false;
  return true;
}
let responses=0,reconciliation=0;
const statuses={};
for(const line of fs.readFileSync(logPath,'utf8').split(/\r?\n/)){
  const m=/QA real_login_http .* method=POST path=\/v1\/tenants\/[^/ ]+\/purchase-orders status=(\d+) response=(.+)$/.exec(line);
  if(!m)continue;
  const value=JSON.parse(m[2]);
  const text=Object.keys(value).length===1&&typeof value.text==='string';
  const media=text?'text/plain':'application/json';
  const schema=bundle.paths[route].post.responses[m[1]]?.content?.[media]?.schema;
  if(!conforms(text?value.text:value,schema))errors.push(`response ${responses+1}: status ${m[1]} ${media} does not match`);
  if(value.code==='purchase_arrival_reconcile_required'){
    reconciliation++;
    if(value.reconciliation_required!==true)errors.push('missing original result lost its explicit reconciliation state');
  }
  statuses[m[1]]=(statuses[m[1]]??0)+1;
  responses++;
}
// Ten scenarios issue 26 HTTP requests; the two old-protocol sources are prepared by SQL.
checks.real_responses_observed=responses===26;
checks.explicit_reconciliation_observed=reconciliation===1;
checks.observed_statuses=['200','409','422'].every(status=>statuses[status]>0);
const sha=path=>crypto.createHash('sha256').update(fs.readFileSync(path)).digest('hex');
const report={utc:new Date().toISOString(),status:errors.length===0&&Object.values(checks).every(Boolean)?'PASS':'FAIL',
  scope:'Observed password-authenticated Axum purchase responses and source/bundle shapes; not public deployment, native/Web UI or Release performance',
  checks,responses,statuses,reconciliation,errors,
  sha256:Object.fromEntries([bundlePath,sourcePath,componentPath,logPath].map(path=>[path,sha(path)]))};
fs.writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({status:report.status,responses,statuses,reconciliation,errors}));
if(report.status!=='PASS')process.exitCode=1;
