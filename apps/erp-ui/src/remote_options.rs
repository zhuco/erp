//! Product option-directory wire adapter. Transport and request lifecycle stay
//! in the host; this module only validates and encodes/decodes JSON.
use erp_grid_core::{ColumnId, OptionIdentity, OptionPage, paged_query::PageQuery};
use std::collections::BTreeSet;

const MAX_BODY: usize = 8 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct OptionHttpQuery {
    pub path: String,
    pub body: Vec<u8>,
    pub identity: OptionIdentity,
    pub limit: u32,
}

#[derive(serde::Serialize)]
struct OptionRequestBody<'a> {
    query: &'a PageQuery,
    column: ColumnId,
    search: &'a Option<String>,
    offset: u64,
    limit: u32,
}

#[derive(serde::Deserialize)]
struct OptionResponse {
    values: Vec<String>,
    total: u64,
    #[serde(default)]
    includes_empty: bool,
}

pub fn encode_options(
    identity: OptionIdentity,
    query: PageQuery,
    limit: u32,
) -> Result<OptionHttpQuery, String> {
    query.validate().map_err(str::to_owned)?;
    if query.schema != 1 || !(1..=100).contains(&limit) {
        return Err("Invalid option query range or schema".into());
    }
    if !(2..=4).contains(&identity.column.0) {
        return Err("Unsupported product option column".into());
    }
    identity.validate()?;
    let search = identity.search.clone();
    let body = serde_json::to_vec(&OptionRequestBody {
        query: &query,
        column: identity.column,
        search: &search,
        offset: identity.offset,
        limit,
    })
    .map_err(|_| "Cannot encode product options request".to_owned())?;
    if body.len() > MAX_BODY {
        return Err("Option request exceeds size limit".into());
    }
    Ok(OptionHttpQuery {
        path: format!("/v1/tenants/{}/products/options", identity.tenant.to_uuid()),
        body,
        identity,
        limit,
    })
}

pub fn decode_options(request: &OptionHttpQuery, bytes: &[u8]) -> Result<OptionPage, String> {
    if bytes.len() > MAX_BODY {
        return Err("Option response exceeds 8 MiB".into());
    }
    let response: OptionResponse =
        serde_json::from_slice(bytes).map_err(|_| "Invalid option response".to_owned())?;
    let _includes_empty = response.includes_empty;
    if response.values.len() > request.limit as usize
        || response.total < response.values.len() as u64
        || (!response.values.is_empty()
            && request.identity.offset
                > response.total.saturating_sub(response.values.len() as u64))
    {
        return Err("Option response page is invalid".into());
    }
    if response
        .values
        .iter()
        .any(|value| value.len() > 4096 || value.contains('\0'))
        || response.values.iter().collect::<BTreeSet<_>>().len() != response.values.len()
    {
        return Err("Option response contains invalid or duplicate values".into());
    }
    Ok(OptionPage {
        values: response.values,
        total: response.total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::RowId;
    fn query(offset: u64) -> PageQuery {
        PageQuery {
            expression: None,
            schema: 1,
            offset,
            limit: 20,
            filters: Default::default(),
            sorts: vec![],
            search: None,
        }
    }
    #[test]
    fn request_contains_full_scope_and_decode_is_bounded() {
        let request = encode_options(
            OptionIdentity {
                tenant: RowId(7),
                query_scope: "products".into(),
                filter_scope: "brand=all".into(),
                column: ColumnId(2),
                search: Some("acme".into()),
                offset: 20,
            },
            query(0),
            20,
        )
        .unwrap();
        assert_eq!(
            request.path,
            format!("/v1/tenants/{}/products/options", RowId(7).to_uuid())
        );
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["offset"], 20);
        assert_eq!(body["limit"], 20);
        let page = decode_options(&request, br#"{"values":["A","B"],"total":22}"#).unwrap();
        assert_eq!(page.total, 22);
        assert!(decode_options(&request, br#"{"values":["A","A"],"total":22}"#).is_err());
    }
    #[test]
    fn current_page_cannot_forge_complete_catalogue() {
        let request = encode_options(
            OptionIdentity {
                tenant: RowId(7),
                query_scope: "products".into(),
                filter_scope: "brand=all".into(),
                column: ColumnId(2),
                search: None,
                offset: 20,
            },
            query(0),
            20,
        )
        .unwrap();
        let page = decode_options(&request, br#"{"values":["A"],"total":101}"#).unwrap();
        assert_ne!(page.values.len() as u64, page.total);
    }
}
