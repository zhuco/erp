"""Same QA business user, different real network login SID, original GET only."""
import hashlib,json,runpy,urllib.request
from pathlib import Path
B=Path(__file__).resolve().parent
acceptance=json.loads((B/'free-erp-operation-report-oct10-recovered-full-independent-recheck.json').read_text());assert acceptance['status']=='PASS'
h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));q=h['query'];l=h['literal'];scope=','.join(l(t) for t in acceptance['new_tenants'])
row=json.loads(q("SELECT json_build_object('tenant_id',o.tenant_id,'work_order_id',o.work_order_id,'request_id',o.request_id,'email',u.email,'original',o.snapshot) FROM mfg_operation_report_outcomes o JOIN users u ON u.tenant_id=o.tenant_id AND u.user_id=o.actor_user_id JOIN tenants t ON t.tenant_id=o.tenant_id JOIN mfg_reports r ON r.tenant_id=o.tenant_id AND r.report_id=o.report_id WHERE o.tenant_id IN ("+scope+") AND u.status='ACTIVE' AND t.credentials_epoch=0 AND (to_jsonb(r)-'created_at')=((o.snapshot->'report')-'created_at') AND EXISTS(select 1 from user_roles ur join role_permissions rp using(tenant_id,role_id) where ur.tenant_id=o.tenant_id and ur.user_id=o.actor_user_id and rp.perm_code in('manufacturing.read','MFG.PRODUCTION.READ')) ORDER BY o.tenant_id,o.request_id LIMIT 1;"))
url='http://127.0.0.1:15441';opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
def request(path,body=None,token=None):
 headers={'content-type':'application/json','User-Agent':'ERP-owned-original-report-network-QA'}
 if token:headers['authorization']='Bearer '+token
 req=urllib.request.Request(url+path,data=None if body is None else json.dumps(body).encode(),headers=headers,method='GET' if body is None else 'POST')
 with opener.open(req,timeout=20) as r:return json.load(r)
def facts():
 tables=['mfg_reports','command_receipts','mfg_operation_report_outcomes','inventory_balances','shared_item_costs','mfg_receipts']
 return {table:q("select coalesce(json_agg(md5(to_jsonb(x)::text) order by md5(to_jsonb(x)::text)),'[]'::json) from "+table+' x where tenant_id='+l(row['tenant_id'])+';') for table in tables}
before=facts()
login=request('/auth/login',{'tenant_id':row['tenant_id'],'email':row['email'],'password':'ERP-owned-QA-real-password-login-2026!','client_kind':'WEB','client_label':'QA-owned-original-report-network'})
original_sid=row['original']['authority']['session_id'];assert login['session_id']!=original_sid
actual=request('/v1/tenants/'+row['tenant_id']+'/manufacturing/work-orders/'+row['work_order_id']+'/reports/commands/'+row['request_id'],token=login['access_token'])
assert actual==row['original'];assert facts()==before
proof={'status':'PASS','scope':'Loopback API actual password login with a different SID and exact original report GET; no report POST, no client UI claim','tenant_id':row['tenant_id'],'request_id':row['request_id'],'original_sid':original_sid,'current_sid':login['session_id'],'same_actor':actual['actor_user_id'],'original_response':actual,'business_hashes':before,'checks':['actual current session differs from original session','same business actor read exact immutable original','six business tables unchanged','no report POST replay'],'api':'127.0.0.1:15441'}
p=B/'free-erp-operation-report-oct10-network.json';assert not p.exists();p.write_text(json.dumps(proof,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print('PASS actual loopback original GET from different current SID')
