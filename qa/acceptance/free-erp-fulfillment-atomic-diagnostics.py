"""Record strict Clippy blockers without presenting unreached API lint as passed."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

base = Path(__file__).resolve().parent
raw = (base / 'free-erp-fulfillment-atomic-clippy.log').read_bytes()
log = raw.decode('utf-16' if raw[:2] in (b'\xff\xfe', b'\xfe\xff') else 'utf-8-sig')
counts = Counter()
for block in re.split(r'(?:^|\n)error:', log)[1:]:
    location = re.search(r'--> ([^:\r\n]+):(\d+):(\d+)', block)
    if location:
        counts[location[1].replace('\\', '/')] += 1
owned = set(json.loads((base / 'free-erp-fulfillment-atomic-original-owned.json').read_text(encoding='utf8')))
owned.update(['crates/erp-repo/src/fulfillment/context.rs', 'crates/erp-api/src/lib.rs'])
report = {'status': 'FAIL', 'command': 'Invoke-ErpBuild.ps1 -Action clippy -Scope api',
          'compiler_failure_summaries': re.findall(r'could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors', log),
          'primary_diagnostics_by_file': dict(sorted(counts.items())),
          'observed_owned_diagnostics': {name: count for name, count in counts.items() if name in owned},
          'api_lint': 'NOT_REACHED', 'repository_test_lint': 'NOT_RUN_IN_THIS_SCOPE',
          'log_sha256': hashlib.sha256(raw).hexdigest()}
output = base / 'free-erp-fulfillment-atomic-clippy-diagnostics.json'
if output.exists():
    raise SystemExit('refusing diagnostic evidence overwrite')
output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
print(json.dumps({key: report[key] for key in ('status', 'compiler_failure_summaries', 'observed_owned_diagnostics', 'api_lint')}))
