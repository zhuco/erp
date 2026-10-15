"""Read-only final artifact/source qualification; retain historical PARTIAL scope."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
prefix = 'free-erp-inventory-location-reservations'
destination = base / f'{prefix}-delivery.json'
if destination.exists():
    raise SystemExit('refusing to overwrite delivery qualification')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


proof = json.loads((base / f'{prefix}-repair.json').read_text(encoding='utf8'))
docs = json.loads((base / 'free-erp-specification-docs.json').read_text(encoding='utf8'))
before = json.loads((base / 'free-erp-inventory-consumers-location-reservations-full-after-source.json').read_text(encoding='utf8'))['files']
after = json.loads((base / 'free-erp-inventory-consumers-location-reservations-delivery-source.json').read_text(encoding='utf8'))['files']
changed_owned = [name for name, digest in proof['owned_current_sha256'].items() if sha(root/name) != digest]
changed_docs = [name for name, digest in docs['document_sha256'].items() if sha(root/name) != digest]
changed_historical = [name for name, digest in proof['historical_sha256'].items() if sha(root/name) != digest]
changes = sorted(name for name in before.keys()|after.keys() if before.get(name) != after.get(name))
qualified = not changed_owned and not changed_docs and not changed_historical and docs['status']=='PASS'
report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'PARTIAL',
          'artifact_qualification': 'PASS' if qualified else 'FAIL',
          'owned_inputs': len(proof['owned_current_sha256']), 'documents': len(docs['document_sha256']),
          'historical_artifacts': len(proof['historical_sha256']), 'changed_owned_inputs': changed_owned,
          'changed_documents': changed_docs, 'changed_historical_artifacts': changed_historical,
          'all_input_count': len(after), 'delivery_dependency_drift_since_full_readback': changes,
          'documentation_checks': docs['checks'],
          'proof_sha256': sha(base/f'{prefix}-repair.json'),
          'documentation_report_sha256': sha(base/'free-erp-specification-docs.json'),
          'limits': proof['remaining_gates']}
destination.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(json.dumps({key:report[key] for key in ['status','artifact_qualification','owned_inputs','documents',
      'historical_artifacts','changed_owned_inputs','changed_documents','changed_historical_artifacts',
      'delivery_dependency_drift_since_full_readback']},ensure_ascii=False))
raise SystemExit(0 if qualified else 1)
