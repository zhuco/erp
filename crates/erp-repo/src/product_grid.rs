//! Product grid schema 1: columns 0..=6 are code/name/brand/category/status/
//! created_at/updated_at. Client values are bound, never SQL identifiers.
use chrono::{DateTime, NaiveDate, Utc};
use erp_grid_core::{
    ColumnId, Filter,
    paged_query::{FilterExpression, PageQuery, civil_parts},
};
use erp_types::TenantId;
use serde::Serialize;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductGridRow {
    pub product_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub product_code: Option<String>,
    pub name: String,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ProductGridPage {
    pub data: Vec<ProductGridRow>,
    pub total: u64,
}

fn field(column: ColumnId) -> Result<&'static str, &'static str> {
    match column.0 {
        0 => Ok("product_code"),
        1 => Ok("name"),
        2 => Ok("brand"),
        3 => Ok("category"),
        4 => Ok("status"),
        5 => Ok("created_at"),
        6 => Ok("updated_at"),
        _ => Err("Unknown product column"),
    }
}

pub fn validate(query: &PageQuery) -> Result<(), &'static str> {
    query.validate()?;
    if query.schema != 1 {
        return Err("Unsupported product grid schema");
    }
    for sort in &query.sorts {
        field(sort.column)?;
    }
    for (column, filter) in &query.filters {
        validate_condition(*column, filter)?;
    }
    if let Some(expression) = &query.expression {
        validate_expression(expression)?;
    }
    Ok(())
}

fn validate_condition(column: ColumnId, filter: &Filter) -> Result<(), &'static str> {
    field(column)?;
    match (column.0, filter) {
        (0..=4, Filter::Text(_) | Filter::Enum(_)) => Ok(()),
        (5..=6, Filter::DateTime { .. }) => Ok(()),
        _ => Err("Filter type does not match product column"),
    }
}

fn validate_expression(expression: &FilterExpression) -> Result<(), &'static str> {
    expression.validate()?;
    fn walk(expression: &FilterExpression) -> Result<(), &'static str> {
        match expression {
            FilterExpression::Condition { column, filter } => validate_condition(*column, filter),
            FilterExpression::And(items) | FilterExpression::Or(items) => {
                for item in items {
                    walk(item)?;
                }
                Ok(())
            }
        }
    }
    walk(expression)
}

// Escape LIKE metacharacters so a literal SKU such as 10%_A searches literally.
fn contains_pattern(text: &str) -> String {
    format!(
        "%{}%",
        text.replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
    )
}

fn append_filter(
    qb: &mut QueryBuilder<'_, Postgres>,
    column: ColumnId,
    filter: &Filter,
) -> anyhow::Result<()> {
    let name = field(column).map_err(anyhow::Error::msg)?;
    match filter {
        Filter::Text(text) => {
            qb.push("coalesce(")
                .push(name)
                .push(", '') ilike ")
                .push_bind(contains_pattern(text))
                .push(" escape '!'");
        }
        Filter::Enum(values) => {
            qb.push("coalesce(")
                .push(name)
                .push(", '') = any(")
                .push_bind(values.iter().cloned().collect::<Vec<_>>())
                .push(")");
        }
        Filter::DateTime { start, end } => {
            qb.push("(");
            let mut has = false;
            if let Some(start) = start {
                qb.push(name).push(" >= ").push_bind(utc_time(*start)?);
                has = true;
            }
            if let Some(end) = end {
                if has {
                    qb.push(" and ");
                }
                let end = utc_time(*end)?
                    .checked_add_signed(chrono::Duration::seconds(1))
                    .ok_or_else(|| anyhow::anyhow!("Date overflow"))?;
                qb.push(name).push(" < ").push_bind(end);
                has = true;
            }
            if !has {
                qb.push("true");
            }
            qb.push(")");
        }
        _ => anyhow::bail!("Invalid product filter"),
    }
    Ok(())
}

fn append_expression(
    qb: &mut QueryBuilder<'_, Postgres>,
    expression: &FilterExpression,
) -> anyhow::Result<()> {
    match expression {
        FilterExpression::Condition { column, filter } => append_filter(qb, *column, filter),
        FilterExpression::And(items) | FilterExpression::Or(items) => {
            qb.push("(");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    qb.push(if matches!(expression, FilterExpression::And(_)) {
                        " and "
                    } else {
                        " or "
                    });
                }
                append_expression(qb, item)?;
            }
            qb.push(")");
            Ok(())
        }
    }
}

fn utc_time(key: i64) -> anyhow::Result<DateTime<Utc>> {
    let p = civil_parts(key).map_err(anyhow::Error::msg)?;
    NaiveDate::from_ymd_opt(p.year, p.month, p.day)
        .and_then(|date| date.and_hms_opt(p.hour, p.minute, p.second))
        .map(|date| date.and_utc())
        .ok_or_else(|| anyhow::anyhow!("Invalid UTC date"))
}

pub(crate) fn predicates(
    qb: &mut QueryBuilder<'_, Postgres>,
    tenant: TenantId,
    query: &PageQuery,
) -> anyhow::Result<()> {
    qb.push(" from products where tenant_id = ")
        .push_bind(uuid::Uuid::from(tenant));
    qb.push(" and status <> 'DELETED'");
    if let Some(text) = query
        .search
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        qb.push(" and (");
        for (index, name) in ["name", "product_code", "alias", "primary_barcode"]
            .iter()
            .enumerate()
        {
            if index > 0 {
                qb.push(" or ");
            }
            qb.push(name)
                .push(" ilike ")
                .push_bind(contains_pattern(text.trim()))
                .push(" escape '!'");
        }
        qb.push(")");
    }
    for (column, filter) in &query.filters {
        qb.push(" and ");
        append_filter(qb, *column, filter)?;
    }
    if let Some(expression) = &query.expression {
        qb.push(" and ");
        append_expression(qb, expression)?;
    }
    Ok(())
}

pub(crate) fn append_ordering(
    qb: &mut QueryBuilder<'_, Postgres>,
    query: &PageQuery,
) -> anyhow::Result<()> {
    qb.push(" order by ");
    if query.sorts.is_empty() {
        qb.push("created_at desc, ");
    }
    for sort in &query.sorts {
        let name = field(sort.column).map_err(anyhow::Error::msg)?;
        if sort.column.0 <= 4 {
            qb.push("lower(nullif(").push(name).push(", ''))");
        } else {
            qb.push(name);
        }
        qb.push(if sort.descending { " desc" } else { " asc" })
            .push(" nulls last, ");
    }
    qb.push("product_id desc");
    Ok(())
}

fn ordering(qb: &mut QueryBuilder<'_, Postgres>, query: &PageQuery) -> anyhow::Result<()> {
    append_ordering(qb, query)?;
    qb.push(" limit ").push_bind(i64::from(query.limit));
    qb.push(" offset ").push_bind(query.offset as i64);
    Ok(())
}

pub async fn query_products(
    pool: &PgPool,
    tenant: TenantId,
    query: &PageQuery,
) -> anyhow::Result<ProductGridPage> {
    validate(query).map_err(anyhow::Error::msg)?;
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    let mut count = QueryBuilder::<Postgres>::new("select count(*)");
    predicates(&mut count, tenant, query)?;
    let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let mut page = QueryBuilder::<Postgres>::new(
        "select product_id, tenant_id, product_code, name, brand, category, status, created_at, updated_at",
    );
    predicates(&mut page, tenant, query)?;
    ordering(&mut page, query)?;
    let data = page.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(ProductGridPage {
        data,
        total: u64::try_from(total)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::SortRule;
    use std::collections::BTreeMap;

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates connection-local temporary tables only"]
    async fn product_grid_postgres_full_query_then_page_and_tenant_isolation() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table products (product_id uuid primary key, tenant_id uuid not null, product_code text, name text not null, alias text, primary_barcode text, brand text, category text, status text not null, created_at timestamptz not null, updated_at timestamptz not null)").execute(&pool).await.unwrap();
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        sqlx::query("insert into products select lpad(to_hex(n), 32, '0')::uuid, $1, 'SKU-' || lpad(n::text, 5, '0'), 'Product ' || n, null, null, case when n % 2 = 0 then 'A' else 'B' end, 'Goods', 'ACTIVE', '2026-09-09T12:34:56.500Z'::timestamptz, '2026-09-09T12:34:56.500Z'::timestamptz from generate_series(1, 10000) n")
            .bind(uuid::Uuid::from(tenant)).execute(&pool).await.unwrap();
        // Other merchant and deleted rows must never enter counts or pages.
        sqlx::query("insert into products values ($1, $2, 'SKU-other', 'Other', null, null, 'A', 'Goods', 'ACTIVE', now(), now()), ($3, $4, 'SKU-deleted', 'Deleted', null, null, 'A', 'Goods', 'DELETED', now(), now())")
            .bind(uuid::Uuid::from_u128(10001)).bind(uuid::Uuid::from_u128(2)).bind(uuid::Uuid::from_u128(10002)).bind(uuid::Uuid::from(tenant)).execute(&pool).await.unwrap();
        let mut query = PageQuery {
            schema: 1,
            offset: 9900,
            limit: 100,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
            expression: None,
        };
        let page = query_products(&pool, tenant, &query).await.unwrap();
        assert_eq!((page.total, page.data.len()), (10000, 100));
        assert_eq!(page.data[0].product_id, uuid::Uuid::from_u128(100));
        assert_eq!(page.data[99].product_id, uuid::Uuid::from_u128(1));
        query.offset = 0;
        query
            .filters
            .insert(ColumnId(0), Filter::Text("SKU-10000".into()));
        let page = query_products(&pool, tenant, &query).await.unwrap();
        assert_eq!((page.total, page.data.len()), (1, 1));
        assert_eq!(page.data[0].product_id, uuid::Uuid::from_u128(10000));
        query.filters.clear();
        query
            .filters
            .insert(ColumnId(2), Filter::Enum(["A".into(), "B".into()].into()));
        query.filters.insert(
            ColumnId(5),
            Filter::DateTime {
                start: Some(20_260_909_123_456),
                end: Some(20_260_909_123_456),
            },
        );
        assert_eq!(
            query_products(&pool, tenant, &query).await.unwrap().total,
            10000
        );
        query.filters.clear();
        query.sorts = vec![SortRule {
            column: ColumnId(0),
            descending: false,
        }];
        assert_eq!(
            query_products(&pool, tenant, &query).await.unwrap().data[0]
                .product_code
                .as_deref(),
            Some("SKU-00001")
        );
        sqlx::query("update pg_temp.products set name = '10%_A' where product_id = $1")
            .bind(uuid::Uuid::from_u128(1))
            .execute(&pool)
            .await
            .unwrap();
        query.filters.insert(ColumnId(1), Filter::Text("%_".into()));
        assert_eq!(
            query_products(&pool, tenant, &query).await.unwrap().total,
            1
        );
        query.filters.clear();
        query.search = Some("'; drop table products; --".into());
        assert_eq!(
            query_products(&pool, tenant, &query).await.unwrap().total,
            0
        );
        query.search = None;
        query.offset = 10000;
        let page = query_products(&pool, tenant, &query).await.unwrap();
        assert_eq!(page.total, 10000);
        assert!(page.data.is_empty());
        query.offset = 0;
        query.expression = Some(FilterExpression::And(vec![
            FilterExpression::Or(vec![
                FilterExpression::Condition {
                    column: ColumnId(0),
                    filter: Filter::Enum(["SKU-00001".into()].into()),
                },
                FilterExpression::Condition {
                    column: ColumnId(0),
                    filter: Filter::Enum(["SKU-00002".into()].into()),
                },
            ]),
            FilterExpression::Condition {
                column: ColumnId(5),
                filter: Filter::DateTime {
                    start: None,
                    end: Some(20_260_909_123_456),
                },
            },
        ]));
        let page = query_products(&pool, tenant, &query).await.unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(
            page.data
                .iter()
                .map(|row| row.product_code.as_deref().unwrap())
                .collect::<Vec<_>>(),
            ["SKU-00001", "SKU-00002"]
        );
        pool.close().await;
    }

    #[test]
    fn product_grid_whitelist_bound_values_and_stable_order() {
        let mut query = PageQuery {
            schema: 1,
            offset: 9900,
            limit: 100,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: Some("'; drop table products; --".into()),
            expression: None,
        };
        query
            .filters
            .insert(ColumnId(1), Filter::Text("100%_中文!".into()));
        query.sorts.push(SortRule {
            column: ColumnId(1),
            descending: false,
        });
        assert!(validate(&query).is_ok());
        let mut qb = QueryBuilder::<Postgres>::new("select product_id");
        predicates(&mut qb, TenantId(uuid::Uuid::nil()), &query).unwrap();
        ordering(&mut qb, &query).unwrap();
        let sql = qb.sql();
        assert!(sql.contains("tenant_id = $1 and status <> 'DELETED'"));
        assert!(sql.contains("lower(nullif(name, '')) asc nulls last, product_id desc limit $"));
        assert!(!sql.contains("drop table"));
        assert!(!sql.contains("中文"));
        assert_eq!(contains_pattern("100%_中文!"), "%100!%!_中文!!%");
        query.sorts[0].column = ColumnId(999);
        assert!(validate(&query).is_err());
        query.sorts.clear();
        query
            .filters
            .insert(ColumnId(5), Filter::Text("date".into()));
        assert!(validate(&query).is_err());
        query.filters.clear();
        query.schema = 2;
        assert!(validate(&query).is_err());
    }

    #[test]
    fn product_grid_date_keys_are_utc_civil_not_epoch() {
        assert_eq!(
            utc_time(20_240_229_235_959).unwrap().to_rfc3339(),
            "2024-02-29T23:59:59+00:00"
        );
        assert!(utc_time(20_250_229_000_000).is_err());
    }

    #[test]
    fn product_date_end_only_does_not_append_spurious_true() {
        let query = PageQuery {
            schema: 1,
            offset: 0,
            limit: 10,
            filters: BTreeMap::from([(
                ColumnId(5),
                Filter::DateTime {
                    start: None,
                    end: Some(20_260_909_123_456),
                },
            )]),
            sorts: vec![],
            search: None,
            expression: None,
        };
        let mut qb = QueryBuilder::<Postgres>::new("select product_id");
        predicates(&mut qb, TenantId(uuid::Uuid::nil()), &query).unwrap();
        let sql = qb.sql();
        assert!(sql.contains("created_at < $2"));
        assert!(!sql.contains("$2true"));
    }

    #[test]
    fn product_expression_preserves_or_parentheses_and_binds_values() {
        let query = PageQuery {
            schema: 1,
            offset: 0,
            limit: 10,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
            expression: Some(FilterExpression::Or(vec![
                FilterExpression::Condition {
                    column: ColumnId(0),
                    filter: Filter::Text("A%".into()),
                },
                FilterExpression::And(vec![
                    FilterExpression::Condition {
                        column: ColumnId(2),
                        filter: Filter::Enum(["Acme".into()].into()),
                    },
                    FilterExpression::Condition {
                        column: ColumnId(4),
                        filter: Filter::Text("ACTIVE".into()),
                    },
                ]),
            ])),
        };
        assert!(validate(&query).is_ok());
        let mut qb = QueryBuilder::<Postgres>::new("select product_id");
        predicates(&mut qb, TenantId(uuid::Uuid::nil()), &query).unwrap();
        let sql = qb.sql();
        assert!(sql.contains(" and (coalesce(product_code"));
        assert!(sql.contains(" or (coalesce(brand"));
        assert!(!sql.contains("A%"));
    }
}
