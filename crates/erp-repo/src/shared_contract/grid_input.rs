//! Named inputs for the existing grid query and selection writers.

use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct GridQueryInput<'a> {
    pub page: &'a str,
    pub filter: &'a str,
    pub sort_key: &'a str,
    pub row_id: &'a str,
    pub offset: i64,
    pub limit: i32,
    pub loaded: i64,
    pub matched: i64,
    pub exact: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GridSelectionInput<'a> {
    pub query_id: Uuid,
    pub kind: &'a str,
    pub row_id: &'a str,
    pub snapshot: &'a str,
    pub excluded: i64,
    pub loaded: i64,
    pub matched: i64,
}
