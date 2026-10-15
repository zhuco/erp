//! Bounded SKU child-grid query.
//!
//! The parent product and tenant are part of the SQL predicate, rather than
//! being left to the caller to filter after the query.  Column names come only
//! from the fixed schema below; values are always bound parameters.
use chrono::{DateTime, NaiveDate, Utc};
use erp_grid_core::{
    ColumnId, Filter,
    paged_query::{FilterExpression, PageQuery, civil_parts},
};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SkuGridRow {
    pub sku_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub product_id: uuid::Uuid,
    pub sku_code: String,
    pub barcode: Option<String>,
    pub name: Option<String>,
    pub sku_kind: String,
    pub cost_cent: Option<i64>,
    pub is_active: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SkuGridPage {
    pub data: Vec<SkuGridRow>,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkuGridRange {
    pub offset: u64,
    pub limit: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkuGridRangesRequest {
    pub query: PageQuery,
    pub ranges: Vec<SkuGridRange>,
}

#[derive(Debug, Serialize)]
pub struct SkuGridRangePage {
    pub offset: u64,
    pub data: Vec<SkuGridRow>,
}

#[derive(Debug, Serialize)]
pub struct SkuGridRangesPage {
    pub total: u64,
    pub pages: Vec<SkuGridRangePage>,
}

// Schema 1: code, barcode, name, kind, cost, active, status, created, updated.
fn field(column: ColumnId) -> Result<&'static str, &'static str> {
    match column.0 {
        0 => Ok("sku_code"),
        1 => Ok("barcode"),
        2 => Ok("name"),
        3 => Ok("sku_kind"),
        4 => Ok("cost_cent"),
        5 => Ok("is_active"),
        6 => Ok("status"),
        7 => Ok("created_at"),
        8 => Ok("updated_at"),
        _ => Err("Unknown SKU column"),
    }
}

// KIT costs are derived from the current BOM, matching `SkusRepo::get_sku`
// and `list_skus`; the stored value is only authoritative for ordinary SKUs.
pub(crate) fn cost_expression() -> &'static str {
    "case when s.sku_kind = 'KIT' then (select coalesce(sum(b.qty * coalesce(c.cost_cent, 0)), 0) from sku_boms b join skus c on c.sku_id = b.component_sku_id and c.tenant_id = s.tenant_id where b.tenant_id = s.tenant_id and b.bundle_sku_id = s.sku_id) else s.cost_cent end::bigint"
}

pub fn validate(query: &PageQuery) -> Result<(), &'static str> {
    query.validate()?;
    if query.schema != 1 {
        return Err("Unsupported SKU grid schema");
    }
    for sort in &query.sorts {
        field(sort.column)?;
    }
    for (column, filter) in &query.filters {
        field(*column)?;
        match (column.0, filter) {
            (0..=3 | 6, Filter::Text(_) | Filter::Enum(_)) => {}
            (4, Filter::Money { .. }) => {}
            (5, Filter::Enum(values)) if values.iter().all(|v| v == "true" || v == "false") => {}
            (7..=8, Filter::DateTime { .. }) => {}
            _ => return Err("Filter type does not match SKU column"),
        }
    }
    if let Some(expression) = &query.expression {
        validate_expression(expression)?;
    }
    Ok(())
}

pub fn validate_ranges(request: &SkuGridRangesRequest) -> Result<(), &'static str> {
    validate(&request.query)?;
    if request.ranges.is_empty() || request.ranges.len() > 2 {
        return Err("One or two SKU ranges are required");
    }
    let mut total_limit = 0_u64;
    for range in &request.ranges {
        if !(1..=1000).contains(&range.limit) {
            return Err("Invalid SKU range limit");
        }
        let limit = u64::from(range.limit);
        if range.offset > i64::MAX as u64 - limit {
            return Err("Invalid SKU range offset");
        }
        total_limit = total_limit
            .checked_add(limit)
            .ok_or("SKU range total overflow")?;
    }
    if total_limit > 2000 {
        return Err("SKU ranges exceed row limit");
    }
    Ok(())
}

fn validate_condition(column: ColumnId, filter: &Filter) -> Result<(), &'static str> {
    field(column)?;
    match (column.0, filter) {
        (0..=3 | 6, Filter::Text(_) | Filter::Enum(_)) => Ok(()),
        (4, Filter::Money { .. }) => Ok(()),
        (5, Filter::Enum(values)) if values.iter().all(|v| v == "true" || v == "false") => Ok(()),
        (7..=8, Filter::DateTime { .. }) => Ok(()),
        _ => Err("Filter type does not match SKU column"),
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

fn contains_pattern(text: &str) -> String {
    format!(
        "%{}%",
        text.replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
    )
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
    product_id: uuid::Uuid,
    query: &PageQuery,
) -> anyhow::Result<()> {
    qb.push(" from skus s join products p on p.product_id = s.product_id and p.tenant_id = s.tenant_id where s.tenant_id = ")
        .push_bind(uuid::Uuid::from(tenant))
        .push(" and s.product_id = ")
        .push_bind(product_id)
        .push(" and s.status <> 'DELETED' and p.status <> 'DELETED'");
    if let Some(text) = query
        .search
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        let pattern = contains_pattern(text.trim());
        qb.push(" and (s.sku_code ilike ")
            .push_bind(pattern.clone())
            .push(" escape '!' or coalesce(s.barcode, '') ilike ")
            .push_bind(pattern.clone())
            .push(" escape '!' or coalesce(s.name, '') ilike ")
            .push_bind(pattern.clone())
            .push(" escape '!' or exists (select 1 from unnest(s.tags) tag where tag ilike ")
            .push_bind(pattern)
            .push(" escape '!'))");
    }
    for (column, filter) in &query.filters {
        let name = field(*column).map_err(anyhow::Error::msg)?;
        let qualified = format!("s.{name}");
        match filter {
            Filter::Text(text) => {
                qb.push(" and coalesce(")
                    .push(&qualified)
                    .push(", '') ilike ")
                    .push_bind(contains_pattern(text))
                    .push(" escape '!'");
            }
            Filter::Enum(values) if column.0 == 5 => {
                qb.push(" and s.is_active::text = any(")
                    .push_bind(values.iter().cloned().collect::<Vec<_>>())
                    .push(")");
            }
            Filter::Enum(values) => {
                qb.push(" and coalesce(")
                    .push(&qualified)
                    .push(", '') = any(")
                    .push_bind(values.iter().cloned().collect::<Vec<_>>())
                    .push(")");
            }
            Filter::Money { min, max } => {
                let value = if column.0 == 4 {
                    cost_expression()
                } else {
                    &qualified
                };
                if let Some(min) = min {
                    qb.push(" and ").push(value).push(" >= ").push_bind(*min);
                }
                if let Some(max) = max {
                    qb.push(" and ").push(value).push(" <= ").push_bind(*max);
                }
            }
            Filter::DateTime { start, end } => {
                if let Some(start) = start {
                    qb.push(" and ")
                        .push(&qualified)
                        .push(" >= ")
                        .push_bind(utc_time(*start)?);
                }
                if let Some(end) = end {
                    let end = utc_time(*end)?
                        .checked_add_signed(chrono::Duration::seconds(1))
                        .ok_or_else(|| anyhow::anyhow!("Date overflow"))?;
                    qb.push(" and ").push(&qualified).push(" < ").push_bind(end);
                }
            }
        }
    }
    if let Some(expression) = &query.expression {
        qb.push(" and ");
        append_expression(qb, expression)?;
    }
    Ok(())
}

fn append_expression(
    qb: &mut QueryBuilder<'_, Postgres>,
    expression: &FilterExpression,
) -> anyhow::Result<()> {
    match expression {
        FilterExpression::Condition { column, filter } => {
            append_expression_condition(qb, *column, filter)
        }
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

fn append_expression_condition(
    qb: &mut QueryBuilder<'_, Postgres>,
    column: ColumnId,
    filter: &Filter,
) -> anyhow::Result<()> {
    let name = field(column).map_err(anyhow::Error::msg)?;
    let qualified = format!("s.{name}");
    match filter {
        Filter::Text(text) => {
            qb.push("coalesce(")
                .push(&qualified)
                .push(", '') ilike ")
                .push_bind(contains_pattern(text))
                .push(" escape '!'");
        }
        Filter::Enum(values) if column.0 == 5 => {
            qb.push("s.is_active::text = any(")
                .push_bind(values.iter().cloned().collect::<Vec<_>>())
                .push(")");
        }
        Filter::Enum(values) => {
            qb.push("coalesce(")
                .push(&qualified)
                .push(", '') = any(")
                .push_bind(values.iter().cloned().collect::<Vec<_>>())
                .push(")");
        }
        Filter::Money { min, max } => {
            let value = if column.0 == 4 {
                cost_expression()
            } else {
                &qualified
            };
            qb.push("(");
            let mut has = false;
            if let Some(min) = min {
                qb.push(value).push(" >= ").push_bind(*min);
                has = true;
            }
            if let Some(max) = max {
                if has {
                    qb.push(" and ");
                }
                qb.push(value).push(" <= ").push_bind(*max);
                has = true;
            }
            if !has {
                qb.push("true");
            }
            qb.push(")");
        }
        Filter::DateTime { start, end } => {
            qb.push("(");
            let mut has = false;
            if let Some(start) = start {
                qb.push(&qualified)
                    .push(" >= ")
                    .push_bind(utc_time(*start)?);
                has = true;
            }
            if let Some(end) = end {
                if has {
                    qb.push(" and ");
                }
                let end = utc_time(*end)?
                    .checked_add_signed(chrono::Duration::seconds(1))
                    .ok_or_else(|| anyhow::anyhow!("Date overflow"))?;
                qb.push(&qualified).push(" < ").push_bind(end);
                has = true;
            }
            if !has {
                qb.push("true");
            }
            qb.push(")");
        }
    }
    Ok(())
}

pub(crate) fn append_ordering(
    qb: &mut QueryBuilder<'_, Postgres>,
    query: &PageQuery,
) -> anyhow::Result<()> {
    qb.push(" order by ");
    if query.sorts.is_empty() {
        qb.push("s.created_at desc, ");
    }
    for sort in &query.sorts {
        let name = field(sort.column).map_err(anyhow::Error::msg)?;
        if sort.column.0 == 4 {
            qb.push(cost_expression());
        } else if matches!(sort.column.0, 0..=3 | 6) {
            qb.push("lower(nullif(s.").push(name).push(", ''))");
        } else {
            qb.push("s.").push(name);
        }
        qb.push(if sort.descending { " desc" } else { " asc" })
            .push(" nulls last, ");
    }
    qb.push("s.sku_id desc");
    Ok(())
}

fn ordering(qb: &mut QueryBuilder<'_, Postgres>, query: &PageQuery) -> anyhow::Result<()> {
    append_ordering(qb, query)?;
    qb.push(" limit ")
        .push_bind(i64::from(query.limit))
        .push(" offset ")
        .push_bind(query.offset as i64);
    Ok(())
}

pub async fn query_skus(
    pool: &PgPool,
    tenant: TenantId,
    product_id: uuid::Uuid,
    query: &PageQuery,
) -> anyhow::Result<SkuGridPage> {
    validate(query).map_err(anyhow::Error::msg)?;
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    let mut count = QueryBuilder::<Postgres>::new("select count(*)");
    predicates(&mut count, tenant, product_id, query)?;
    let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let mut page = QueryBuilder::<Postgres>::new(
        "select s.sku_id, s.tenant_id, s.product_id, s.sku_code, s.barcode, s.name, s.sku_kind, ",
    );
    page.push(cost_expression())
        .push(" as cost_cent, s.is_active, s.status, s.created_at, s.updated_at");
    predicates(&mut page, tenant, product_id, query)?;
    ordering(&mut page, query)?;
    let data = page.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(SkuGridPage {
        data,
        total: u64::try_from(total)?,
    })
}

pub async fn query_sku_ranges(
    pool: &PgPool,
    tenant: TenantId,
    product_id: uuid::Uuid,
    request: &SkuGridRangesRequest,
) -> anyhow::Result<SkuGridRangesPage> {
    validate_ranges(request).map_err(anyhow::Error::msg)?;
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;

    let mut count = QueryBuilder::<Postgres>::new("select count(*)");
    predicates(&mut count, tenant, product_id, &request.query)?;
    let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let mut pages = Vec::with_capacity(request.ranges.len());
    for range in &request.ranges {
        let mut page = QueryBuilder::<Postgres>::new(
            "select s.sku_id, s.tenant_id, s.product_id, s.sku_code, s.barcode, s.name, s.sku_kind, ",
        );
        page.push(cost_expression())
            .push(" as cost_cent, s.is_active, s.status, s.created_at, s.updated_at");
        predicates(&mut page, tenant, product_id, &request.query)?;
        append_ordering(&mut page, &request.query)?;
        page.push(" limit ")
            .push_bind(i64::from(range.limit))
            .push(" offset ")
            .push_bind(range.offset as i64);
        let data = page.build_query_as().fetch_all(&mut *tx).await?;
        pages.push(SkuGridRangePage {
            offset: range.offset,
            data,
        });
    }
    tx.commit().await?;
    Ok(SkuGridRangesPage {
        total: u64::try_from(total)?,
        pages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::SortRule;
    use std::collections::BTreeMap;

    #[test]
    fn whitelist_and_filter_types_are_explicit() {
        let mut query = PageQuery {
            schema: 1,
            offset: 0,
            limit: 100,
            filters: BTreeMap::new(),
            sorts: vec![SortRule {
                column: ColumnId(0),
                descending: false,
            }],
            search: Some("%_'中文".into()),
            expression: None,
        };
        query.filters.insert(
            ColumnId(4),
            Filter::Money {
                min: Some(100),
                max: None,
            },
        );
        assert!(validate(&query).is_ok());
        let mut qb = QueryBuilder::<Postgres>::new("select s.sku_id");
        predicates(
            &mut qb,
            TenantId(uuid::Uuid::nil()),
            uuid::Uuid::nil(),
            &query,
        )
        .unwrap();
        ordering(&mut qb, &query).unwrap();
        let sql = qb.sql();
        assert!(sql.contains("s.product_id = $2"));
        assert!(sql.contains("s.status <> 'DELETED'"));
        assert!(!sql.contains("中文") && !sql.contains("drop table"));
        query
            .filters
            .insert(ColumnId(4), Filter::Text("bad".into()));
        assert!(validate(&query).is_err());
        query.sorts[0].column = ColumnId(99);
        assert!(validate(&query).is_err());
    }

    #[test]
    fn range_request_is_bounded_and_keeps_query_validation() {
        let query = PageQuery {
            schema: 1,
            offset: 0,
            limit: 500,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
            expression: None,
        };
        let mut request = SkuGridRangesRequest {
            query,
            ranges: vec![
                SkuGridRange {
                    offset: 5,
                    limit: 1000,
                },
                SkuGridRange {
                    offset: 900,
                    limit: 1000,
                },
            ],
        };
        assert!(validate_ranges(&request).is_ok());
        request.ranges.push(SkuGridRange {
            offset: 0,
            limit: 1,
        });
        assert_eq!(
            validate_ranges(&request),
            Err("One or two SKU ranges are required")
        );
        request.ranges.truncate(1);
        request.ranges[0].limit = 0;
        assert_eq!(validate_ranges(&request), Err("Invalid SKU range limit"));
        request.ranges[0].limit = 1;
        request.ranges[0].offset = i64::MAX as u64;
        assert_eq!(validate_ranges(&request), Err("Invalid SKU range offset"));
        request.ranges[0].offset = 0;
        request.query.sorts = vec![SortRule {
            column: ColumnId(99),
            descending: false,
        }];
        assert_eq!(validate_ranges(&request), Err("Unknown SKU column"));
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; uses connection-local temporary tables"]
    async fn sku_grid_counts_before_paging_and_scopes_parent() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table products (product_id uuid primary key, tenant_id uuid not null, status text not null)")
            .execute(&pool).await.unwrap();
        sqlx::query("create temporary table skus (sku_id uuid primary key, tenant_id uuid not null, product_id uuid not null, sku_code text not null, barcode text, name text, sku_kind text not null, cost_cent bigint, is_active boolean not null, status text not null, tags text[] not null, created_at timestamptz not null, updated_at timestamptz not null)")
            .execute(&pool).await.unwrap();
        sqlx::query("create temporary table sku_boms (tenant_id uuid not null, bundle_sku_id uuid not null, component_sku_id uuid not null, qty bigint not null)")
            .execute(&pool).await.unwrap();
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        let parent = uuid::Uuid::from_u128(10);
        let other_tenant = TenantId(uuid::Uuid::from_u128(2));
        let other_parent = uuid::Uuid::from_u128(11);
        sqlx::query("insert into products values ($1, $2, 'ACTIVE'), ($3, $2, 'ACTIVE'), ($4, $5, 'ACTIVE')")
            .bind(parent)
            .bind(uuid::Uuid::from(tenant))
            .bind(other_parent)
            .bind(uuid::Uuid::from_u128(12))
            .bind(uuid::Uuid::from(other_tenant))
            .execute(&pool)
            .await
            .unwrap();
        let sku_rows = [
            (
                1_u128,
                "SKU-ALPHA",
                Some("100%_A"),
                Some("Red Alpha"),
                "NORMAL",
                100_i64,
                true,
                "ACTIVE",
                "2026-01-01 00:00:00+00",
                "2026-01-01 00:00:00+00",
                "{alpha}",
            ),
            (
                2,
                "SKU-BETA",
                Some("BETA"),
                Some("Blue Beta"),
                "KIT",
                200,
                false,
                "ACTIVE",
                "2026-01-02 00:00:00+00",
                "2026-01-02 00:00:00+00",
                "{beta}",
            ),
            (
                3,
                "SKU-GAMMA",
                None,
                Some("Gamma"),
                "NORMAL",
                300,
                true,
                "ACTIVE",
                "2026-01-03 00:00:00+00",
                "2026-01-03 00:00:00+00",
                "{gamma}",
            ),
            (
                4,
                "SKU-DELETED",
                None,
                Some("Deleted"),
                "NORMAL",
                400,
                true,
                "DELETED",
                "2026-01-04 00:00:00+00",
                "2026-01-04 00:00:00+00",
                "{}",
            ),
        ];
        for (id, code, barcode, name, kind, cost, active, status, created, updated, tags) in
            sku_rows
        {
            sqlx::query("insert into skus (sku_id, tenant_id, product_id, sku_code, barcode, name, sku_kind, cost_cent, is_active, status, tags, created_at, updated_at) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::text[], $12::timestamptz, $13::timestamptz)")
                .bind(uuid::Uuid::from_u128(id))
                .bind(uuid::Uuid::from(tenant))
                .bind(parent)
                .bind(code)
                .bind(barcode)
                .bind(name)
                .bind(kind)
                .bind(cost)
                .bind(active)
                .bind(status)
                .bind(tags)
                .bind(created)
                .bind(updated)
                .execute(&pool).await.unwrap();
        }
        sqlx::query("insert into skus (sku_id, tenant_id, product_id, sku_code, barcode, name, sku_kind, cost_cent, is_active, status, tags, created_at, updated_at) values ($1, $2, $3, 'OTHER-PARENT', null, 'Other', 'NORMAL', 1, true, 'ACTIVE', '{}', '2026-01-01+00'::timestamptz, '2026-01-01+00'::timestamptz), ($4, $5, $6, 'OTHER-TENANT', null, 'Other tenant', 'NORMAL', 1, true, 'ACTIVE', '{}', '2026-01-01+00'::timestamptz, '2026-01-01+00'::timestamptz)")
            .bind(uuid::Uuid::from_u128(20)).bind(uuid::Uuid::from(tenant)).bind(other_parent)
            .bind(uuid::Uuid::from_u128(21)).bind(uuid::Uuid::from(other_tenant)).bind(uuid::Uuid::from_u128(12))
            .execute(&pool).await.unwrap();
        sqlx::query("insert into skus (sku_id, tenant_id, product_id, sku_code, name, sku_kind, cost_cent, is_active, status, tags, created_at, updated_at) values ($1, $2, $3, 'COMPONENT', 'Component', 'NORMAL', 30, true, 'ACTIVE', '{}', '2026-01-01+00'::timestamptz, '2026-01-01+00'::timestamptz)")
            .bind(uuid::Uuid::from_u128(30)).bind(uuid::Uuid::from(tenant)).bind(other_parent)
            .execute(&pool).await.unwrap();
        sqlx::query("insert into sku_boms (tenant_id, bundle_sku_id, component_sku_id, qty) values ($1, $2, $3, 4)")
            .bind(uuid::Uuid::from(tenant)).bind(uuid::Uuid::from_u128(2)).bind(uuid::Uuid::from_u128(30))
            .execute(&pool).await.unwrap();
        let query = PageQuery {
            schema: 1,
            offset: 1,
            limit: 1,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
            expression: None,
        };
        let page = query_skus(&pool, tenant, parent, &query).await.unwrap();
        assert_eq!((page.total, page.data.len()), (3, 1));
        assert_eq!(page.data[0].product_id, parent);

        let mut escaped_search = query.clone();
        escaped_search.offset = 0;
        escaped_search.limit = 10;
        escaped_search.search = Some("100%_A".into());
        let page = query_skus(&pool, tenant, parent, &escaped_search)
            .await
            .unwrap();
        assert_eq!((page.total, page.data.len()), (1, 1));
        assert_eq!(page.data[0].sku_code, "SKU-ALPHA");

        let nested = FilterExpression::And(vec![
            FilterExpression::Or(vec![
                FilterExpression::Condition {
                    column: ColumnId(3),
                    filter: Filter::Enum(["NORMAL".into()].into_iter().collect()),
                },
                FilterExpression::Condition {
                    column: ColumnId(3),
                    filter: Filter::Enum(["KIT".into()].into_iter().collect()),
                },
            ]),
            FilterExpression::Condition {
                column: ColumnId(4),
                filter: Filter::Money {
                    min: Some(100),
                    max: Some(120),
                },
            },
        ]);
        let mut expression_query = escaped_search.clone();
        expression_query.search = None;
        expression_query.expression = Some(nested);
        let page = query_skus(&pool, tenant, parent, &expression_query)
            .await
            .unwrap();
        assert_eq!(page.total, 2);

        let mut active_query = escaped_search.clone();
        active_query.search = None;
        active_query.filters.insert(
            ColumnId(5),
            Filter::Enum(["false".into()].into_iter().collect()),
        );
        let page = query_skus(&pool, tenant, parent, &active_query)
            .await
            .unwrap();
        assert_eq!(
            (page.total, page.data[0].sku_code.as_str()),
            (1, "SKU-BETA")
        );

        let mut cost_query = escaped_search.clone();
        cost_query.search = None;
        cost_query.filters.insert(
            ColumnId(4),
            Filter::Money {
                min: None,
                max: Some(200),
            },
        );
        let page = query_skus(&pool, tenant, parent, &cost_query)
            .await
            .unwrap();
        assert_eq!(page.total, 2);

        let mut date_query = escaped_search.clone();
        date_query.search = None;
        date_query.filters.insert(
            ColumnId(7),
            Filter::DateTime {
                start: None,
                end: Some(20260102000000),
            },
        );
        let page = query_skus(&pool, tenant, parent, &date_query)
            .await
            .unwrap();
        assert_eq!(page.total, 2);

        let mut sorted = escaped_search.clone();
        sorted.search = None;
        sorted.offset = 1;
        sorted.limit = 1;
        sorted.sorts = vec![SortRule {
            column: ColumnId(4),
            descending: false,
        }];
        let page = query_skus(&pool, tenant, parent, &sorted).await.unwrap();
        assert_eq!(
            (page.total, page.data[0].sku_code.as_str()),
            (3, "SKU-BETA")
        );
        assert_eq!(page.data[0].cost_cent, Some(120));

        let ranges = query_sku_ranges(
            &pool,
            tenant,
            parent,
            &SkuGridRangesRequest {
                query: sorted.clone(),
                ranges: vec![
                    SkuGridRange {
                        offset: 0,
                        limit: 2,
                    },
                    SkuGridRange {
                        offset: 1,
                        limit: 2,
                    },
                ],
            },
        )
        .await
        .unwrap();
        assert_eq!(ranges.total, 3);
        assert_eq!(ranges.pages.len(), 2);
        assert_eq!(ranges.pages[0].offset, 0);
        assert_eq!(ranges.pages[1].offset, 1);
        assert_eq!(ranges.pages[0].data.len(), 2);
        assert_eq!(ranges.pages[1].data.len(), 2);
        assert_eq!(
            ranges.pages[0].data[1].sku_id,
            ranges.pages[1].data[0].sku_id
        );

        let mut malformed = escaped_search;
        malformed.expression = Some(FilterExpression::Condition {
            column: ColumnId(99),
            filter: Filter::Text("x".into()),
        });
        assert!(query_skus(&pool, tenant, parent, &malformed).await.is_err());
        pool.close().await;
    }
}
