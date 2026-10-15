"""Build a guarded QA compatibility script from actual migration fragments.

Does not apply DDL, backfill orders or change old records. The protected QA
database lacks historical OMS schema; this is not a full migration acceptance.
"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
destination = BASE / 'free-erp-fulfillment-schema-qa.sql'
manifest = BASE / 'free-erp-fulfillment-schema-inputs.json'
if destination.exists() or manifest.exists():
    raise SystemExit('refusing to replace prepared schema input')
sources = {}


def source(name):
    raw = (ROOT / name).read_bytes()
    sources[name] = hashlib.sha256(raw).hexdigest()
    return raw.decode('utf-8-sig')


initial = source('migrations/0001_init.sql')
index = initial[initial.index('create table if not exists order_index ('):
                initial.index('-- Orders table (monthly RANGE')]
shipping = source('migrations/0007_shipping_schema.sql')
oms = source('migrations/0022_oms_fulfillment.sql')
rules = oms[oms.index('create table if not exists warehouse_rules ('):
            oms.index('alter table orders')]
ship_items = oms[oms.index('alter table shipments'):]
resend = source('migrations/0039_oms_resend_cancel.sql')
costs = source('migrations/0045_cost_ledgers.sql')
cogs = costs[costs.index('create table if not exists order_cogs ('):costs.index('alter table ap_docs')]
text = r'''\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected fulfillment QA identity required'; end if;
 if to_regclass('public.order_index') is not null or to_regclass('public.shipments') is not null
  or to_regclass('public.shipment_items') is not null or to_regclass('public.warehouse_rules') is not null
  or to_regclass('public.order_cogs') is not null then raise exception 'initial QA schema no longer matches'; end if;
end $$;
-- Nullable compatibility columns preserve every historical value as NULL.
-- New test orders explicitly supply their real initial lifecycle and flags.
alter table orders add column inventory_shortage boolean, add column allocated_at timestamptz,
 add column order_status text, add column fulfillment_status text, add column exception_status text,
 add column exception_flags jsonb, add column ship_at timestamptz;
alter table skus add column weight_gram integer;
alter table warehouses add column is_default boolean, add column updated_at timestamptz;
'''
text += '\n'.join([index, shipping, rules, ship_items, resend, cogs])
text += '''\n-- Status columns on new, empty shipment tables; no old status rewrite.
alter table shipments add column status_version int not null default 0,
 add column shipment_status text not null default 'REQUESTED';
commit;
'''
destination.write_text(text, encoding='utf8')
manifest.write_text(json.dumps({'scope': 'Protected QA compatibility prerequisites, not production or complete historical migration',
 'source_sha256': sources,'script_sha256':hashlib.sha256(destination.read_bytes()).hexdigest(),
 'existing_data_updates':0,'historical_order_index_backfill':False,
 'nullable_compatibility_columns':{'orders':['inventory_shortage','allocated_at','order_status','fulfillment_status',
 'exception_status','exception_flags','ship_at'],'skus':['weight_gram'],'warehouses':['is_default','updated_at']}},
 ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(json.dumps({'script':destination.relative_to(ROOT).as_posix(),'migration_sources':len(sources)}))
