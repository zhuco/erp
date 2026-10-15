"""Keep owned and concurrent source drift separate in immutable input windows."""
import argparse
import json
from pathlib import Path

base = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('stage')
args = parser.parse_args()
owned = set(json.loads((base / 'free-erp-fulfillment-atomic-original-owned.json').read_text(encoding='utf8')))
owned.update(['crates/erp-repo/src/fulfillment/context.rs', 'crates/erp-api/src/lib.rs'])
before = base / f'free-erp-inventory-consumers-fulfillment-atomic-{args.stage}-before-source.json'
after = base / f'free-erp-inventory-consumers-fulfillment-atomic-{args.stage}-after-source.json'
left = json.loads(before.read_text(encoding='utf8'))['files']
right = json.loads(after.read_text(encoding='utf8'))['files']
changed = [name for name in sorted(set(left) | set(right)) if left.get(name) != right.get(name)]
report = {'stage': args.stage, 'input_count_before': len(left), 'input_count_after': len(right),
          'owned_input_count': len(owned), 'owned_changed': [name for name in changed if name in owned],
          'other_changed': [name for name in changed if name not in owned],
          'source_qualification': 'PARTIAL' if changed else 'PASS'}
output = base / f'free-erp-fulfillment-atomic-{args.stage}-source-window.json'
if output.exists():
    raise SystemExit('refusing to replace source-window evidence')
output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
print(json.dumps(report))
