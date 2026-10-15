"""Independent current records of the seven fresh formal trade HTTP scenarios."""
import json,re,subprocess,hashlib
from pathlib import Path
base=Path(__file__).resolve().parent
log=(base/'free-erp-trade-docs-oct8-expanded-test.log').read_text(encoding='utf-8-sig')
result=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log)
assert result and result.groups()==('7','0','0')
fixtures=dict(re.findall(r'QA trade_facts_fixture tenant=([a-f0-9-]+).*?case=(document-[a-z-]+) ',log));assert len(fixtures)==8
tables=['trade_sales_documents','trade_sales_document_revisions','trade_sales_line_bindings','shared_sales_revisions','shared_sales_lines','shared_partners','skus','command_receipts','operation_logs','audit_logs','outbox_events','shared_sales_deliveries']
ids=','.join("'"+id+"'::uuid" for id in sorted(fixtures))
parts=["'"+table+"',coalesce((select jsonb_agg(to_jsonb(x)) from "+table+" x where x.tenant_id=t.tenant_id),'[]'::jsonb)" for table in tables]
sql=r"""\set ON_ERROR_STOP on
begin read only;
do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test' or inet_server_port()<>15438 or inet_server_addr()<>inet '127.0.0.1' then raise exception 'QA identity mismatch'; end if; end $$;
"""+"select jsonb_build_object('tenant',t.tenant_id,'facts',jsonb_build_object("+','.join(parts)+")) from tenants t where t.tenant_id in ("+ids+") order by t.tenant_id;\ncommit;\n"
p=base/'free-erp-trade-docs-oct8-expanded-facts.sql';assert not p.exists();p.write_text(sql,encoding='utf8')
out=base/'free-erp-trade-docs-oct8-expanded-facts.jsonl';err=base/'free-erp-trade-docs-oct8-expanded-facts.stderr.log'
with out.open('xb') as f,err.open('xb') as e:
    r=subprocess.run(['G:/PostgreSQL/17/bin/psql.exe','-X','-q','-t','-A','-h','127.0.0.1','-p','15438','-U','erp_grid_test','-d','erp_shared_contract_test','-v','ON_ERROR_STOP=1','-f',str(p)],stdout=f,stderr=e,check=False)
assert r.returncode==0
checks=0;rows={}
for raw in out.read_text(encoding='utf8').splitlines():
    if not raw.strip():continue
    row=json.loads(raw);facts=row['facts'];tenant=row['tenant'];rows[tenant]=facts
    if fixtures[tenant]=='document-bypass-other':
        assert not facts['trade_sales_documents'] and not facts['shared_sales_revisions'];checks+=2;continue
    assert len(facts['trade_sales_documents'])==1;checks+=1
    expected=2 if fixtures[tenant]=='document-same-fact' else 1
    assert len(facts['trade_sales_document_revisions'])==len(facts['shared_sales_revisions'])==expected;checks+=2
    for header in facts['trade_sales_document_revisions']:
        revision=[r for r in facts['shared_sales_revisions'] if r['revision_id']==header['revision_id']];assert len(revision)==1 and revision[0]['so_id']==header['so_id'];checks+=2
        commands=[r for r in facts['command_receipts'] if r['request_id']==header['request_id']];assert len(commands)==1 and commands[0]['status']=='SUCCEEDED' and commands[0]['effect_id']==header['revision_id'];checks+=3
        assert re.fullmatch('[a-f0-9]{64}',commands[0]['payload_hash']);checks+=1
        lines=[r for r in facts['shared_sales_lines'] if r['revision_id']==header['revision_id']];bindings=[r for r in facts['trade_sales_line_bindings'] if r['revision_id']==header['revision_id']];assert len(lines)==len(bindings)==2;checks+=2
        total=sum(r['qty_micros']*r['price_cent']//1000000-r['discount_cent']+r['tax_cent'] for r in lines);assert total==header['snapshot']['total_cent'];checks+=1
        for binding in bindings:
            line=[r for r in lines if r['line_no']==binding['line_no']];sku=[r for r in facts['skus'] if r['sku_id']==binding['sku_id']];assert len(line)==len(sku)==1 and line[0]['item_code']==sku[0]['sku_code'];checks+=3
        actor=header['snapshot']['actor']['user_id'];assert len(header['snapshot']['authority'])==2;checks+=1
        ops=[r for r in facts['operation_logs'] if r['action']=='TRADE_SALES_SAVE' and r.get('ref_id')==header['revision_id']]
        audits=[r for r in facts['audit_logs'] if r['action']=='trade.sales.save' and r.get('entity_id')==header['revision_id']]
        events=[r for r in facts['outbox_events'] if r['event_type']=='trade.sales.saved' and r['payload'].get('revision_id')==header['revision_id']]
        assert len(ops)==len(audits)==len(events)==1 and ops[0]['actor_user_id']==actor and audits[0]['user_id']==actor;checks+=5
for match in re.finditer(r'QA real_login_http tenant=([a-f0-9-]+).*?status=200 response=(.*)',log):
    tenant,raw=match.groups()
    if tenant not in rows:continue
    response=json.loads(raw);document=response.get('document',response)
    if 'header' not in document:continue
    header=document['header'];facts=rows[tenant];actual=[h for h in facts['trade_sales_document_revisions'] if h['revision_id']==header['revision_id']]
    assert len(actual)==1 and all(header.get(key)==value for key,value in actual[0]['snapshot'].items());checks+=2
    assert sorted((int(line['qty_micros']),line['line_no']) for line in document['items'])==[(1250000,3),(2000001,9)];checks+=2
before=json.loads((base/'free-erp-inventory-consumers-trade-docs-oct8-expanded-before-source.json').read_text())['files'];after=json.loads((base/'free-erp-inventory-consumers-trade-docs-oct8-expanded-after-source.json').read_text())['files'];assert before==after;checks+=1
report={'status':'PASS','checks':checks,'fixtures':len(fixtures),'passed':7,'failed':0,'ignored':0,'source_inputs':len(before),'source_drift':[],'facts_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'scope':'eight fresh QA enterprises with seven registered HTTP cases; not full prior-enterprise scan, UI, all trade delivery/finance contracts, Release or free hosting'}
dest=base/'free-erp-trade-docs-oct8-independent.json';assert not dest.exists();dest.write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report))
