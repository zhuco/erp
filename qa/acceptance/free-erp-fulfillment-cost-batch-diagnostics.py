"""Derive current strict dependency lint facts without claiming unreached API lint."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

base = Path(__file__).resolve().parent
destination = base / 'free-erp-fulfillment-cost-batch-clippy-diagnostics.json'
if destination.exists():
    raise SystemExit('refusing lint evidence overwrite')
raw = (base / 'free-erp-fulfillment-cost-batch-clippy.log').read_bytes()
log = raw.decode('utf-16' if raw[:2] in (b'\xff\xfe', b'\xfe\xff') else 'utf-8-sig')
counts = Counter()
for block in re.split(r'(?:^|\n)error:', log)[1:]:
    location = re.search(r'--> ([^:\r\n]+):(\d+):(\d+)', block)
    if location:
        counts[location[1].replace('\\', '/')] += 1
failures = re.findall(r'could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors', log)
failed = bool(failures or 'Cargo clippy failed' in log)
owned = set(json.loads((base / 'free-erp-fulfillment-cost-batch-original-owned.json').read_text(encoding='utf8')))
before = json.loads((base / 'free-erp-inventory-consumers-fulfillment-cost-batch-clippy-before-source.json').read_text(encoding='utf8'))['files']
after = json.loads((base / 'free-erp-inventory-consumers-fulfillment-cost-batch-clippy-after-source.json').read_text(encoding='utf8'))['files']
changed = [name for name in sorted(set(before) | set(after)) if before.get(name) != after.get(name)]
report = dict(status='FAIL' if failed else 'PASS' if 'Finished' in log else 'UNKNOWN',
    command='Invoke-ErpBuild.ps1 -Action clippy -Scope api', compiler_failure_summaries=failures,
    primary_diagnostics_by_file=dict(sorted(counts.items())),
    observed_owned_diagnostics={name: count for name, count in counts.items() if name in owned},
    api_lint='FAILED' if any(row[0] == 'erp-api' for row in failures) else 'NOT_REACHED' if failed else 'PASS',
    repository_test_lint='NOT_RUN_IN_THIS_SCOPE', source_window=dict(owned_changed=[name for name in changed if name in owned], other_changed=[name for name in changed if name not in owned]),
    log_sha256=hashlib.sha256(raw).hexdigest())
destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
print(json.dumps({key: report[key] for key in ('status', 'compiler_failure_summaries', 'observed_owned_diagnostics', 'api_lint', 'source_window')}))
