"""Capture independent protected QA facts without replacing prior evidence."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'


def file_hash(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


parser = argparse.ArgumentParser()
parser.add_argument('stage')
mode = parser.add_mutually_exclusive_group()
mode.add_argument('--legacy', action='store_true')
mode.add_argument('--schema', action='store_true')
args = parser.parse_args()
if not re.fullmatch(r'[a-z0-9-]+', args.stage):
    raise SystemExit('invalid evidence stage')
sql = BASE / ('free-erp-inventory-legacy-reservations-stream-readback.sql' if args.legacy
              else 'free-erp-inventory-kit-sources-schema-readback.sql' if args.schema
              else 'free-erp-inventory-kit-sources-stream-readback.sql')
destination = BASE / f'free-erp-inventory-kit-sources-qa-{args.stage}.jsonl'
error_file = BASE / f'free-erp-inventory-kit-sources-qa-{args.stage}.stderr.log'
if destination.exists() or error_file.exists():
    raise SystemExit('refusing to replace historical readback')
with destination.open('wb') as output, error_file.open('wb') as errors:
    completed = subprocess.run([
        'G:/PostgreSQL/17/bin/psql.exe', '-X', '-q', '-t', '-A', '-h', '127.0.0.1',
        '-p', '15438', '-U', 'erp_grid_test', '-d', 'erp_shared_contract_test',
        '-v', 'ON_ERROR_STOP=1', '-f', str(sql),
    ], cwd=ROOT, stdout=output, stderr=errors, check=False)
print(json.dumps({'stage': args.stage, 'exit_code': completed.returncode,
                  'bytes': destination.stat().st_size,
                  'sha256': file_hash(destination),
                  'sql_sha256': file_hash(sql)}))
raise SystemExit(completed.returncode)
