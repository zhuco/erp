"""New owned material browser company. Parents are real; stock/cost use API commands."""
import json,runpy,uuid,urllib.request
from pathlib import Path
from argon2 import PasswordHasher
BASE=Path(__file__).resolve().parent
h=runpy.run_path(str(BASE/'free-erp-product-web-fixture.py'))
q=h['query'];lit=h['literal'];manifest=BASE/'free-erp-material-batch-web-oct10-fixture.json'
PASSWORD='ERP-owned-QA-material-browser-2026!'
if manifest.exists():
 f=json.loads(manifest.read_text());assert q('SELECT name FROM tenants WHERE tenant_id='+lit(f['tenant_id'])+';')=='QA-MATERIAL-BATCH-WEB-OCT10'
else:
 f={key:str(uuid.uuid4()) for key in ['tenant_id','user_id','role_id','product_id','material_product_id','sku_id','warehouse_id','source_bom_id','bom_version_id','work_order_id','stock_request_id','cost_request_id','batch_id']}
 f.update(email=f['user_id']+'@material-web.qa.invalid',api='http://127.0.0.1:15441',work_order_no='QA-WO-BATCH-001',item_code='QA-BATCH-MATERIAL-001')
 v={k:lit(x) for k,x in f.items()};ph=lit(PasswordHasher(time_cost=2,memory_cost=19456,parallelism=1).hash(PASSWORD))
 perms=['skus.read','skus.write','BASIC.SKU.READ','BASIC.SKU.WRITE','manufacturing.read','MFG.PRODUCTION.READ','manufacturing.write','MFG.PRODUCTION.WRITE','inventory.read','inventory.write']
 values=','.join('('+v['tenant_id']+','+v['role_id']+','+lit(code)+')' for code in perms)
 q(f"""
 INSERT INTO tenants(tenant_id,name) VALUES({v['tenant_id']},'QA-MATERIAL-BATCH-WEB-OCT10');
 INSERT INTO users(tenant_id,user_id,email,password_hash,status) VALUES({v['tenant_id']},{v['user_id']},{v['email']},{ph},'ACTIVE');
 INSERT INTO roles(tenant_id,role_id,name) VALUES({v['tenant_id']},{v['role_id']},'QA material operator');
 INSERT INTO user_roles(tenant_id,user_id,role_id) VALUES({v['tenant_id']},{v['user_id']},{v['role_id']});
 INSERT INTO role_permissions(tenant_id,role_id,perm_code) VALUES {values};
 INSERT INTO products(tenant_id,product_id,product_code,name,status) VALUES({v['tenant_id']},{v['product_id']},'QA-PRODUCT-001','QA finished product','ACTIVE'),({v['tenant_id']},{v['material_product_id']},'QA-MATERIAL-PARENT','QA raw material parent','ACTIVE');
 INSERT INTO skus(tenant_id,product_id,sku_id,sku_code,name,is_active) VALUES({v['tenant_id']},{v['material_product_id']},{v['sku_id']},{v['item_code']},'QA raw material',true);
 INSERT INTO warehouses(tenant_id,warehouse_id,code,name) VALUES({v['tenant_id']},{v['warehouse_id']},'QA-RAW-001','QA raw warehouse');
 INSERT INTO user_warehouse_access(tenant_id,user_id,warehouse_id) VALUES({v['tenant_id']},{v['user_id']},{v['warehouse_id']});
 INSERT INTO mfg_boms(tenant_id,bom_id,product_code,status,version,base_qty_micros) VALUES({v['tenant_id']},{v['source_bom_id']},'QA-PRODUCT-001','PUBLISHED',1,1000000);
 INSERT INTO shared_mfg_bom_versions(tenant_id,version_id,series_code,version_no,status,start_date,source_bom_id) VALUES({v['tenant_id']},{v['bom_version_id']},'QA-PRODUCT-001',1,'PUBLISHED','2026-10-01',{v['source_bom_id']});
 INSERT INTO shared_mfg_bom_version_lines(tenant_id,version_id,line_no,parent_code,component_code,qty_per_micros,unit_code,scrap_ppm,supply_kind) VALUES({v['tenant_id']},{v['bom_version_id']},1,'QA-PRODUCT-001',{v['item_code']},1000000,'kg',0,'PURCHASE');
 INSERT INTO mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,bom_id,status,order_qty_micros) VALUES({v['tenant_id']},{v['work_order_id']},{v['work_order_no']},'QA-PRODUCT-001',{v['bom_version_id']},'RELEASED',1000000);
 """)
 manifest.write_text(json.dumps(f,indent=2)+'\n',encoding='utf-8')
q("INSERT INTO role_permissions(tenant_id,role_id,perm_code) SELECT "+lit(f['tenant_id'])+","+lit(f['role_id'])+",code FROM unnest(ARRAY['WMS.INVENTORY.READ','WMS.INVENTORY.WRITE']) code ON CONFLICT DO NOTHING;")
q("UPDATE skus SET batch_enabled=true WHERE tenant_id="+lit(f['tenant_id'])+" AND sku_id="+lit(f['sku_id'])+"; INSERT INTO sku_batches(tenant_id,sku_id,batch_id,batch_no,expire_at) VALUES("+lit(f['tenant_id'])+","+lit(f['sku_id'])+","+lit(f['batch_id'])+",'QA-RAW-LOT-001',clock_timestamp()+interval '30 days') ON CONFLICT DO NOTHING;")
opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
def http(path,body,token=None):
 headers={'content-type':'application/json'}
 if token:headers['authorization']='Bearer '+token
 req=urllib.request.Request(f['api']+path,data=json.dumps(body).encode(),headers=headers,method='POST')
 with opener.open(req,timeout=15) as response:return json.load(response)
login=http('/auth/login',{'tenant_id':f['tenant_id'],'email':f['email'],'password':PASSWORD})
token=login['access_token']
stock=http('/v1/tenants/'+f['tenant_id']+'/inventory/adjustments',{'request_id':f['stock_request_id'],'warehouse_id':f['warehouse_id'],'sku_id':f['sku_id'],'quantity_delta':1,'batch_id':f['batch_id'],'reason':'QA material opening','reference_id':'QA-MATERIAL-WEB-OPENING'},token)
period=q("SELECT to_char(current_date,'YYYY-MM');")
cost=http('/v1/tenants/'+f['tenant_id']+'/cost-receipts',{'request_id':f['cost_request_id'],'item_code':f['item_code'],'qty_micros':1000000,'amount_cent':1200,'period_code':period},token)
proof={'tenant_id':f['tenant_id'],'stock':stock,'cost':cost,'marker':'owned real parent fixtures; actual stock and cost command responses'}
out=BASE/'free-erp-material-batch-web-oct10-opening.json'
if not out.exists():out.write_text(json.dumps(proof,indent=2)+'\n',encoding='utf-8')
print('Own material browser fixture ready:',f['work_order_no'],f['tenant_id'])
