"""Extract the existing material posting kernel without changing its statements."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
path = ROOT / "crates/erp-repo/src/shared_contract/material_moves.rs"
raw = path.read_bytes()
text = raw.decode("utf8")
newline = "\r\n" if "\r\n" in text else "\n"
source = text.replace("\r\n", "\n")
if "pub(super) async fn append_material_move_with_tx" in source:
    raise SystemExit("kernel already extracted; refusing to rerun")
start = source.index('        if status != "RELEASED"', source.index("pub async fn post_material_move("))
end = source.index("        let receipt = succeeded(request_id, payload_hash, &id.to_string());", start)
body = source[start:end]
assert body.count("apply_stock(&mut tx, self, tenant, input, id).await?;") == 1
assert body.count("material_balance(&mut tx, tenant, input.work_order_id).await?;") == 1
replacement = "        let (id, next) = append_material_move_with_tx(&mut tx, self, tenant, input, &status).await?;\n"
source = source[:start] + replacement + source[end:]
body = "\n".join(line[4:] if line.startswith("    ") else line for line in body.rstrip().splitlines())
body = body.replace("&mut tx", "tx").replace("&mut *tx", "&mut **tx")
body = body.replace("apply_stock(tx, self, tenant, input, id)", "apply_stock(tx, repo, tenant, input, id)")
kernel = '''
/// The caller holds the work-order and command locks and validates physical ownership.
/// This kernel does not commit or write a command receipt; the caller commits all effects.
pub(super) async fn append_material_move_with_tx(
    tx: &mut Transaction<'_, Postgres>,
    repo: &SharedContractRepo<'_>,
    tenant: TenantId,
    input: MaterialMoveInput<'_>,
    status: &str,
) -> anyhow::Result<(Uuid, MaterialBalance)> {
''' + body + "\n    Ok((id, next))\n}\n"
marker = "\nasync fn apply_stock("
assert source.count(marker) == 1
source = source.replace(marker, kernel + marker, 1)
archive = BASE / "free-erp-atomic-effects-material-before-refactor.rs.txt"
with archive.open("xb") as output:
    output.write(raw)
assert path.read_bytes() == raw, "material leaf changed during extraction"
updated = source.replace("\n", newline).encode("utf8")
path.write_bytes(updated)
print(json.dumps({"file": str(path.relative_to(ROOT)), "before_sha256": hashlib.sha256(raw).hexdigest(),
                  "after_sha256": hashlib.sha256(updated).hexdigest(), "kernel_source_lines": len(body.splitlines())}))
