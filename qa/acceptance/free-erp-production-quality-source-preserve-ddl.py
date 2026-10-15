"""Preserve the exact first applied QA-only drafts before fixing their unreleased chain."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
entries = []
for name in ["0291_inspection_source_bindings.sql", "0292_inspection_command_identity.sql"]:
    source = ROOT / "migrations" / name
    raw = source.read_bytes()
    target = QA / f"free-erp-production-quality-source-first-applied-{name}.txt"
    with target.open("xb") as stream:
        stream.write(raw)
    entries.append(dict(source=source.relative_to(ROOT).as_posix(), archive=target.relative_to(ROOT).as_posix(),
                        sha256=hashlib.sha256(raw).hexdigest(), unchanged_during_copy=source.read_bytes() == raw))
destination = QA / "free-erp-production-quality-source-first-applied-ddl.json"
with destination.open("x", encoding="utf8") as stream:
    stream.write(json.dumps(dict(scope="First QA-only drafts; no production migration was performed", files=entries), indent=2) + "\n")
print(json.dumps(entries))
