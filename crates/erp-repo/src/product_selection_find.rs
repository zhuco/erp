//! Bounded cell search over an actor-bound frozen product selection.
use std::collections::BTreeSet;

use erp_grid_core::ColumnId;
use erp_types::{TenantId, UserId};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

pub use crate::product_grid_find::{FindAnchor, FindHit, FindRequest};
use crate::product_selection::{self, SelectionError};

pub fn validate(request: &FindRequest) -> Result<(), SelectionError> {
    let query = &request.query;
    if query.schema != 1
        || !query.filters.is_empty()
        || query.search.is_some()
        || query.expression.is_some()
    {
        return Err(SelectionError::Invalid(
            "frozen find query may contain only schema and sorts".into(),
        ));
    }
    product_selection::validate_sorts(&query.sorts)?;
    if request.text.is_empty()
        || request.text.chars().count() > 256
        || request.text.len() > 1024
        || request.text.contains('\0')
    {
        return Err(SelectionError::Invalid("Invalid find text".into()));
    }
    if !(1..=7).contains(&request.columns.len()) {
        return Err(SelectionError::Invalid(
            "Find requires between one and seven columns".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    if request
        .columns
        .iter()
        .any(|column| column.0 > 6 || !seen.insert(*column))
    {
        return Err(SelectionError::Invalid(
            "Invalid or duplicate find column".into(),
        ));
    }
    if request
        .after
        .as_ref()
        .is_some_and(|anchor| anchor.row_id.is_nil() || !request.columns.contains(&anchor.column))
    {
        return Err(SelectionError::Invalid(
            "Find anchor must identify a visible cell".into(),
        ));
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

fn cell_expression(column: ColumnId) -> Result<&'static str, SelectionError> {
    match column.0 {
        0 => Ok("coalesce(data ->> 'product_code', '')"),
        1 => Ok("coalesce(data ->> 'name', '')"),
        2 => Ok("coalesce(data ->> 'brand', '')"),
        3 => Ok("coalesce(data ->> 'category', '')"),
        4 => Ok("coalesce(data ->> 'status', '')"),
        5 => Ok("coalesce(data ->> 'created_at', '')"),
        6 => Ok("coalesce(data ->> 'updated_at', '')"),
        _ => Err(SelectionError::Invalid(
            "Unknown product find column".into(),
        )),
    }
}

#[derive(sqlx::FromRow)]
struct FindHitRow {
    row_id: Uuid,
    column_id: i64,
    row_offset: i64,
    wrapped: bool,
}

/// Finds at most one visible cell in the frozen membership and ordering.
pub async fn find(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    token: Uuid,
    request: &FindRequest,
) -> Result<Option<FindHit>, SelectionError> {
    validate(request)?;
    let mut tx = pool.begin().await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    product_selection::header(&mut tx, tenant, actor, token).await?;

    let mut sql = QueryBuilder::<Postgres>::new(
        "with ranked as (select product_id, data, (row_number() over (order by ",
    );
    if request.query.sorts.is_empty() {
        sql.push("ordinal");
    } else {
        for sort in &request.query.sorts {
            sql.push(product_selection::selection_sort_expression(sort.column.0)?)
                .push(if sort.descending { " desc" } else { " asc" })
                .push(" nulls last, ");
        }
        sql.push("product_id asc");
    }
    sql.push(") - 1)::bigint as row_offset from grid_product_selection_items where selection_token = ")
        .push_bind(token)
        .push("), cells as (select ranked.product_id as row_id, ranked.row_offset, cell.column_id, cell.column_position, cell.cell_text from ranked cross join lateral (values ");
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
            .ok_or_else(|| SelectionError::Invalid("Find anchor column is not visible".into()))?;
        sql.push(position as i64)
            .push("::bigint as column_position from ranked where product_id = ")
            .push_bind(anchor.row_id);
    } else {
        sql.push("null::bigint as column_position from ranked where false");
    }
    sql.push(" limit 1), matching as (select cells.*, anchor.row_offset as anchor_offset, anchor.column_position as anchor_column_position from cells left join anchor on true where cell_text ")
        .push(if request.case_sensitive { "like " } else { "ilike " })
        .push_bind(contains_pattern(&request.text))
        .push(" escape '!') select row_id, column_id, row_offset, (case when anchor_offset is null then 0 when ");
    let direction = if request.backward {
        "(row_offset, column_position) < (anchor_offset, anchor_column_position)"
    } else {
        "(row_offset, column_position) > (anchor_offset, anchor_column_position)"
    };
    sql.push(direction)
        .push(" then 0 else 1 end) = 1 and anchor_offset is not null as wrapped from matching order by case when anchor_offset is null then 0 when ")
        .push(direction)
        .push(" then 0 else 1 end, row_offset ")
        .push(if request.backward { "desc" } else { "asc" })
        .push(", column_position ")
        .push(if request.backward { "desc" } else { "asc" })
        .push(" limit 1");

    let row: Option<FindHitRow> = sql.build_query_as().fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    row.map(|row| {
        Ok(FindHit {
            row_id: row.row_id,
            column: ColumnId(
                u64::try_from(row.column_id)
                    .map_err(|_| SelectionError::Invalid("invalid stored find column".into()))?,
            ),
            offset: u64::try_from(row.row_offset)
                .map_err(|_| SelectionError::Invalid("invalid stored find offset".into()))?,
            wrapped: row.wrapped,
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{SortRule, paged_query::PageQuery};
    use std::collections::BTreeMap;

    fn frozen_find_request(text: &str, columns: Vec<ColumnId>) -> FindRequest {
        FindRequest {
            query: PageQuery {
                schema: 1,
                offset: u64::MAX,
                limit: 0,
                filters: BTreeMap::new(),
                sorts: vec![],
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
    fn frozen_find_rejects_live_query_predicates_but_ignores_page_range() {
        assert!(validate(&frozen_find_request("x", vec![ColumnId(0)])).is_ok());
        let mut request = frozen_find_request("x", vec![ColumnId(0)]);
        request.query.search = Some("live".into());
        assert!(matches!(
            validate(&request),
            Err(SelectionError::Invalid(_))
        ));
        request.query.search = None;
        request.query.sorts = vec![SortRule {
            column: ColumnId(7),
            descending: false,
        }];
        assert!(matches!(
            validate(&request),
            Err(SelectionError::Invalid(_))
        ));
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates connection-local temporary tables only"]
    async fn frozen_find_uses_snapshot_sort_columns_wrap_and_binding() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table grid_product_selections (token uuid primary key, tenant_id uuid not null, actor_user_id uuid not null, normalized_query jsonb not null, query_fingerprint text not null, total bigint not null, expires_at timestamptz not null)").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_items (selection_token uuid not null references grid_product_selections(token) on delete cascade, ordinal bigint not null, product_id uuid not null, product_version timestamptz not null, data jsonb not null, primary key(selection_token, ordinal), unique(selection_token, product_id))").execute(&pool).await.unwrap();
        let tenant = TenantId(Uuid::from_u128(1));
        let actor = UserId(Uuid::from_u128(2));
        let token = Uuid::from_u128(3);
        sqlx::query("insert into grid_product_selections values ($1,$2,$3,'{}','f',3,clock_timestamp()+interval '5 minutes')")
            .bind(token).bind(Uuid::from(tenant)).bind(Uuid::from(actor)).execute(&pool).await.unwrap();
        let rows = [
            (
                20_u128,
                0_i64,
                serde_json::json!({"product_code":"C","name":"needle last","brand":null,"category":"Goods","status":"ACTIVE","created_at":"2026-09-10T04:34:56Z","updated_at":"2026-09-10T04:35:00Z"}),
            ),
            (
                10,
                1,
                serde_json::json!({"product_code":"A","name":"plain","brand":"needle brand","category":"Goods","status":"ACTIVE","created_at":"2026-09-10T04:34:57.123Z","updated_at":"2026-09-10T04:35:01Z"}),
            ),
            (
                30,
                2,
                serde_json::json!({"product_code":"B","name":"NEEDLE middle","brand":null,"category":"Goods","status":"ACTIVE","created_at":"2026-09-10T04:34:58.123456Z","updated_at":"2026-09-10T04:35:02Z"}),
            ),
        ];
        for (id, ordinal, data) in rows {
            sqlx::query("insert into grid_product_selection_items values ($1,$2,$3,now(),$4)")
                .bind(token)
                .bind(ordinal)
                .bind(Uuid::from_u128(id))
                .bind(data)
                .execute(&pool)
                .await
                .unwrap();
        }

        let mut request = frozen_find_request("needle", vec![ColumnId(2), ColumnId(1)]);
        request.query.sorts = vec![SortRule {
            column: ColumnId(0),
            descending: false,
        }];
        let first = find(&pool, tenant, actor, token, &request)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (first.row_id.as_u128(), first.column, first.offset),
            (10, ColumnId(2), 0)
        );
        request.after = Some(FindAnchor {
            row_id: first.row_id,
            column: first.column,
        });
        let second = find(&pool, tenant, actor, token, &request)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (second.row_id.as_u128(), second.offset, second.wrapped),
            (30, 1, false)
        );
        request.after = Some(FindAnchor {
            row_id: second.row_id,
            column: second.column,
        });
        let third = find(&pool, tenant, actor, token, &request)
            .await
            .unwrap()
            .unwrap();
        assert_eq!((third.row_id.as_u128(), third.offset), (20, 2));
        request.after = Some(FindAnchor {
            row_id: third.row_id,
            column: third.column,
        });
        assert!(
            find(&pool, tenant, actor, token, &request)
                .await
                .unwrap()
                .unwrap()
                .wrapped
        );

        request.backward = true;
        request.after = Some(FindAnchor {
            row_id: Uuid::from_u128(10),
            column: ColumnId(2),
        });
        let backward = find(&pool, tenant, actor, token, &request)
            .await
            .unwrap()
            .unwrap();
        assert_eq!((backward.row_id.as_u128(), backward.wrapped), (20, true));

        let date = frozen_find_request(".123456Z", vec![ColumnId(5)]);
        assert_eq!(
            find(&pool, tenant, actor, token, &date)
                .await
                .unwrap()
                .unwrap()
                .row_id
                .as_u128(),
            30
        );
        assert!(matches!(
            find(&pool, tenant, UserId(Uuid::from_u128(99)), token, &date).await,
            Err(SelectionError::Unavailable)
        ));
        sqlx::query("update grid_product_selections set expires_at=clock_timestamp()-interval '1 second' where token=$1").bind(token).execute(&pool).await.unwrap();
        assert!(matches!(
            find(&pool, tenant, actor, token, &date).await,
            Err(SelectionError::Unavailable)
        ));
        pool.close().await;
    }
}
