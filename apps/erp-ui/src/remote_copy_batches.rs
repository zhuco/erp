//! Pure wire adapter for durable product copy batches.
//!
//! The host owns authentication and I/O. This module only validates bounded
//! request and response payloads and constructs tenant-scoped API requests.
use crate::http::HttpRequest;
use crate::remote_selection::{self, Credential};
use erp_grid_core::RowId;
use std::collections::BTreeSet;

const MAX_BODY: usize = 1024 * 1024;
const MAX_BATCHES: usize = 20;
const MAX_TOTAL: u64 = 100_000;
const MAX_EXCLUDED: usize = 10_000;
const ADVANCE_ITEMS: u8 = 25;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartBody {
    pub(crate) batch_id: RowId,
    pub(crate) token: RowId,
    pub(crate) excluded: Vec<RowId>,
    pub(crate) confirmed_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BatchStatus {
    pub(crate) batch_id: RowId,
    pub(crate) total: u64,
    pub(crate) applied: u64,
    pub(crate) rejected: u64,
    pub(crate) cancelled: u64,
    pub(crate) pending: u64,
    pub(crate) unknown: u64,
    pub(crate) cancel_requested: bool,
}

impl BatchStatus {
    pub(crate) fn terminal(&self) -> bool {
        self.pending == 0 && self.unknown == 0
    }
}

pub(crate) fn encode_start(tenant: RowId, body: &StartBody) -> Result<HttpRequest, String> {
    validate_id(tenant, "tenant")?;
    validate_id(body.batch_id, "batch")?;
    validate_id(body.token, "selection token")?;
    if !(1..=MAX_TOTAL).contains(&body.confirmed_count) || body.excluded.len() > MAX_EXCLUDED {
        return Err("Invalid product copy batch size".into());
    }
    let mut excluded = BTreeSet::new();
    if body
        .excluded
        .iter()
        .any(|id| id.0 == 0 || !excluded.insert(*id))
    {
        return Err("Copy batch exclusions must be unique non-nil IDs".into());
    }
    let json = serde_json::to_vec(body).map_err(|_| "Cannot encode product copy batch")?;
    if json.len() > MAX_BODY {
        return Err("Product copy batch request exceeds 1 MiB".into());
    }
    Ok(HttpRequest {
        path: collection_path(tenant),
        query: vec![],
        json: Some(json),
        bearer: None,
    })
}

pub(crate) fn encode_list(tenant: RowId) -> Result<HttpRequest, String> {
    validate_id(tenant, "tenant")?;
    Ok(HttpRequest {
        path: collection_path(tenant),
        query: vec![],
        json: None,
        bearer: None,
    })
}

pub(crate) fn encode_retry_review(
    tenant: RowId,
    batch: RowId,
    request_id: RowId,
) -> Result<HttpRequest, String> {
    validate_id(tenant, "tenant")?;
    validate_id(batch, "batch")?;
    validate_id(request_id, "retry review request")?;
    let json = serde_json::to_vec(&serde_json::json!({"request_id": request_id}))
        .map_err(|_| "Cannot encode product copy retry review")?;
    Ok(HttpRequest {
        path: format!(
            "{}/{}/retry-review",
            collection_path(tenant),
            batch.to_uuid()
        ),
        query: vec![],
        json: Some(json),
        bearer: None,
    })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RetryReviewWire {
    batch_id: RowId,
    request_id: RowId,
    selection: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RetryReview {
    pub(crate) batch_id: RowId,
    pub(crate) request_id: RowId,
    pub(crate) selection: Credential,
}

pub(crate) fn decode_retry_review(
    bytes: &[u8],
    expected_batch: RowId,
    expected_request: RowId,
    now: u64,
) -> Result<RetryReview, String> {
    if bytes.len() > MAX_BODY {
        return Err("Product copy retry review exceeds 1 MiB".into());
    }
    validate_id(expected_batch, "expected batch")?;
    validate_id(expected_request, "expected retry review request")?;
    let review: RetryReviewWire =
        serde_json::from_slice(bytes).map_err(|_| "Invalid product copy retry review")?;
    if review.batch_id != expected_batch
        || review.request_id != expected_request
        || review.batch_id.0 == 0
        || review.request_id.0 == 0
    {
        return Err("Product copy retry review identity mismatch".into());
    }
    let selection = serde_json::to_vec(&review.selection)
        .map_err(|_| "Invalid product copy retry selection")?;
    let credential = remote_selection::decode_credential(&selection, now)
        .map_err(|_| "Invalid product copy retry selection".to_owned())?;
    if credential.total == 0 || credential.total > MAX_TOTAL {
        return Err("Invalid product copy retry selection count".into());
    }
    Ok(RetryReview {
        batch_id: review.batch_id,
        request_id: review.request_id,
        selection: credential,
    })
}

pub(crate) fn encode_request(
    tenant: RowId,
    batch: RowId,
    action: &str,
) -> Result<HttpRequest, String> {
    validate_id(tenant, "tenant")?;
    validate_id(batch, "batch")?;
    let base = format!("{}/{}", collection_path(tenant), batch.to_uuid());
    let (path, json) = match action {
        "status" => (base, None),
        "advance" => (
            format!("{base}/advance"),
            Some(
                serde_json::to_vec(&serde_json::json!({"max_items": ADVANCE_ITEMS}))
                    .map_err(|_| "Cannot encode copy batch advance")?,
            ),
        ),
        "cancel" => (format!("{base}/cancel"), Some(Vec::new())),
        _ => return Err("Unknown product copy batch action".into()),
    };
    Ok(HttpRequest {
        path,
        query: vec![],
        json,
        bearer: None,
    })
}

pub(crate) fn decode_status(bytes: &[u8], expected_batch: RowId) -> Result<BatchStatus, String> {
    if bytes.len() > MAX_BODY {
        return Err("Product copy batch response exceeds 1 MiB".into());
    }
    validate_id(expected_batch, "expected batch")?;
    let status: BatchStatus =
        serde_json::from_slice(bytes).map_err(|_| "Invalid product copy batch status")?;
    validate_status(&status, Some(expected_batch))?;
    Ok(status)
}

pub(crate) fn decode_list(bytes: &[u8]) -> Result<Vec<BatchStatus>, String> {
    if bytes.len() > MAX_BODY {
        return Err("Product copy batch response exceeds 1 MiB".into());
    }
    let statuses: Vec<BatchStatus> =
        serde_json::from_slice(bytes).map_err(|_| "Invalid product copy batch list")?;
    if statuses.len() > MAX_BATCHES {
        return Err("Product copy batch list exceeds 20 items".into());
    }
    let mut batches = BTreeSet::new();
    for status in &statuses {
        validate_status(status, None)?;
        if !batches.insert(status.batch_id) {
            return Err("Product copy batch list contains duplicate IDs".into());
        }
    }
    Ok(statuses)
}

fn validate_status(status: &BatchStatus, expected: Option<RowId>) -> Result<(), String> {
    validate_id(status.batch_id, "response batch")?;
    if expected.is_some_and(|batch| batch != status.batch_id)
        || !(1..=MAX_TOTAL).contains(&status.total)
    {
        return Err("Product copy batch identity or total mismatch".into());
    }
    let counted = status
        .applied
        .checked_add(status.rejected)
        .and_then(|value| value.checked_add(status.cancelled))
        .and_then(|value| value.checked_add(status.pending))
        .and_then(|value| value.checked_add(status.unknown))
        .ok_or("Product copy batch counts overflow")?;
    if counted != status.total {
        return Err("Product copy batch counts do not match total".into());
    }
    Ok(())
}

fn validate_id(id: RowId, name: &str) -> Result<(), String> {
    if id.0 == 0 {
        return Err(format!("Invalid {name} ID"));
    }
    Ok(())
}

fn collection_path(tenant: RowId) -> String {
    format!("/v1/tenants/{}/products/copy-batches", tenant.to_uuid())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(batch_id: RowId) -> BatchStatus {
        BatchStatus {
            batch_id,
            total: 10,
            applied: 3,
            rejected: 1,
            cancelled: 0,
            pending: 5,
            unknown: 1,
            cancel_requested: false,
        }
    }

    #[test]
    fn routes_and_bodies_are_exact_and_tenant_scoped() {
        let tenant = RowId(1);
        let batch = RowId(2);
        let token = RowId(3);
        let start = encode_start(
            tenant,
            &StartBody {
                batch_id: batch,
                token,
                excluded: vec![RowId(4), RowId(5)],
                confirmed_count: 8,
            },
        )
        .unwrap();
        assert_eq!(
            start.path,
            format!("/v1/tenants/{}/products/copy-batches", tenant.to_uuid())
        );
        assert!(start.query.is_empty());
        assert!(start.bearer.is_none());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&start.json.unwrap()).unwrap(),
            serde_json::json!({
                "batch_id": batch.to_uuid(),
                "token": token.to_uuid(),
                "excluded": [RowId(4).to_uuid(), RowId(5).to_uuid()],
                "confirmed_count": 8
            })
        );

        let list = encode_list(tenant).unwrap();
        assert_eq!(list.path, start_path(tenant));
        assert!(list.json.is_none());
        let current = encode_request(tenant, batch, "status").unwrap();
        assert_eq!(
            current.path,
            format!("{}/{}", start_path(tenant), batch.to_uuid())
        );
        assert!(current.json.is_none());
        let advance = encode_request(tenant, batch, "advance").unwrap();
        assert_eq!(
            advance.path,
            format!("/{}/advance", current.path.trim_start_matches('/'))
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&advance.json.unwrap()).unwrap(),
            serde_json::json!({"max_items": 25})
        );
        let cancel = encode_request(tenant, batch, "cancel").unwrap();
        assert_eq!(
            cancel.path,
            format!("/{}/cancel", current.path.trim_start_matches('/'))
        );
        assert_eq!(cancel.json, Some(Vec::new()));
        assert!(encode_request(tenant, batch, "delete").is_err());
    }

    #[test]
    fn start_rejects_unscoped_or_ambiguous_identities_and_counts() {
        let valid = StartBody {
            batch_id: RowId(2),
            token: RowId(3),
            excluded: vec![],
            confirmed_count: 1,
        };
        assert!(encode_start(RowId(0), &valid).is_err());
        assert!(
            encode_start(
                RowId(1),
                &StartBody {
                    batch_id: RowId(0),
                    ..valid.clone()
                }
            )
            .is_err()
        );
        assert!(
            encode_start(
                RowId(1),
                &StartBody {
                    token: RowId(0),
                    ..valid.clone()
                }
            )
            .is_err()
        );
        assert!(
            encode_start(
                RowId(1),
                &StartBody {
                    confirmed_count: 0,
                    ..valid.clone()
                }
            )
            .is_err()
        );
        assert!(
            encode_start(
                RowId(1),
                &StartBody {
                    confirmed_count: 100_001,
                    ..valid.clone()
                }
            )
            .is_err()
        );
        assert!(
            encode_start(
                RowId(1),
                &StartBody {
                    excluded: (1..=10_001).map(RowId).collect(),
                    ..valid.clone()
                }
            )
            .is_err()
        );
        assert!(
            encode_start(
                RowId(1),
                &StartBody {
                    excluded: vec![RowId(4), RowId(4)],
                    ..valid
                }
            )
            .is_err()
        );
    }

    #[test]
    fn status_rejects_mismatched_ids_malformed_counts_and_malicious_payloads() {
        let batch = RowId(7);
        let valid = status(batch);
        let encoded = serde_json::to_vec(&valid).unwrap();
        assert_eq!(decode_status(&encoded, batch).unwrap(), valid);
        assert!(decode_status(&encoded, RowId(8)).is_err());

        let mut malformed = status(batch);
        malformed.pending = 4;
        assert!(decode_status(&serde_json::to_vec(&malformed).unwrap(), batch).is_err());
        let mut overflow = status(batch);
        overflow.total = MAX_TOTAL;
        overflow.applied = u64::MAX;
        assert!(decode_status(&serde_json::to_vec(&overflow).unwrap(), batch).is_err());
        assert!(decode_status(&vec![b' '; MAX_BODY + 1], batch).is_err());
        assert!(decode_status(br#"{"batch_id":"not-a-uuid"}"#, batch).is_err());
    }

    #[test]
    fn list_is_bounded_unique_and_validates_every_status() {
        let statuses = vec![status(RowId(1)), status(RowId(2))];
        let encoded = serde_json::to_vec(&statuses).unwrap();
        assert_eq!(decode_list(&encoded).unwrap(), statuses);
        assert!(decode_list(&serde_json::to_vec(&vec![status(RowId(1)); 21]).unwrap()).is_err());
        assert!(
            decode_list(&serde_json::to_vec(&vec![status(RowId(1)), status(RowId(1))]).unwrap())
                .is_err()
        );
        let mut zero = status(RowId(0));
        zero.pending = 0;
        assert!(decode_list(&serde_json::to_vec(&[zero]).unwrap()).is_err());
        let mut complete = status(RowId(9));
        complete.pending = 0;
        complete.unknown = 0;
        complete.applied = 9;
        assert!(complete.terminal());
        assert!(!status(RowId(9)).terminal());
    }

    fn start_path(tenant: RowId) -> String {
        format!("/v1/tenants/{}/products/copy-batches", tenant.to_uuid())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BatchItem {
    pub ordinal: u64,
    pub source: RowId,
    pub operation_id: RowId,
    pub state: String,
    pub receipt: Option<erp_grid_core::actions::ActionReceipt>,
}
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BatchResults {
    pub batch_id: RowId,
    pub items: Vec<BatchItem>,
    pub next_after: Option<u64>,
}
pub(crate) fn encode_results(
    tenant: RowId,
    batch: RowId,
    after: Option<u64>,
    failed_only: bool,
) -> Result<HttpRequest, String> {
    validate_id(tenant, "tenant")?;
    validate_id(batch, "batch")?;
    if after.is_some_and(|after| after >= MAX_TOTAL) {
        return Err("Invalid result cursor".into());
    }
    let mut query = vec![
        ("limit".into(), "100".into()),
        ("failed_only".into(), failed_only.to_string()),
    ];
    if let Some(after) = after {
        query.push(("after".into(), after.to_string()));
    }
    Ok(HttpRequest {
        path: format!("{}/{}/results", collection_path(tenant), batch.to_uuid()),
        query,
        json: None,
        bearer: None,
    })
}
pub(crate) fn decode_results(
    bytes: &[u8],
    batch: RowId,
    after: Option<u64>,
    failed_only: bool,
) -> Result<BatchResults, String> {
    use erp_grid_core::actions::ActionOutcome;
    if bytes.len() > MAX_BODY {
        return Err("Batch results exceed 1 MiB".into());
    }
    let result: BatchResults =
        serde_json::from_slice(bytes).map_err(|_| "Invalid batch results")?;
    if batch.0 == 0 || result.batch_id != batch || result.items.len() > 100 {
        return Err("Batch results identity or size mismatch".into());
    }
    let mut cursor = after;
    let mut ids = BTreeSet::new();
    for item in &result.items {
        let state_valid = match (&*item.state, &item.receipt) {
            ("APPLIED", Some(receipt)) => matches!(receipt.outcome, ActionOutcome::Applied { .. }),
            ("REJECTED", Some(receipt)) => {
                matches!(receipt.outcome, ActionOutcome::Rejected { .. })
            }
            ("PENDING" | "IN_FLIGHT" | "CANCELLED", None) => true,
            _ => false,
        };
        if item.ordinal >= MAX_TOTAL
            || cursor.is_some_and(|cursor| item.ordinal <= cursor)
            || item.source.0 == 0
            || item.operation_id.0 == 0
            || !ids.insert(item.operation_id)
            || !state_valid
            || (failed_only && item.state != "REJECTED")
            || item.receipt.as_ref().is_some_and(|receipt| {
                receipt.operation_id != item.operation_id || receipt.source != item.source
            })
        {
            return Err("Invalid batch result item".into());
        }
        cursor = Some(item.ordinal);
    }
    if result.next_after.is_some() && (result.items.len() != 100 || result.next_after != cursor) {
        return Err("Invalid batch result continuation".into());
    }
    Ok(result)
}

#[cfg(test)]
mod result_tests {
    use super::*;
    #[test]
    fn copy_batch_results_reject_wrong_scope_cursor_and_forged_receipt() {
        let batch = RowId(5);
        let item = serde_json::json!({"ordinal":3,"source":RowId(10),"operation_id":RowId(20),"state":"PENDING","receipt":null});
        let valid = serde_json::json!({"batch_id":batch,"items":[item],"next_after":null});
        let encode = |value: &serde_json::Value| serde_json::to_vec(value).unwrap();
        assert_eq!(
            decode_results(&encode(&valid), batch, Some(2), false)
                .unwrap()
                .items
                .len(),
            1
        );
        assert!(decode_results(&encode(&valid), RowId(6), None, false).is_err());
        assert!(decode_results(&encode(&valid), batch, Some(3), false).is_err());
        assert!(decode_results(&encode(&valid), batch, None, true).is_err());
        let mut malformed = valid.clone();
        malformed["next_after"] = serde_json::json!(3);
        assert!(decode_results(&encode(&malformed), batch, None, false).is_err());
        let mut forged = valid.clone();
        forged["items"][0]["state"] = serde_json::json!("REJECTED");
        forged["items"][0]["receipt"] = serde_json::json!({"operation_id":RowId(99),"source":RowId(10),"command_fingerprint":"fixture","outcome":{"Rejected":{"code":"FAIL","message":"failed"}}});
        assert!(decode_results(&encode(&forged), batch, None, false).is_err());
        let request = encode_results(RowId(1), batch, Some(3), true).unwrap();
        assert!(request.json.is_none());
        assert!(request.path.ends_with("/results"));
        assert!(request.query.contains(&("after".into(), "3".into())));
    }
}

#[cfg(test)]
mod retry_review_tests {
    use super::*;

    fn response(batch: RowId, request: RowId) -> serde_json::Value {
        serde_json::json!({
            "batch_id": batch,
            "request_id": request,
            "selection": {
                "token": RowId(9),
                "query_fingerprint": "0123456789abcdef0123456789abcdef",
                "total": 3,
                "expires_at": 700
            }
        })
    }

    #[test]
    fn retry_review_route_body_and_reusable_selection_are_identity_bound() {
        let tenant = RowId(1);
        let batch = RowId(2);
        let request_id = RowId(3);
        let request = encode_retry_review(tenant, batch, request_id).unwrap();
        assert_eq!(
            request.path,
            format!(
                "/v1/tenants/{}/products/copy-batches/{}/retry-review",
                tenant.to_uuid(),
                batch.to_uuid()
            )
        );
        assert!(request.query.is_empty());
        assert!(request.bearer.is_none());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&request.json.unwrap()).unwrap(),
            serde_json::json!({"request_id": request_id})
        );

        let bytes = serde_json::to_vec(&response(batch, request_id)).unwrap();
        let review = decode_retry_review(&bytes, batch, request_id, 100).unwrap();
        assert_eq!((review.batch_id, review.request_id), (batch, request_id));
        assert_eq!(review.selection.token, RowId(9));
        assert_eq!(review.selection.total, 3);
        assert_eq!(review.selection.expires_at, 700);
    }

    #[test]
    fn retry_review_rejects_forged_expired_unbounded_and_oversized_responses() {
        let batch = RowId(2);
        let request_id = RowId(3);
        let valid = response(batch, request_id);
        let encode = |value: &serde_json::Value| serde_json::to_vec(value).unwrap();
        assert!(decode_retry_review(&encode(&valid), RowId(4), request_id, 100).is_err());
        assert!(decode_retry_review(&encode(&valid), batch, RowId(4), 100).is_err());

        let mut forged = valid.clone();
        forged["batch_id"] = serde_json::json!(RowId(4));
        assert!(decode_retry_review(&encode(&forged), batch, request_id, 100).is_err());
        forged = valid.clone();
        forged["request_id"] = serde_json::json!(RowId(4));
        assert!(decode_retry_review(&encode(&forged), batch, request_id, 100).is_err());
        forged = valid.clone();
        forged["selection"]["total"] = serde_json::json!(0);
        assert!(decode_retry_review(&encode(&forged), batch, request_id, 100).is_err());
        forged["selection"]["total"] = serde_json::json!(100_001);
        assert!(decode_retry_review(&encode(&forged), batch, request_id, 100).is_err());
        forged = valid.clone();
        forged["selection"]["expires_at"] = serde_json::json!(100);
        assert!(decode_retry_review(&encode(&forged), batch, request_id, 100).is_err());
        forged = valid;
        forged["extra"] = serde_json::json!(true);
        assert!(decode_retry_review(&encode(&forged), batch, request_id, 100).is_err());
        assert!(decode_retry_review(&vec![b' '; MAX_BODY + 1], batch, request_id, 100).is_err());

        assert!(encode_retry_review(RowId(0), batch, request_id).is_err());
        assert!(encode_retry_review(RowId(1), RowId(0), request_id).is_err());
        assert!(encode_retry_review(RowId(1), batch, RowId(0)).is_err());
    }
}
