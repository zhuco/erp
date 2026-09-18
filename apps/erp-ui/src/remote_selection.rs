//! Wire adapter for server-frozen product selections.
//!
//! This module only validates and translates selection payloads.  It does not
//! perform HTTP requests or make authorization decisions on behalf of the API.
use crate::remote_products::{self, ProductHttpQuery};
use erp_grid_core::{RowId, remote::*};
use std::collections::BTreeSet;

const MAX_TOTAL: u64 = 100_000;
const MAX_EXCLUDED: usize = 10_000;
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credential {
    pub token: RowId,
    pub query_fingerprint: String,
    pub total: u64,
    pub expires_at: u64,
}

#[derive(serde::Deserialize)]
struct CredentialWire {
    token: uuid::Uuid,
    query_fingerprint: String,
    total: u64,
    expires_at: u64,
}

#[derive(serde::Deserialize)]
struct PageWire {
    data: Vec<serde_json::Value>,
    total: u64,
    token: uuid::Uuid,
    expires_at: u64,
}

pub fn encode_freeze(tenant: RowId, request: &PageRequest) -> Result<ProductHttpQuery, String> {
    let mut encoded = remote_products::encode_query(tenant, request)?;
    encoded.path = format!("/v1/tenants/{}/products/selections", tenant.to_uuid());
    Ok(encoded)
}

pub fn decode_credential(bytes: &[u8], now: u64) -> Result<Credential, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Selection response exceeds 8 MiB".into());
    }
    let wire: CredentialWire =
        serde_json::from_slice(bytes).map_err(|_| "Invalid selection credential")?;
    if wire.token.is_nil()
        || wire.query_fingerprint.len() != 32
        || !wire
            .query_fingerprint
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
        || wire.total > MAX_TOTAL
        || wire.expires_at <= now
        || wire.expires_at > now.checked_add(601).ok_or("Selection clock overflow")?
    {
        return Err("Invalid selection credential".into());
    }
    Ok(Credential {
        token: RowId(wire.token.as_u128()),
        query_fingerprint: wire.query_fingerprint,
        total: wire.total,
        expires_at: wire.expires_at,
    })
}

pub fn encode_page(
    tenant: RowId,
    credential: &Credential,
    offset: u64,
    limit: u16,
    sorts: &[erp_grid_core::SortRule],
) -> Result<ProductHttpQuery, String> {
    let mut columns = BTreeSet::new();
    if sorts.len() > 8
        || sorts
            .iter()
            .any(|sort| sort.column.0 > 6 || !columns.insert(sort.column))
    {
        return Err("Invalid frozen product sort".into());
    }
    if credential.token.0 == 0 || !(1..=500).contains(&limit) {
        return Err("Invalid selection page range".into());
    }
    if offset > i64::MAX as u64 - u64::from(limit) {
        return Err("Invalid selection page range".into());
    }
    Ok(ProductHttpQuery {
        path: format!(
            "/v1/tenants/{}/products/selections/{}/page",
            tenant.to_uuid(),
            credential.token.to_uuid()
        ),
        body: serde_json::to_vec(&serde_json::json!({
            "offset": offset,
            "limit": limit,
            "sorts": sorts,
        }))
        .map_err(|_| "Cannot encode selection page")?,
    })
}

/// Private fill pages must use the same total ordering as the live query.
pub fn encode_query_order_page(
    tenant: RowId,
    credential: &Credential,
    offset: u64,
    limit: u16,
    sorts: &[erp_grid_core::SortRule],
) -> Result<ProductHttpQuery, String> {
    let mut encoded = encode_page(tenant, credential, offset, limit, sorts)?;
    let mut body: serde_json::Value =
        serde_json::from_slice(&encoded.body).map_err(|_| "Cannot encode selection page")?;
    body["query_order"] = serde_json::Value::Bool(true);
    encoded.body = serde_json::to_vec(&body).map_err(|_| "Cannot encode selection page")?;
    Ok(encoded)
}

pub fn decode_query_order_page(
    tenant: RowId,
    credential: &Credential,
    request: &PageRequest,
    bytes: &[u8],
    now: u64,
) -> Result<RemotePage, String> {
    decode_ordered_page(tenant, credential, request, bytes, now, true)
}

pub fn decode_page(
    tenant: RowId,
    credential: &Credential,
    request: &PageRequest,
    bytes: &[u8],
    now: u64,
) -> Result<RemotePage, String> {
    decode_ordered_page(tenant, credential, request, bytes, now, false)
}

fn decode_ordered_page(
    tenant: RowId,
    credential: &Credential,
    request: &PageRequest,
    bytes: &[u8],
    now: u64,
    query_order: bool,
) -> Result<RemotePage, String> {
    if bytes.len() > MAX_BYTES || now >= credential.expires_at {
        return Err("Selection credential is expired or response is too large".into());
    }
    let PageAddress::Offset { start, limit } = &request.address else {
        return Err("Selection page requires offset paging".into());
    };
    if !(1..=500).contains(limit) || start.checked_add(u64::from(*limit)).is_none() {
        return Err("Invalid selection page range".into());
    }
    let expected = credential
        .total
        .saturating_sub(*start)
        .min(u64::from(*limit));
    let wire: PageWire = serde_json::from_slice(bytes).map_err(|_| "Invalid selection page")?;
    if wire.token.as_u128() != credential.token.0
        || wire.total != credential.total
        || wire.expires_at != credential.expires_at
        || wire.data.len() as u64 != expected
    {
        return Err("Selection page credential or row count mismatch".into());
    }
    let page = remote_products::decode_page(tenant, request, bytes)?;
    let unique: BTreeSet<_> = page.rows.iter().map(|row| row.id).collect();
    if page.rows.iter().any(|row| row.id.0 == 0)
        || unique.len() != page.rows.len()
        || (!query_order
            && request.query.sorts.is_empty()
            && page.rows.windows(2).any(|rows| rows[0].id >= rows[1].id))
    {
        return Err("Selection page rows are not unique and ordered".into());
    }
    Ok(RemotePage {
        snapshot: Some(credential.token.to_uuid()),
        ..page
    })
}

pub fn encode_count(
    tenant: RowId,
    credential: &Credential,
    excluded: &[RowId],
) -> Result<ProductHttpQuery, String> {
    if credential.token.0 == 0 || excluded.len() > MAX_EXCLUDED {
        return Err("Invalid selection exclusions".into());
    }
    let mut unique = BTreeSet::new();
    for id in excluded {
        if id.0 == 0 || !unique.insert(*id) {
            return Err("Exclusions must be unique, non-nil product IDs".into());
        }
    }
    Ok(ProductHttpQuery {
        path: format!(
            "/v1/tenants/{}/products/selections/{}/count",
            tenant.to_uuid(),
            credential.token.to_uuid()
        ),
        body: serde_json::to_vec(&serde_json::json!({
            "excluded": excluded.iter().map(|id| id.to_uuid()).collect::<Vec<_>>(),
        }))
        .map_err(|_| "Cannot encode selection count")?,
    })
}

pub fn decode_count(bytes: &[u8], total: u64, excluded_len: usize) -> Result<u64, String> {
    if bytes.len() > MAX_BYTES || excluded_len as u64 > total {
        return Err("Invalid selection count".into());
    }
    #[derive(serde::Deserialize)]
    struct CountWire {
        affected: u64,
    }
    let wire: CountWire = serde_json::from_slice(bytes).map_err(|_| "Invalid selection count")?;
    let expected = total - excluded_len as u64;
    if wire.affected != expected {
        return Err("Selection count mismatch".into());
    }
    Ok(wire.affected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::remote::{DataScope, PageAddress, RemoteQuery};
    use std::collections::BTreeMap;

    fn credential() -> Credential {
        Credential {
            token: RowId(7),
            query_fingerprint: "0123456789abcdef0123456789abcdef".into(),
            total: 2,
            expires_at: 100,
        }
    }
    fn request(tenant: RowId, start: u64, limit: u16) -> PageRequest {
        PageRequest {
            ticket: 1,
            generation: 1,
            query: RemoteQuery {
                expression: None,
                scope: DataScope {
                    source: format!("products:{}", tenant.to_uuid()),
                    schema: 1,
                    authorization: "test".into(),
                },
                filters: BTreeMap::new(),
                sorts: vec![],
                search: None,
            },
            address: PageAddress::Offset { start, limit },
        }
    }
    fn page_json(token: RowId, expires_at: u64, rows: Vec<serde_json::Value>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "token": token.to_uuid(), "expires_at": expires_at, "total": 2,
            "data": rows,
        }))
        .unwrap()
    }
    fn row(tenant: RowId, product: RowId) -> serde_json::Value {
        serde_json::json!({
            "product_id": product.to_uuid(), "tenant_id": tenant.to_uuid(),
            "name": "p", "status": "ACTIVE", "created_at": "t", "updated_at": "t"
        })
    }

    #[test]
    fn query_order_pages_preserve_identity_checks_without_assuming_ascending_ids() {
        let tenant = RowId(1);
        let c = credential();
        let request = request(tenant, 0, 2);
        let encoded = encode_query_order_page(tenant, &c, 0, 2, &[]).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&encoded.body).unwrap();
        assert_eq!(body["query_order"], true);
        let data = page_json(
            c.token,
            c.expires_at,
            vec![row(tenant, RowId(3)), row(tenant, RowId(2))],
        );
        assert!(decode_query_order_page(tenant, &c, &request, &data, 1).is_ok());
        assert!(decode_page(tenant, &c, &request, &data, 1).is_err());
        let duplicate = page_json(
            c.token,
            c.expires_at,
            vec![row(tenant, RowId(3)), row(tenant, RowId(3))],
        );
        assert!(decode_query_order_page(tenant, &c, &request, &duplicate, 1).is_err());
    }

    #[test]
    fn credential_rejects_wrong_fingerprint_expiry_and_bounds() {
        let valid = serde_json::json!({"token": RowId(7).to_uuid(), "query_fingerprint": "0123456789abcdef0123456789abcdef", "total": 2, "expires_at": 100});
        assert!(decode_credential(&serde_json::to_vec(&valid).unwrap(), 100).is_err());
        assert!(decode_credential(&serde_json::to_vec(&valid).unwrap(), 0).is_ok());
        let mut wrong = valid.clone();
        wrong["query_fingerprint"] = serde_json::json!("short");
        assert!(decode_credential(&serde_json::to_vec(&wrong).unwrap(), 0).is_err());
        wrong["total"] = serde_json::json!(100_001);
        assert!(decode_credential(&serde_json::to_vec(&wrong).unwrap(), 0).is_err());
    }

    #[test]
    fn page_rejects_wrong_token_cross_tenant_and_duplicate_rows() {
        let tenant = RowId(1);
        let request = request(tenant, 0, 2);
        let c = credential();
        let rows = vec![row(tenant, RowId(2)), row(tenant, RowId(3))];
        assert!(
            decode_page(
                tenant,
                &c,
                &request,
                &page_json(RowId(8), 100, rows.clone()),
                1
            )
            .is_err()
        );
        assert!(
            decode_page(
                RowId(9),
                &c,
                &request,
                &page_json(c.token, 100, rows.clone()),
                1
            )
            .is_err()
        );
        let duplicate = vec![row(tenant, RowId(2)), row(tenant, RowId(2))];
        assert!(decode_page(tenant, &c, &request, &page_json(c.token, 100, duplicate), 1).is_err());
    }

    #[test]
    fn page_and_count_enforce_bounds_and_exact_affected_count() {
        let c = credential();
        let tenant = RowId(1);
        assert!(encode_page(tenant, &c, u64::MAX, 1, &[]).is_err());
        assert!(encode_count(tenant, &c, &[RowId(2), RowId(2)]).is_err());
        assert_eq!(decode_count(br#"{"affected":1}"#, 2, 1), Ok(1));
        assert!(decode_count(br#"{"affected":2}"#, 2, 1).is_err());
    }

    #[test]
    fn sorted_selection_pages_encode_priorities_and_still_reject_duplicate_ids() {
        let tenant = RowId(1);
        let c = credential();
        let sort = erp_grid_core::SortRule {
            column: erp_grid_core::ColumnId(1),
            descending: true,
        };
        let encoded = encode_page(tenant, &c, 0, 2, std::slice::from_ref(&sort)).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&encoded.body).unwrap();
        assert_eq!(
            body["sorts"],
            serde_json::json!([{"column":1,"descending":true}])
        );
        assert!(encode_page(tenant, &c, 0, 2, &[sort, sort]).is_err());
        let mut request = request(tenant, 0, 2);
        request.query.sorts = vec![sort];
        let reversed = vec![row(tenant, RowId(3)), row(tenant, RowId(2))];
        assert!(
            decode_page(
                tenant,
                &c,
                &request,
                &page_json(c.token, c.expires_at, reversed),
                1
            )
            .is_ok()
        );
        let duplicates = vec![row(tenant, RowId(3)), row(tenant, RowId(3))];
        assert!(
            decode_page(
                tenant,
                &c,
                &request,
                &page_json(c.token, c.expires_at, duplicates),
                1
            )
            .is_err()
        );
    }
}
