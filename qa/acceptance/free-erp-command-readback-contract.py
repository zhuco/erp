"""Update only command-read contracts from the tested read-only protocol."""
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
path = ROOT / "openapi/src/paths.yaml"
before = path.read_bytes()
source = before.decode("utf-8-sig")

def modify_path(text, key, transform):
    header = re.search(re.escape(key) + r":\r?\n", text)
    if not header:
        raise SystemExit(f"missing exact path {key}")
    start = header.start()
    next_path = re.search(r"^/", text[header.end():], re.M)
    end = header.end()+next_path.start() if next_path else len(text)
    block = text[start:end]
    result = transform(block.replace("\r\n", "\n"))
    if "\r\n" in block:
        result = result.replace("\n", "\r\n")
    return text[:start] + result + text[end:]

def command(block):
    sections = block.split("  post:\n", 1)
    sections[0] = re.sub(r"    description: [^\n]+", "    description: Read-only original receipt and effect, including typed business commands without generic metadata. Object and effect identities are distinct. Source version 0 means unavailable original version metadata. Missing commands create no receipt. Historical generic terminal states without authoritative business proof require reconciliation.", sections[0], count=1)
    sections[0] += '      "409":\n        description: Original result or historical generic completion requires reconciliation\n'
    sections[1] = re.sub(r"    summary: [^\n]+", "    summary: Observe the original command status without changing it", sections[1], count=1)
    sections[1] = re.sub(r"    description: [^\n]+", "    description: UNKNOWN or an identical observed status returns the original receipt with replay=true. A different client-reported status is refused. Client error_text and recoverable never modify server facts. Missing commands return 404 without occupying their identity. This endpoint writes no receipt, metadata, stock, log or completion time.", sections[1], count=1)
    sections[1] = sections[1].replace("description: stored status kept or advanced", "description: Original stored status returned unchanged")
    sections[1] = sections[1].replace("description: the payload hash does not match the opened command", "description: Payload conflict, forbidden client status transition or original result requires reconciliation")
    sections[1] = sections[1].replace("description: the command was not opened or the reported status is invalid", "description: Command readback database failure")
    sections[1] += '      "404":\n        description: Original command missing; no unknown or failed placeholder is stored\n'
    return "  post:\n".join(sections)

def lost(block):
    block = re.sub(r"    description: [^\n]+", "    description: A read-only original-result lookup. The authenticated actor owns both sessions; the old session is revoked and the new session is active, unexpired and uses the current credential epoch for an active user. UNKNOWN and non-UNKNOWN both return the original status. Compatibility warehouse_id, sku_id and qty_units never execute or prove a business source. No stock, metadata, log or receipt is written, and a missing command creates no placeholder.", block, count=1)
    block = block.replace("description: stored command returned or completed once", "description: Original receipt and effect returned unchanged with replay=true")
    block += '      "403":\n        description: Sessions are not owned, active, unexpired and current\n      "404":\n        description: Original command missing\n'
    return block

source = modify_path(source, "/v1/tenants/{tenant_id}/commands/{request_id}", command)
source = modify_path(source, "/v1/tenants/{tenant_id}/commands/{request_id}/lost-lookups", lost)
source = modify_path(source, "/v1/tenants/{tenant_id}/commands/{request_id}/restarts", lambda block: re.sub(
    r"    description: [^\n]+", "    description: Read the original receipt without writing or executing. restarted=true is only a compatibility client flag and does not prove a real process restart. The same original effect and unknown source-version policy apply; historical generic completion without proof requires reconciliation.", block, count=1))
archive = BASE / "free-erp-command-readback-openapi-paths-before.txt"
if archive.exists():
    raise SystemExit("refusing to replace original command contract")
archive.write_bytes(before)
if path.read_bytes() != before:
    raise SystemExit("concurrent OpenAPI edit; reviewed patch must be reapplied to fresh source")
path.write_bytes(source.encode("utf8"))
print(json.dumps({"file": path.relative_to(ROOT).as_posix(), "before_sha256": hashlib.sha256(before).hexdigest(),
                  "after_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}))
