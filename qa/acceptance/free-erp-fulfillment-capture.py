"""Independent protected QA facts for the real existing OMS stock consumer."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
parser = argparse.ArgumentParser()
parser.add_argument('stage')
args = parser.parse_args()
if not re.fullmatch(r'[a-z0-9-]+', args.stage):
    raise SystemExit('invalid immutable capture stage')
sql = BASE / 'free-erp-fulfillment-readback.sql'
output = BASE / f'free-erp-fulfillment-{args.stage}.jsonl'
errors = BASE / f'free-erp-fulfillment-{args.stage}.stderr.log'
if output.exists() or errors.exists():
    raise SystemExit('refusing to replace historical facts')
with output.open('wb') as facts, errors.open('wb') as diagnostic:
    result = subprocess.run(['G:/PostgreSQL/17/bin/psql.exe', '-X', '-q', '-t', '-A',
        '-h', '127.0.0.1', '-p', '15438', '-U', 'erp_grid_test', '-d', 'erp_shared_contract_test',
        '-v', 'ON_ERROR_STOP=1', '-f', str(sql)], cwd=ROOT, stdout=facts, stderr=diagnostic, check=False)
digest = hashlib.sha256()
with output.open('rb') as facts:
    for block in iter(lambda: facts.read(1024 * 1024), b''):
        digest.update(block)
print(json.dumps({'stage': args.stage, 'exit_code': result.returncode, 'bytes': output.stat().st_size,
                  'sha256': digest.hexdigest(), 'sql_sha256': hashlib.sha256(sql.read_bytes()).hexdigest()}))
raise SystemExit(result.returncode)
