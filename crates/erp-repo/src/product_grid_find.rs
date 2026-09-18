//! Bounded server-side cell search over the complete filtered product query.
use std::collections::BTreeSet;

use erp_grid_core::{ColumnId, paged_query::PageQuery};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::product_grid::{append_ordering, predicates};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindAnchor {
    pub row_id: Uuid,
    pub column: ColumnId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindRequest {
    pub query: PageQuery,
    pub text: String,
    pub columns: Vec<ColumnId>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub backward: bool,
    #[serde(default)]
    pub after: Option<FindAnchor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FindHit {
    pub row_id: Uuid,
    pub column: ColumnId,
    pub offset: u64,
    pub wrapped: bool,
}

pub fn validate(request: &FindRequest) -> Result<(), &'static str> {
    crate::product_grid::validate(&request.query)?;
    if request.text.is_empty()
        || request.text.chars().count() > 256
        || request.text.len() > 1024
        || request.text.contains('\0')
    {
        return Err("Invalid find text");
    }
    if !(1..=7).contains(&request.columns.len()) {
        return Err("Find requires between one and seven columns");
    }
    let mut seen = BTreeSet::new();
    if request
        .columns
        .iter()
        .any(|column| column.0 > 6 || !seen.insert(*column))
    {
        return Err("Invalid or duplicate find column");
    }
    if request
        .after
        .as_ref()
        .is_some_and(|anchor| anchor.row_id.is_nil() || !request.columns.contains(&anchor.column))
    {
        return Err("Find anchor must identify a visible cell");
    }
    Ok(())
}

fn contains_pattern(text: &str) -> String {
    format!(
        "%{}%",
        text.replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
    )
}

fn cell_expression(column: ColumnId) -> anyhow::Result<&'static str> {
    match column.0 {
        0 => Ok("coalesce(product_code, '')"),
        1 => Ok("name"),
        2 => Ok("coalesce(brand, '')"),
        3 => Ok("coalesce(category, '')"),
        4 => Ok("status"),
        5 => Ok(
            "to_char(created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS') || case when extract(microseconds from created_at)::bigint % 1000000 = 0 then '' when extract(microseconds from created_at)::bigint % 1000 = 0 then '.' || to_char(created_at at time zone 'UTC', 'MS') else '.' || to_char(created_at at time zone 'UTC', 'US') end || 'Z'",
        ),
        6 => Ok(
            "to_char(updated_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS') || case when extract(microseconds from updated_at)::bigint % 1000000 = 0 then '' when extract(microseconds from updated_at)::bigint % 1000 = 0 then '.' || to_char(updated_at at time zone 'UTC', 'MS') else '.' || to_char(updated_at at time zone 'UTC', 'US') end || 'Z'",
        ),
        _ => anyhow::bail!("Unknown product find column"),
    }
}

#[derive(sqlx::FromRow)]
struct FindHitRow {
    row_id: Uuid,
    column_id: i64,
    row_offset: i64,
    wrapped: bool,
}

/// Finds one cell without applying the page offset/limit or collecting rows client-side.
pub async fn find(
    pool: &PgPool,
    tenant: TenantId,
    request: &FindRequest,
) -> anyhow::Result<Option<FindHit>> {
    validate(request).map_err(anyhow::Error::msg)?;
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;

    let mut sql = QueryBuilder::<Postgres>::new(
        "with ranked as (select product_id, product_code, name, brand, category, status, created_at, updated_at, (row_number() over (",
    );
    append_ordering(&mut sql, &request.query)?;
    sql.push(") - 1)::bigint as row_offset");
    predicates(&mut sql, tenant, &request.query)?;
    sql.push("), cells as (select ranked.product_id as row_id, ranked.row_offset, cell.column_id, cell.column_position, cell.cell_text from ranked cross join lateral (values ");
    for (position, column) in request.columns.iter().enumerate() {
        if position > 0 {
            sql.push(", ");
        }
        sql.push("(")
            .push(column.0 as i64)
            .push("::bigint, ")
            .push(position as i64)
            .push("::bigint, ")
            .push(cell_expression(*column)?)
            .push(")");
    }
    sql.push(") as cell(column_id, column_position, cell_text)), anchor as (select row_offset, ");
    if let Some(anchor) = &request.after {
        let position = request
            .columns
            .iter()
            .position(|column| *column == anchor.column)
            .ok_or_else(|| anyhow::anyhow!("Find anchor column is not visible"))?;
        sql.push(position as i64)
            .push("::bigint as column_position from ranked where product_id = ")
            .push_bind(anchor.row_id);
    } else {
        sql.push("null::bigint as column_position from ranked where false");
    }
    sql.push(" limit 1), matching as (select cells.*, anchor.row_offset as anchor_offset, anchor.column_position as anchor_column_position from cells left join anchor on true where cell_text ");
    sql.push(if request.case_sensitive { "like " } else { "ilike " }).push_bind(contains_pattern(&request.text)).push(" escape '!') select row_id, column_id, row_offset, (case when anchor_offset is null then 0 when ");
    if request.backward {
        sql.push("(row_offset, column_position) < (anchor_offset, anchor_column_position)");
    } else {
        sql.push("(row_offset, column_position) > (anchor_offset, anchor_column_position)");
    }
    sql.push(" then 0 else 1 end) = 1 and anchor_offset is not null as wrapped from matching order by case when anchor_offset is null then 0 when ");
    if request.backward {
        sql.push("(row_offset, column_position) < (anchor_offset, anchor_column_position)");
    } else {
        sql.push("(row_offset, column_position) > (anchor_offset, anchor_column_position)");
    }
    sql.push(" then 0 else 1 end, row_offset ")
        .push(if request.backward { "desc" } else { "asc" })
        .push(", column_position ")
        .push(if request.backward { "desc" } else { "asc" })
        .push(" limit 1");

    let row: Option<FindHitRow> = sql.build_query_as().fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    row.map(|row| {
        Ok(FindHit {
            row_id: row.row_id,
            column: ColumnId(u64::try_from(row.column_id)?),
            offset: u64::try_from(row.row_offset)?,
            wrapped: row.wrapped,
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn request(text: &str, columns: Vec<ColumnId>) -> FindRequest {
        FindRequest {
            query: PageQuery {
                schema: 1,
                offset: 2,
                limit: 1,
                filters: BTreeMap::new(),
                sorts: vec![erp_grid_core::SortRule {
                    column: ColumnId(0),
                    descending: false,
                }],
                search: None,
                expression: None,
            },
            text: text.into(),
            columns,
            case_sensitive: false,
            backward: false,
            after: None,
        }
    }

    #[test]
    fn validates_text_and_ordered_unique_columns() {
        assert!(validate(&request("x", vec![ColumnId(1), ColumnId(0)])).is_ok());
        assert!(validate(&request("", vec![ColumnId(0)])).is_err());
        assert!(validate(&request(&"界".repeat(257), vec![ColumnId(0)])).is_err());
        assert!(validate(&request("x", vec![ColumnId(0), ColumnId(0)])).is_err());
        assert!(validate(&request("x", vec![ColumnId(7)])).is_err());
        let mut invalid_anchor = request("x", vec![ColumnId(0)]);
        invalid_anchor.after = Some(FindAnchor {
            row_id: Uuid::nil(),
            column: ColumnId(0),
        });
        assert!(validate(&invalid_anchor).is_err());
        invalid_anchor.after = Some(FindAnchor {
            row_id: Uuid::from_u128(1),
            column: ColumnId(1),
        });
        assert!(validate(&invalid_anchor).is_err());
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates and drops an isolated schema"]
    async fn postgres_finds_across_pages_wraps_and_preserves_scope() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("product_grid_find_test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("create schema {schema}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(&format!("set search_path to {schema}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("create table products (product_id uuid primary key, tenant_id uuid not null, product_code text, name text not null, alias text, primary_barcode text, brand text, category text, status text not null, created_at timestamptz not null, updated_at timestamptz not null)").execute(&pool).await.unwrap();
        let tenant = TenantId(Uuid::from_u128(1));
        sqlx::query("insert into products values ($1,$5,'A','needle-name',null,null,'needle-brand','cat','ACTIVE','2026-09-10 12:34:56+08','2026-09-10 04:34:57.123456+00'), ($2,$5,'B','plain',null,null,'100%_literal','cat','ACTIVE','2026-09-10 04:34:58.123+00',now()), ($3,$5,'C','last needle',null,null,null,'cat','ACTIVE','2026-09-10 04:34:59.120+00','2026-09-10 04:35:00.123400+00'), ($4,$6,'D','needle-other',null,null,null,'cat','ACTIVE',now(),now()), ($7,$5,'E','needle-deleted',null,null,null,'cat','DELETED',now(),now())")
            .bind(Uuid::from_u128(11)).bind(Uuid::from_u128(12)).bind(Uuid::from_u128(13)).bind(Uuid::from_u128(14)).bind(Uuid::from(tenant)).bind(Uuid::from_u128(2)).bind(Uuid::from_u128(15)).execute(&pool).await.unwrap();

        let mut input = request("needle", vec![ColumnId(2), ColumnId(1)]);
        let first = find(&pool, tenant, &input).await.unwrap().unwrap();
        assert_eq!(
            (first.row_id, first.column, first.offset, first.wrapped),
            (Uuid::from_u128(11), ColumnId(2), 0, false)
        );
        input.after = Some(FindAnchor {
            row_id: first.row_id,
            column: first.column,
        });
        let same_row = find(&pool, tenant, &input).await.unwrap().unwrap();
        assert_eq!(
            (same_row.row_id, same_row.column),
            (Uuid::from_u128(11), ColumnId(1))
        );
        input.after = Some(FindAnchor {
            row_id: same_row.row_id,
            column: same_row.column,
        });
        let second = find(&pool, tenant, &input).await.unwrap().unwrap();
        assert_eq!(
            (second.row_id, second.offset, second.wrapped),
            (Uuid::from_u128(13), 2, false)
        );
        input.after = Some(FindAnchor {
            row_id: second.row_id,
            column: second.column,
        });
        let wrapped = find(&pool, tenant, &input).await.unwrap().unwrap();
        assert_eq!(
            (wrapped.row_id, wrapped.wrapped),
            (Uuid::from_u128(11), true)
        );
        input.backward = true;
        input.after = Some(FindAnchor {
            row_id: Uuid::from_u128(11),
            column: ColumnId(2),
        });
        let backward = find(&pool, tenant, &input).await.unwrap().unwrap();
        assert_eq!(
            (backward.row_id, backward.wrapped),
            (Uuid::from_u128(13), true)
        );
        input.backward = false;
        input.after = Some(FindAnchor {
            row_id: Uuid::from_u128(999),
            column: ColumnId(1),
        });
        assert!(!find(&pool, tenant, &input).await.unwrap().unwrap().wrapped);

        assert_eq!(
            find(&pool, tenant, &request("%_", vec![ColumnId(2)]))
                .await
                .unwrap()
                .unwrap()
                .row_id,
            Uuid::from_u128(12)
        );
        assert_eq!(
            find(
                &pool,
                tenant,
                &request("2026-09-10T04:34:56Z", vec![ColumnId(5)])
            )
            .await
            .unwrap()
            .unwrap()
            .row_id,
            Uuid::from_u128(11)
        );
        assert_eq!(
            find(&pool, tenant, &request(".123456Z", vec![ColumnId(6)]))
                .await
                .unwrap()
                .unwrap()
                .row_id,
            Uuid::from_u128(11)
        );
        assert_eq!(
            find(&pool, tenant, &request(".123Z", vec![ColumnId(5)]))
                .await
                .unwrap()
                .unwrap()
                .row_id,
            Uuid::from_u128(12)
        );
        let millis: chrono::DateTime<chrono::Utc> = "2026-09-10T04:34:59.120Z".parse().unwrap();
        let micros: chrono::DateTime<chrono::Utc> = "2026-09-10T04:35:00.123400Z".parse().unwrap();
        let millis_json = serde_json::to_string(&millis).unwrap();
        let micros_json = serde_json::to_string(&micros).unwrap();
        assert_eq!(
            find(
                &pool,
                tenant,
                &request(millis_json.trim_matches('"'), vec![ColumnId(5)])
            )
            .await
            .unwrap()
            .unwrap()
            .row_id,
            Uuid::from_u128(13)
        );
        assert_eq!(
            find(
                &pool,
                tenant,
                &request(micros_json.trim_matches('"'), vec![ColumnId(6)])
            )
            .await
            .unwrap()
            .unwrap()
            .row_id,
            Uuid::from_u128(13)
        );

        sqlx::query("set search_path to public")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(&format!("drop schema {schema} cascade"))
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }
}
