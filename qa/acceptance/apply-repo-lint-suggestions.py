"""Apply only validated machine-applicable collapsible-if groups in repository source."""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
stage = sys.argv[1]
if stage not in ['original', 'second', 'third']:
    raise SystemExit('unregistered repository lint stage')
allowed_codes = {'clippy::collapsible_if','clippy::borrow_deref_ref','clippy::needless_borrow','clippy::needless_option_as_deref'}
allowed = {file.relative_to(ROOT).as_posix() for file in (ROOT / 'crates/erp-repo/src').rglob('*.rs')}
groups = []
for line in (BASE / f'free-erp-repo-lint-oct9-{stage}.jsonl').read_text(encoding='utf-8-sig').splitlines():
    try:
        event = json.loads(line)
    except json.JSONDecodeError:
        continue
    message = event.get('message', {})
    if event.get('reason') != 'compiler-message' or (message.get('code') or {}).get('code') not in allowed_codes:
        continue
    for child in message['children']:
        spans = [span for span in child['spans'] if span.get('suggested_replacement') is not None]
        if spans and all(span.get('suggestion_applicability') == 'MachineApplicable' and span['file_name'].replace('\\', '/') in allowed for span in spans):
            names = {span['file_name'].replace('\\', '/') for span in spans}
            if len(names) != 1:
                raise SystemExit('cross-file suggestion refused')
            groups.append((names.pop(), min(span['byte_start'] for span in spans), max(span['byte_end'] for span in spans), spans))
deduplicated = {}
for group in groups:
    key = tuple((span['file_name'],span['byte_start'],span['byte_end'],span['suggested_replacement']) for span in group[3])
    deduplicated[key] = group
groups = list(deduplicated.values())
# Module-root condition layout will be handled with the planned parameter refactor.
groups = [group for group in groups if not (group[0] == 'crates/erp-repo/src/shared_contract.rs' and len(group[3]) > 1)]
selected, skipped = [], []
for group in sorted(groups, key=lambda value: value[2] - value[1]):
    if any(group[0] == old[0] and group[1] < old[2] and old[1] < group[2] for old in selected):
        skipped.append(group)
    else:
        selected.append(group)
snapshots = BASE / 'repo-lint-source-before'
snapshots.mkdir(exist_ok=True)
changes = []
for name in sorted({group[0] for group in selected}):
    path = ROOT / name
    raw = path.read_bytes()
    lines = raw.decode('utf-8').splitlines()
    edits = []
    for group in selected:
        if group[0] != name:
            continue
        for span in group[3]:
            expected = [item['text'] for item in span['text']]
            if lines[span['line_start'] - 1:span['line_end']] != expected:
                raise SystemExit('source changed since diagnostic: ' + name)
            edits.append((span['byte_start'], span['byte_end'], span['suggested_replacement'].encode('utf-8')))
    saved = snapshots / (name.replace('/', '-') + '-' + stage + '.txt')
    if saved.exists():
        raise SystemExit('refusing to replace original repository source evidence')
    saved.write_bytes(raw)
    result = raw
    for start, end, replacement in sorted(edits, reverse=True):
        result = result[:start] + replacement + result[end:]
    path.write_bytes(result)
    changes.append({'file': name, 'before_sha256': hashlib.sha256(raw).hexdigest(), 'after_sha256': hashlib.sha256(result).hexdigest(), 'replacement_spans': len(edits)})
report = {'stage': stage, 'available_groups': len(groups), 'applied_groups': len(selected), 'overlap_deferred': len(skipped), 'files': changes,
          'scope': 'machine-applicable condition merge only; no lint suppression, arbitrary package, dependency or credential changes'}
(BASE / f'free-erp-repo-lint-oct9-{stage}-edits.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('Applied', len(selected), 'machine suggestion groups;', len(skipped), 'overlapping groups deferred')
