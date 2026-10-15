"""Independent read of only the fresh HTTP fixtures; no existing tenant mutation."""
import json,re,subprocess,hashlib
from pathlib import Path
base=Path(__file__).resolve().parent
log=(base/'free-erp-trade-lines-oct8-fixed-test.log').read_text(encoding='utf-8-sig')
result=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log)
assert result and result.groups()==('4','0','0'),'four registered cases must finish first'
fixtures=dict(re.findall(r'QA trade_facts_fixture tenant=([a-f0-9-]+).*?case=(sales-multiline-[a-z]+) ',log))
assert len(fixtures)==4
ids=','.join("'"+tenant+"'::uuid" for tenant in sorted(fixtures))
tables=['shared_sales_revisions','shared_sales_lines','command_receipts','shared_sales_demands']
parts=["'"+table+"',coalesce((select jsonb_agg(to_jsonb(x)) from "+table+" x where x.tenant_id=t.tenant_id),'[]'::jsonb)" for table in tables]
sql=r"""\set ON_ERROR_STOP on
begin read only;
do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test' or inet_server_port()<>15438 or inet_server_addr()<>inet '127.0.0.1' then raise exception 'QA identity mismatch'; end if; end $$;
"""+"select jsonb_build_object('tenant',t.tenant_id,'facts',jsonb_build_object("+','.join(parts)+")) from tenants t where t.tenant_id in ("+ids+") order by t.tenant_id;\ncommit;\n"
p=base/'free-erp-trade-lines-oct8-fixed-facts.sql';assert not p.exists();p.write_text(sql,encoding='utf8')
out=base/'free-erp-trade-lines-oct8-fixed-facts.jsonl';err=base/'free-erp-trade-lines-oct8-fixed-facts.stderr.log'
with out.open('xb') as f,err.open('xb') as e:
    r=subprocess.run(['G:/PostgreSQL/17/bin/psql.exe','-X','-q','-t','-A','-h','127.0.0.1','-p','15438','-U','erp_grid_test','-d','erp_shared_contract_test','-v','ON_ERROR_STOP=1','-f',str(p)],stdout=f,stderr=e,check=False)
assert r.returncode==0,'independent read failed'
checks=0
for raw in out.read_text(encoding='utf8').splitlines():
    if not raw.strip():continue
    row=json.loads(raw);facts=row['facts'];case=fixtures[row['tenant']]
    if case=='sales-multiline-invalid':
        assert all(not facts[table] for table in tables);checks+=4;continue
    assert len(facts['shared_sales_revisions'])==len(facts['command_receipts'])==1;checks+=2
    revision=facts['shared_sales_revisions'][0];receipt=facts['command_receipts'][0]
    assert receipt['effect_id']==revision['revision_id'];checks+=1
    assert re.fullmatch('[a-f0-9]{64}',receipt['payload_hash']);checks+=1
    assert revision['version_no']==1 and revision['customer_code']=='QA-CUSTOMER';checks+=2
    lines=sorted(facts['shared_sales_lines'],key=lambda row:row['line_no'])
    assert [(row['line_no'],row['qty_micros'],row['price_cent'],row['tax_cent']) for row in lines]==[(3,1250000,100,5),(9,2000001,200,7)];checks+=8
    assert all(row['revision_id']==revision['revision_id'] for row in lines);checks+=2
    assert not facts['shared_sales_demands'];checks+=1
before=json.loads((base/'free-erp-inventory-consumers-trade-lines-oct8-fixed-before-source.json').read_text())['files']
after=json.loads((base/'free-erp-inventory-consumers-trade-lines-oct8-fixed-after-source.json').read_text())['files']
assert before==after,'source inputs changed';checks+=1
report={'status':'PASS','checks':checks,'fixtures':len(fixtures),'passed':4,'failed':0,'ignored':0,'source_inputs':len(before),'source_drift':[],
        'facts_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'scope':'four fresh password-authenticated sales revision fixtures; not all prior enterprises, formal UI, complete trade documents, Release or free hosting'}
dest=base/'free-erp-trade-lines-oct8-independent.json';assert not dest.exists();dest.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8')
print(json.dumps(report))
