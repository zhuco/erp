"""Protected 77-table snapshots, including the immutable current actor evidence."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT/"qa/acceptance"
PREFIX = "free-erp-receipt-authority-"
parser = argparse.ArgumentParser()
parser.add_argument("action",choices=["prepare","snapshot"])
parser.add_argument("stage",nargs="?")
args = parser.parse_args()
psql = ["G:/PostgreSQL/17/bin/psql.exe","-X","-q","-t","-A","-h","127.0.0.1","-p","15438",
        "-U","erp_grid_test","-d","erp_shared_contract_test","-v","ON_ERROR_STOP=1"]
sql = BASE/(PREFIX+"readback.sql")
if args.action=="prepare":
    original = BASE/"free-erp-receipt-values-readback.sql"
    raw = original.read_text(encoding="utf-8-sig")
    if hashlib.sha256(original.read_bytes()).hexdigest()!="f50070457c007034aa79b6ae0f286a1bea47d33673ea3cb2539392d889864126" or raw.count("'partner_balances'")!=3:
        raise SystemExit("review the immutable source snapshot before extension")
    names = ["shared_receipt_value_actors"]
    if any("'"+name+"'" in raw for name in names):
        raise SystemExit("review already-selected source tables")
    raw = raw.replace("'partner_balances'","'partner_balances',"+",".join("'"+name+"'" for name in names)).replace("'selected_tables',76","'selected_tables',77")
    with sql.open("x",encoding="utf8") as output:
        output.write(raw)
    catalog_sql = """begin transaction read only;
do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438
then raise exception 'protected trade QA identity required'; end if; end $$;
select jsonb_object_agg(table_name,cols) from
(select table_name,jsonb_agg(column_name order by column_name) cols from information_schema.columns
where table_schema='public' and data_type='timestamp with time zone' group by table_name) x;
rollback;"""
    result = subprocess.run(psql+["-c",catalog_sql],cwd=ROOT,capture_output=True,check=True)
    catalog = json.loads(result.stdout.decode("utf8").strip())
    with (BASE/(PREFIX+"timestamp-catalog.json")).open("x",encoding="utf8") as output:
        json.dump(catalog,output,indent=2)
        output.write("\n")
    print(json.dumps({"prepared_tables":77,"sql_sha256":hashlib.sha256(sql.read_bytes()).hexdigest()}))
else:
    if not args.stage or not re.fullmatch(r"[a-z0-9-]+",args.stage):
        raise SystemExit("invalid immutable snapshot stage")
    destination = BASE/(PREFIX+args.stage+".jsonl")
    errors = BASE/(PREFIX+args.stage+".stderr.log")
    if destination.exists() or errors.exists():
        raise SystemExit("refusing to replace snapshot evidence")
    with destination.open("xb") as output,errors.open("xb") as error_output:
        result = subprocess.run(psql+["-f",str(sql)],cwd=ROOT,stdout=output,stderr=error_output,check=False)
    sha = hashlib.sha256()
    with destination.open("rb") as stream:
        for chunk in iter(lambda:stream.read(1048576),b""):
            sha.update(chunk)
    print(json.dumps({"stage":args.stage,"exit_code":result.returncode,"bytes":destination.stat().st_size,"sha256":sha.hexdigest()}))
    raise SystemExit(result.returncode)
