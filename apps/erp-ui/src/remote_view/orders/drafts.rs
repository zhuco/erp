//! Pure, bounded persistence contract for one current order edit.
//!
//! The host must durably commit `dispatching` before starting its HTTP PUT. A
//! restored `Unknown` is review-only: it must never be cleared or resent
//! automatically. Decode errors leave the caller's stored bytes untouched.
use super::{parse_cents, stable_child_id};
use erp_grid_core::{ColumnId, RowId};
use serde::{Deserialize, Serialize};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_TEXT_BYTES: usize = 4096;
const MAX_ID_BYTES: usize = 256;
const MAX_VERSION: i32 = i32::MAX;
const TTL_SECONDS: u64 = 7 * 24 * 60 * 60;
const QTY: ColumnId = ColumnId(13);
const PRICE: ColumnId = ColumnId(14);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Scope {
    pub(super) server: String,
    pub(super) tenant: RowId,
    pub(super) actor: RowId,
    pub(super) order_id: String,
    pub(super) schema: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DraftEdit {
    pub(super) line_id: String,
    pub(super) row: RowId,
    pub(super) column: ColumnId,
    pub(super) before: String,
    pub(super) after: String,
    pub(super) expected_version: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OriginalRequest {
    pub(super) path: String,
    pub(super) body: Vec<u8>,
    pub(super) expected_version: i32,
    pub(super) edit: DraftEdit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Recovery {
    Unsent { edit: DraftEdit },
    Unknown { request: OriginalRequest },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub(super) scope: Scope,
    pub(super) saved_at: u64,
    /// Expiry is a review marker. In particular, an expired Unknown remains
    /// recoverable and blocks replacement until a person resolves it.
    pub(super) expires_at: u64,
    pub(super) recovery: Recovery,
}

impl Snapshot {
    pub(super) fn expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    pub(super) fn unknown(&self) -> bool {
        matches!(self.recovery, Recovery::Unknown { .. })
    }
}

pub(super) fn unsent(edit: DraftEdit) -> Recovery {
    Recovery::Unsent { edit }
}

/// State to flush before dispatch. The request bytes are preserved verbatim so
/// any explicit retry can reuse the same CAS/idempotency contract.
pub(super) fn dispatching(request: OriginalRequest) -> Recovery {
    Recovery::Unknown { request }
}

pub(super) fn encode(
    scope: &Scope,
    now: u64,
    current: Option<&Snapshot>,
    recovery: &Recovery,
) -> Result<Vec<u8>, String> {
    validate_scope(scope)?;
    if let Some(current) = current {
        if &current.scope != scope {
            return Err("Order recovery belongs to another account or order".into());
        }
        if current.unknown() {
            return Err("Unknown order request must be resolved before another write".into());
        }
        if let (Recovery::Unsent { edit }, Recovery::Unknown { request }) =
            (&current.recovery, recovery)
            && edit != &request.edit
        {
            return Err("Dispatch request does not match the stored unsent edit".into());
        }
    }
    validate_recovery(scope, recovery)?;
    let snapshot = Snapshot {
        scope: scope.clone(),
        saved_at: now,
        expires_at: now
            .checked_add(TTL_SECONDS)
            .ok_or("Order recovery expiry overflow")?,
        recovery: recovery.clone(),
    };
    let bytes = serde_json::to_vec(&snapshot).map_err(|_| "Cannot encode order recovery")?;
    if bytes.len() > MAX_BYTES {
        return Err("Order recovery exceeds 1 MiB".into());
    }
    Ok(bytes)
}

pub(super) fn decode(scope: &Scope, now: u64, bytes: &[u8]) -> Result<Snapshot, String> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("Order recovery must contain at most 1 MiB".into());
    }
    let snapshot: Snapshot =
        serde_json::from_slice(bytes).map_err(|_| "Cannot decode order recovery")?;
    if &snapshot.scope != scope {
        return Err("Order recovery belongs to another account or order".into());
    }
    validate_scope(&snapshot.scope)?;
    validate_recovery(&snapshot.scope, &snapshot.recovery)?;
    if snapshot.saved_at > now
        || snapshot
            .saved_at
            .checked_add(TTL_SECONDS)
            .is_none_or(|expires_at| expires_at != snapshot.expires_at)
    {
        return Err("Order recovery has invalid timestamps".into());
    }
    // Expiry never turns Unknown into an absent or reusable slot.
    Ok(snapshot)
}

pub(super) fn decode_for_account(
    server: &str,
    tenant: RowId,
    actor: RowId,
    now: u64,
    bytes: &[u8],
) -> Result<Snapshot, String> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("Order recovery must contain at most 1 MiB".into());
    }
    let candidate: Snapshot =
        serde_json::from_slice(bytes).map_err(|_| "Cannot decode order recovery")?;
    if candidate.scope.server != server
        || candidate.scope.tenant != tenant
        || candidate.scope.actor != actor
    {
        return Err("Order recovery belongs to another account".into());
    }
    let scope = candidate.scope.clone();
    decode(&scope, now, bytes)
}

fn validate_scope(scope: &Scope) -> Result<(), String> {
    let server = url::Url::parse(&scope.server).map_err(|_| "Invalid order recovery scope")?;
    if !matches!(server.scheme(), "http" | "https")
        || server.host_str().is_none()
        || !server.username().is_empty()
        || server.password().is_some()
        || server.query().is_some()
        || server.fragment().is_some()
        || scope.server.len() > MAX_TEXT_BYTES
        || scope.tenant.0 == 0
        || scope.actor.0 == 0
        || scope.schema != 1
        || !valid_id(&scope.order_id)
        || scope
            .order_id
            .chars()
            .any(|character| matches!(character, '/' | '?' | '#'))
    {
        return Err("Invalid order recovery scope".into());
    }
    Ok(())
}

fn validate_recovery(scope: &Scope, recovery: &Recovery) -> Result<(), String> {
    let edit = match recovery {
        Recovery::Unsent { edit } => edit,
        Recovery::Unknown { request } => {
            validate_request(scope, request)?;
            &request.edit
        }
    };
    validate_edit(scope, edit)
}

fn validate_edit(scope: &Scope, edit: &DraftEdit) -> Result<(), String> {
    if !valid_id(&edit.line_id)
        || edit.row != stable_child_id(&scope.order_id, &edit.line_id)
        || !matches!(edit.column, QTY | PRICE)
        || !(1..=MAX_VERSION).contains(&edit.expected_version)
        || edit.before.len() > MAX_TEXT_BYTES
        || edit.after.len() > MAX_TEXT_BYTES
        || edit.before.contains('\0')
        || edit.after.contains('\0')
        || edit.before == edit.after
    {
        return Err("Invalid order recovery edit".into());
    }
    Ok(())
}

fn validate_request(scope: &Scope, request: &OriginalRequest) -> Result<(), String> {
    if request.expected_version != request.edit.expected_version
        || request.path
            != format!(
                "/v1/tenants/{}/orders/{}",
                scope.tenant.to_uuid(),
                scope.order_id
            )
        || request.body.is_empty()
        || request.body.len() > MAX_BYTES
    {
        return Err("Invalid original order request".into());
    }
    let body: RequestBody =
        serde_json::from_slice(&request.body).map_err(|_| "Invalid original order request")?;
    if body.expected_version != request.expected_version
        || body.item_edits.len() != 1
        || body.item_edits[0].line_id != request.edit.line_id
        || !body.item_edits[0].matches_edit(&request.edit)
    {
        return Err("Original order request does not match its edit".into());
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES && !value.contains('\0')
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestBody {
    expected_version: i32,
    item_edits: Vec<RequestItemEdit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestItemEdit {
    line_id: String,
    qty: Option<i32>,
    price_cent: Option<i64>,
}

impl RequestItemEdit {
    fn matches_edit(&self, edit: &DraftEdit) -> bool {
        match edit.column {
            QTY => {
                self.price_cent.is_none()
                    && self.qty.is_some_and(|qty| qty > 0)
                    && edit.after.trim().parse::<i32>().ok() == self.qty
            }
            PRICE => {
                self.qty.is_none()
                    && self.price_cent.is_some_and(|price| price >= 0)
                    && parse_cents(&edit.after).ok() == self.price_cent
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(actor: u128, order: &str) -> Scope {
        Scope {
            server: "https://erp.example.test/api/".into(),
            tenant: RowId(1),
            actor: RowId(actor),
            order_id: order.into(),
            schema: 1,
        }
    }

    fn edit(column: ColumnId, after: &str) -> DraftEdit {
        DraftEdit {
            line_id: "line-1".into(),
            row: stable_child_id("order-1", "line-1"),
            column,
            before: "2".into(),
            after: after.into(),
            expected_version: 7,
        }
    }

    fn request() -> OriginalRequest {
        OriginalRequest {
            path: "/v1/tenants/00000000-0000-0000-0000-000000000001/orders/order-1".into(),
            body: br#"{"expected_version":7,"item_edits":[{"line_id":"line-1","qty":3}]}"#.to_vec(),
            expected_version: 7,
            edit: edit(QTY, "3"),
        }
    }

    #[test]
    fn unsent_round_trip_is_scoped_and_contains_no_credentials() {
        let original_scope = scope(2, "order-1");
        let bytes = encode(&original_scope, 100, None, &unsent(edit(QTY, "3"))).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(!text.contains("token"));
        assert!(!text.contains("password"));
        assert!(decode(&original_scope, 101, &bytes).is_ok());
        assert!(decode(&scope(3, "order-1"), 101, &bytes).is_err());
        assert!(decode(&scope(2, "order-2"), 101, &bytes).is_err());
    }

    #[test]
    fn expired_unknown_remains_present_for_manual_review() {
        let scope = scope(2, "order-1");
        let bytes = encode(&scope, 100, None, &dispatching(request())).unwrap();
        let restored = decode(&scope, 100 + TTL_SECONDS, &bytes).unwrap();
        assert!(restored.expired(100 + TTL_SECONDS));
        assert!(restored.unknown());
        let Recovery::Unknown { request: restored } = restored.recovery else {
            panic!("expected unknown recovery");
        };
        assert_eq!(restored, request());
    }

    #[test]
    fn unknown_rejects_changed_request_or_edit() {
        let scope = scope(2, "order-1");
        let mut changed = request();
        changed.body =
            br#"{"expected_version":7,"item_edits":[{"line_id":"line-1","qty":4}]}"#.to_vec();
        assert!(encode(&scope, 100, None, &dispatching(changed)).is_err());
        let mut changed = request();
        changed.path.push_str("/other");
        assert!(encode(&scope, 100, None, &dispatching(changed)).is_err());
    }

    #[test]
    fn corrupt_or_oversized_data_is_rejected_without_a_replacement_value() {
        let scope = scope(2, "order-1");
        assert!(decode(&scope, 100, b"not-json").is_err());
        assert!(decode(&scope, 100, &vec![b'x'; MAX_BYTES + 1]).is_err());
    }

    #[test]
    fn unknown_blocks_replacement_until_explicit_resolution() {
        let scope = scope(2, "order-1");
        let bytes = encode(&scope, 100, None, &dispatching(request())).unwrap();
        let current = decode(&scope, 100 + TTL_SECONDS, &bytes).unwrap();
        assert!(
            encode(
                &scope,
                100 + TTL_SECONDS,
                Some(&current),
                &unsent(edit(QTY, "4"))
            )
            .is_err()
        );
    }

    #[test]
    fn unsent_can_be_atomically_advanced_to_dispatching_unknown() {
        let scope = scope(2, "order-1");
        let bytes = encode(&scope, 100, None, &unsent(edit(QTY, "3"))).unwrap();
        let current = decode(&scope, 101, &bytes).unwrap();
        assert!(encode(&scope, 101, Some(&current), &dispatching(request())).is_ok());
    }
}
