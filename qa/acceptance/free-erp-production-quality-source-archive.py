"""Preserve exact original source bytes and record concurrent copy changes."""
import hashlib
import json
from pathlib import Path

root=Path(__file__).resolve().parents[2]
base=root/"qa/acceptance"
files=["crates/erp-repo/src/shared_contract.rs","crates/erp-repo/src/shared_contract/production_receipt_release.rs",
       "crates/erp-api/src/manufacturing.rs","crates/erp-api/tests/free_erp_acceptance/production_quality_sources.rs",
       "crates/erp-domain/src/manufacturing.rs"]
records=[]
for name in files:
    path=root/name
    raw=path.read_bytes()
    destination=base/("free-erp-production-quality-source-original-"+name.replace("/","_")+".txt")
    with destination.open("xb") as stream:
        stream.write(raw)
    records.append(dict(path=name,archive=destination.relative_to(root).as_posix(),
                        sha256=hashlib.sha256(raw).hexdigest(),
                        unchanged_during_copy=raw==path.read_bytes()))
with (base/"free-erp-production-quality-source-original-inputs.json").open("x",encoding="utf8") as stream:
    json.dump(dict(files=records),stream,indent=2,ensure_ascii=False)
    stream.write("\n")
print(json.dumps(dict(archived=len(records),all_stable=all(row["unchanged_during_copy"] for row in records))))
