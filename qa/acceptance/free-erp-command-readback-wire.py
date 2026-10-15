"""Scoped fresh-source wiring; preserves unrelated edits in shared parents."""
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"

def edit(name, transform):
    path = ROOT / name
    before = path.read_bytes()
    text = before.decode("utf-8-sig")
    result = transform(text)
    if result == text:
        raise SystemExit(f"no intended edit found: {name}")
    archive = BASE / ("free-erp-command-readback-wire-before-" + name.replace("/", "_") + ".txt")
    if archive.exists():
        raise SystemExit("refusing to replace pre-wire inputs")
    archive.write_bytes(before)
    if path.read_bytes() != before:
        raise SystemExit(f"concurrent source change detected: {name}")
    path.write_bytes(result.encode("utf8"))
    print(json.dumps({"file": name, "before_sha256": hashlib.sha256(before).hexdigest(),
                      "after_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}))

def repository(text):
    text = text.replace("mod bom_structure;", "mod bom_structure;\nmod command_readback;", 1)
    text = text.replace("pub use inspection_outcomes::", "pub use command_readback::CommandReadbackError;\n\npub use inspection_outcomes::", 1)
    start = text.index("    /// Moves an accepted command")
    end = text.index("    /// Commits stock ledger", start)
    replacement = '''    /// Reads the original status after a client observation; clients cannot advance it.
    pub async fn advance_command(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        reported_status: &str,
        _error_text: &str,
        _recoverable: bool,
    ) -> anyhow::Result<Result<SharedWrite, ReceiptError>> {
        self.observe_command_status(tenant_id, request_id, payload_hash, reported_status).await
    }

    /// Reads the original receipt after session loss. Compatibility stock fields never execute.
    pub async fn lookup_lost_command(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        actor_user_id: &str,
        revoked_session_id: Uuid,
        current_session_id: Uuid,
        _client_unknown: bool,
        _warehouse_id: Uuid,
        _sku_id: Uuid,
        _qty_units: i64,
    ) -> anyhow::Result<Result<SharedWrite, ReceiptError>> {
        self.lookup_lost_command_readonly(command_readback::LostCommandRead {
            tenant: tenant_id,
            request: request_id,
            payload_hash,
            actor: actor_user_id,
            revoked_session: revoked_session_id,
            current_session: current_session_id,
        }).await
    }

    /// Reads actual typed command receipts even when generic prototype metadata is absent.
    pub async fn read_command(
        &self,
        tenant_id: TenantId,
        request_id: &str,
    ) -> anyhow::Result<Option<(String, String, i64, String, bool, String)>> {
        self.read_command_result(tenant_id, request_id).await
    }

'''
    text = text[:start] + replacement + text[end:]
    for symbol in ["client_unknown_is_not_failure", "committed_effect_survives_session_loss", "lost_response_retry_decision", "command_status"]:
        if len(re.findall(r"\b" + symbol + r"\b", text)) == 1:
            text = re.sub(r"\b" + symbol + r",\s*", "", text, count=1)
    # An accepted prototype has an object reference, not a committed business effect.
    start = text.index("    pub async fn open_command(")
    end = text.index("    /// Reads the original status", start)
    portion = text[start:end]
    assert "effect_id: object_id.to_string()," in portion
    text = text[:start] + portion.replace("effect_id: object_id.to_string(),", "effect_id: String::new(),", 1) + text[end:]
    return text

def api(text):
    text = text.replace("mod inspection_outcomes;", "mod inspection_outcomes;\nmod command_readback;", 1)
    names = ["advance_command_route", "read_command_route", "lookup_lost_command_route", "read_restarted_command_route"]
    for name in names:
        start = text.index("async fn " + name + "(")
        end = text.index("\n}\n", start) + 3
        old = text[start:end]
        pattern = "Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!(\"{error}\")).into_response(),"
        if old.count(pattern) != 1:
            raise SystemExit(f"expected one exact readback error branch: {name}")
        text = text[:start] + old.replace(pattern, "Err(error) => command_readback::error_response(error),", 1) + text[end:]
    return text

def domain(text):
    start = text.index("/// After a lost response,")
    end = text.index("/// A revoked session", start)
    return text[:start] + '''/// Lost-response lookup only observes the original server status.
/// Neither a client timeout nor its disappearance authorizes a new business effect.
pub fn lost_response_retry_decision(
    stored_status: CommandStatus,
    _client_unknown: bool,
) -> Result<&'static str, ReceiptError> {
    if stored_status.is_complete() { Ok("KEEP") } else { Ok("LOOKUP") }
}

''' + text[end:]

def domain_test(text):
    text = text.replace("fn lost_response_keeps_the_stored_status_and_allows_one_finish()", "fn lost_response_only_reads_the_stored_status()", 1)
    return text.replace('lost_response_retry_decision(CommandStatus::Accepted, false),\n        Ok("COMPLETE")', 'lost_response_retry_decision(CommandStatus::Accepted, false),\n        Ok("LOOKUP")', 1)

edit("crates/erp-repo/src/shared_contract.rs", repository)
edit("crates/erp-api/src/manufacturing.rs", api)
edit("crates/erp-domain/src/receipt.rs", domain)
edit("crates/erp-domain/src/contract_tests.rs", domain_test)
