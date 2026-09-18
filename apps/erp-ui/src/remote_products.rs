//! Adapter for product grid query schema 1. Unsupported semantics are rejected,
//! never simulated by filtering a fetched page or downloading the whole catalogue.
use erp_grid_core::{ColumnId, Filter, RowId, paged_query::PageQuery, remote::*};
use std::collections::BTreeMap;

pub const CODE: ColumnId = ColumnId(0);
pub const NAME: ColumnId = ColumnId(1);
pub const BRAND: ColumnId = ColumnId(2);
pub const CATEGORY: ColumnId = ColumnId(3);
pub const STATUS: ColumnId = ColumnId(4);
pub const CREATED: ColumnId = ColumnId(5);
pub const UPDATED: ColumnId = ColumnId(6);

#[derive(Debug, PartialEq, Eq)]
pub struct ProductHttpQuery {
    pub path: String,
    pub body: Vec<u8>,
}

pub fn encode_query(tenant: RowId, request: &PageRequest) -> Result<ProductHttpQuery, String> {
    if request.query.scope.source != format!("products:{}", tenant.to_uuid()) {
        return Err("Product request belongs to a different data source".into());
    }
    let PageAddress::Offset { start, limit } = &request.address else {
        return Err("Product grid API requires offset paging".into());
    };
    let query = PageQuery {
        expression: request.query.expression.as_deref().cloned(),
        schema: request.query.scope.schema,
        offset: *start,
        limit: *limit,
        filters: request.query.filters.clone(),
        sorts: request.query.sorts.clone(),
        search: request.query.search.clone(),
    };
    query.validate().map_err(str::to_owned)?;
    if query.schema != 1 || query.sorts.iter().any(|sort| sort.column.0 > UPDATED.0) {
        return Err("Unsupported product schema or sort column".into());
    }
    for (column, filter) in &request.query.filters {
        match (column.0, filter) {
            (0..=4, Filter::Text(_) | Filter::Enum(_)) | (5..=6, Filter::DateTime { .. }) => {}
            _ => return Err("Filter type does not match product column".into()),
        }
    }
    Ok(ProductHttpQuery {
        path: format!("/v1/tenants/{}/products/query", tenant.to_uuid()),
        body: serde_json::to_vec(&query).map_err(|_| "Cannot encode product query")?,
    })
}

#[derive(serde::Deserialize)]
struct ProductList {
    data: Vec<ProductRow>,
    total: u64,
}
#[derive(serde::Deserialize)]
struct ProductRow {
    product_id: RowId,
    tenant_id: RowId,
    product_code: Option<String>,
    name: String,
    brand: Option<String>,
    category: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

pub fn decode_page(
    tenant: RowId,
    request: &PageRequest,
    bytes: &[u8],
) -> Result<RemotePage, String> {
    encode_query(tenant, request)?;
    if bytes.len() > RemotePages::MAX_BYTES {
        return Err("Product response exceeds 8 MiB".into());
    }
    let response: ProductList =
        serde_json::from_slice(bytes).map_err(|_| "Invalid product response")?;
    if response.data.len() > request.address.limit() {
        return Err("Product response exceeds requested page size".into());
    }
    let mut rows = Vec::with_capacity(response.data.len());
    for row in response.data {
        if row.tenant_id != tenant {
            return Err("Product response merchant mismatch".into());
        }
        rows.push(RemoteRow {
            id: row.product_id,
            // Used by products.update as the expected database revision.
            version: Some(row.updated_at.clone()),
            cells: BTreeMap::from([
                (CODE, row.product_code.unwrap_or_default()),
                (NAME, row.name),
                (BRAND, row.brand.unwrap_or_default()),
                (CATEGORY, row.category.unwrap_or_default()),
                (STATUS, row.status),
                (CREATED, row.created_at),
                (UPDATED, row.updated_at),
            ]),
        });
    }
    Ok(RemotePage {
        rows,
        total: RowCount::Exact(response.total),
        next_cursor: None,
        snapshot: None,
    })
}

pub fn decode_product(tenant: RowId, product: RowId, bytes: &[u8]) -> Result<RemoteRow, String> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Product response exceeds 8 MiB".into());
    }
    let row: ProductRow = serde_json::from_slice(bytes).map_err(|_| "Invalid product response")?;
    if row.tenant_id != tenant || row.product_id != product || row.status == "DELETED" {
        return Err("Product identity or merchant mismatch, or product was deleted".into());
    }
    if row.updated_at.is_empty() || row.updated_at.len() > 128 {
        return Err("Product version is invalid".into());
    }
    Ok(RemoteRow {
        id: row.product_id,
        version: Some(row.updated_at.clone()),
        cells: BTreeMap::from([
            (CODE, row.product_code.unwrap_or_default()),
            (NAME, row.name),
            (BRAND, row.brand.unwrap_or_default()),
            (CATEGORY, row.category.unwrap_or_default()),
            (STATUS, row.status),
            (CREATED, row.created_at),
            (UPDATED, row.updated_at),
        ]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::SortRule;
    fn request() -> (RowId, PageRequest) {
        let tenant = RowId(1);
        (
            tenant,
            PageRequest {
                ticket: 1,
                generation: 1,
                query: RemoteQuery {
                    expression: None,
                    scope: DataScope {
                        source: format!("products:{}", tenant.to_uuid()),
                        schema: 1,
                        authorization: "merchant-1:user-1:permission-v1".into(),
                    },
                    filters: BTreeMap::new(),
                    sorts: vec![],
                    search: None,
                },
                address: PageAddress::Offset {
                    start: 0,
                    limit: 100,
                },
            },
        )
    }
    #[test]
    fn product_adapter_does_not_change_requested_filter_or_sort_semantics() {
        let (tenant, mut request) = request();
        request.query.expression = Some(Box::new(
            erp_grid_core::paged_query::FilterExpression::Or(vec![
                erp_grid_core::paged_query::FilterExpression::Condition {
                    column: NAME,
                    filter: Filter::Text("A".into()),
                },
                erp_grid_core::paged_query::FilterExpression::Condition {
                    column: NAME,
                    filter: Filter::Text("B".into()),
                },
            ]),
        ));
        request
            .query
            .filters
            .insert(BRAND, Filter::Enum(["A&B 中文".into()].into()));
        request
            .query
            .filters
            .insert(NAME, Filter::Text("shirt".into()));
        request.query.sorts.push(SortRule {
            column: NAME,
            descending: false,
        });
        request.address = PageAddress::Offset {
            start: 100,
            limit: 100,
        };
        let encoded = encode_query(tenant, &request).unwrap();
        let decoded: PageQuery = serde_json::from_slice(&encoded.body).unwrap();
        assert_eq!(decoded.filters, request.query.filters);
        assert_eq!(
            decoded.expression.as_ref(),
            request.query.expression.as_deref()
        );
        assert_eq!(decoded.sorts, request.query.sorts);
        assert_eq!((decoded.offset, decoded.limit), (100, 100));
        assert!(encoded.path.ends_with("/products/query"));
        request
            .query
            .filters
            .insert(CREATED, Filter::Text("2026".into()));
        assert!(encode_query(tenant, &request).is_err());
    }
    #[test]
    fn product_response_preserves_uuid_and_exact_count_and_rejects_other_merchant() {
        let (tenant, request) = request();
        let product = RowId(u128::MAX - 1);
        let mut body = serde_json::json!({"data": [{"product_id": product.to_uuid(), "tenant_id": tenant.to_uuid(), "name": "Long ID", "status": "ACTIVE", "created_at": "2026-09-09T00:00:00Z", "updated_at": "2026-09-09T00:00:00Z"}], "total": 1});
        let page = decode_page(tenant, &request, &serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(page.rows[0].id, product);
        assert_eq!(page.total, RowCount::Exact(1));
        assert!(page.snapshot.is_none());
        let text = serde_json::to_string(&product).unwrap();
        assert!(text.starts_with('"'));
        assert_eq!(serde_json::from_str::<RowId>(&text).unwrap(), product);
        body["data"][0]["tenant_id"] = serde_json::json!(RowId(2).to_uuid());
        assert!(decode_page(tenant, &request, &serde_json::to_vec(&body).unwrap()).is_err());
    }
}
