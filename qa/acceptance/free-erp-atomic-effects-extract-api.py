"""Move only the current atomic handler into an independently formattable leaf."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
parent = ROOT / "crates/erp-api/src/manufacturing.rs"
leaf = ROOT / "crates/erp-api/src/manufacturing/atomic_effects.rs"
old = parent.read_bytes()
source = old.decode("utf8").replace("\r\n", "\n")
start = source.index('#[derive(Debug, Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct AtomicEffectRequest')
end = source.index('async fn transition_work_order_route(', start)
block = source[start:end]
assert block.count('async fn commit_atomic_effect_route(') == 1
with (ROOT / 'qa/acceptance/free-erp-atomic-effects-api-before-extract.txt').open('xb') as output:
    output.write(old)
with leaf.open('x', encoding='utf8') as output:
    output.write('//! Owned precise material issue and immutable original HTTP response.\nuse super::*;\n\n' + block.replace('async fn commit_atomic_effect_route(', 'pub(super) async fn commit_atomic_effect_route(', 1))
source = source[:start] + source[end:]
assert 'mod atomic_effects;' not in source
source = source.replace('mod inspection_outcomes;', 'mod atomic_effects;\nuse atomic_effects::commit_atomic_effect_route;\n\nmod inspection_outcomes;', 1)
assert parent.read_bytes() == old, 'parallel parent edit; do not overwrite'
parent.write_bytes(source.replace('\n', '\r\n').encode('utf8'))
report = {'parent':parent.relative_to(ROOT).as_posix(), 'leaf':leaf.relative_to(ROOT).as_posix(),
          'before_sha256':hashlib.sha256(old).hexdigest(), 'after_sha256':hashlib.sha256(parent.read_bytes()).hexdigest()}
with (ROOT/'qa/acceptance/free-erp-atomic-effects-api-extraction.json').open('x',encoding='utf8') as output:
    output.write(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
