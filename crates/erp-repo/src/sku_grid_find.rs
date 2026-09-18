//! Bounded server-side cell search over the complete filtered SKU child grid.
use std::collections::BTreeSet;

use erp_grid_core::ColumnId;
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

pub use crate::product_grid_find::{FindAnchor, FindHit, FindRequest};
use crate::sku_grid::{append_ordering, cost_expression, predicates};

pub fn validate(request: &FindRequest) -> Result<(), &'static str> {
    crate::sku_grid::validate(&request.query)?;
    if request.text.is_empty()
        || request.text.chars().count() > 256
        || request.text.len() > 1024
        || request.text.contains('\0')
    {
        return Err("Invalid find text");
    }
    if !(1..=9).contains(&request.columns.len()) {
        return Err("Find requires between one and nine columns");
    }
    let mut seen = BTreeSet::new();
    if request
        .columns
        .iter()
        .any(|column| column.0 > 8 || !seen.insert(*column))
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
        0 => Ok("sku_code"),
        1 => Ok("coalesce(barcode, '')"),
        2 => Ok("coalesce(name, '')"),
        3 => Ok("sku_kind"),
        4 => Ok(
            "case when effective_cost_cent is null then '' else case when effective_cost_cent < 0 then '-' else '' end || trunc(abs(effective_cost_cent::numeric) / 100)::text || '.' || lpad(mod(abs(effective_cost_cent::numeric), 100)::text, 2, '0') end",
        ),
        5 => Ok("is_active::text"),
        6 => Ok("status"),
        7 => Ok(
            "to_char(created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS') || case when extract(microseconds from created_at)::bigint % 1000000 = 0 then '' when extract(microseconds from created_at)::bigint % 1000 = 0 then '.' || to_char(created_at at time zone 'UTC', 'MS') else '.' || to_char(created_at at time zone 'UTC', 'US') end || 'Z'",
        ),
        8 => Ok(
            "to_char(updated_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS') || case when extract(microseconds from updated_at)::bigint % 1000000 = 0 then '' when extract(microseconds from updated_at)::bigint % 1000 = 0 then '.' || to_char(updated_at at time zone 'UTC', 'MS') else '.' || to_char(updated_at at time zone 'UTC', 'US') end || 'Z'",
        ),
        _ => anyhow::bail!("Unknown SKU find column"),
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
    product_id: Uuid,
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
        "with ranked as (select s.sku_id, s.sku_code, s.barcode, s.name, s.sku_kind, ",
    );
    sql.push(cost_expression()).push(
        " as effective_cost_cent, s.is_active, s.status, s.created_at, s.updated_at, (row_number() over (",
    );
    append_ordering(&mut sql, &request.query)?;
    sql.push(") - 1)::bigint as row_offset");
    predicates(&mut sql, tenant, product_id, &request.query)?;
    sql.push("), cells as (select ranked.sku_id as row_id, ranked.row_offset, cell.column_id, cell.column_position, cell.cell_text from ranked cross join lateral (values ");
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
            .push("::bigint as column_position from ranked where sku_id = ")
            .push_bind(anchor.row_id);
    } else {
        sql.push("null::bigint as column_position from ranked where false");
    }
    sql.push(" limit 1), matching as (select cells.*, anchor.row_offset as anchor_offset, anchor.column_position as anchor_column_position from cells left join anchor on true where cell_text ");
    sql.push(if request.case_sensitive { "like " } else { "ilike " })
        .push_bind(contains_pattern(&request.text))
        .push(" escape '!') select row_id, column_id, row_offset, (case when anchor_offset is null then 0 when ");
    let comparison = if request.backward { "<" } else { ">" };
    sql.push("(row_offset, column_position) ")
        .push(comparison)
        .push(" (anchor_offset, anchor_column_position)");
    sql.push(" then 0 else 1 end) = 1 and anchor_offset is not null as wrapped from matching order by case when anchor_offset is null then 0 when (row_offset, column_position) ")
        .push(comparison)
        .push(" (anchor_offset, anchor_column_position) then 0 else 1 end, row_offset ")
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
    use erp_grid_core::{SortRule, paged_query::PageQuery};
    use std::collections::BTreeMap;

    fn request(text: &str, columns: Vec<ColumnId>) -> FindRequest {
        FindRequest {
            query: PageQuery {
                schema: 1,
                offset: 50,
                limit: 1,
                filters: BTreeMap::new(),
                sorts: vec![SortRule {
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
    fn sku_find_validates_all_nine_ordered_unique_columns() {
        assert!(validate(&request("x", (0..=8).map(ColumnId).collect())).is_ok());
        assert!(validate(&request("x", vec![ColumnId(9)])).is_err());
        assert!(validate(&request("x", vec![ColumnId(1), ColumnId(1)])).is_err());
        assert!(validate(&request("", vec![ColumnId(0)])).is_err());
        let mut invalid_anchor = request("x", vec![ColumnId(0)]);
        invalid_anchor.after = Some(FindAnchor {
            row_id: Uuid::from_u128(1),
            column: ColumnId(1),
        });
        assert!(validate(&invalid_anchor).is_err());
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates and drops an isolated schema"]
    async fn sku_find_crosses_pages_wraps_and_preserves_parent_tenant_scope() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("sku_grid_find_test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("create schema {schema}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(&format!("set search_path to {schema}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("create table products (product_id uuid primary key, tenant_id uuid not null, status text not null)")
            .execute(&pool).await.unwrap();
        sqlx::query("create table skus (sku_id uuid primary key, tenant_id uuid not null, product_id uuid not null, sku_code text not null, barcode text, name text, sku_kind text not null, cost_cent bigint, is_active boolean not null, status text not null, tags text[] not null default '{}', created_at timestamptz not null, updated_at timestamptz not null)")
            .execute(&pool).await.unwrap();
        sqlx::query("create table sku_boms (tenant_id uuid not null, bundle_sku_id uuid not null, component_sku_id uuid not null, qty bigint not null)")
            .execute(&pool).await.unwrap();
        let tenant = TenantId(Uuid::from_u128(1));
        let product = Uuid::from_u128(10);
        let other_product = Uuid::from_u128(20);
        sqlx::query(
            "insert into products values ($1,$3,'ACTIVE'),($2,$3,'ACTIVE'),($4,$5,'ACTIVE')",
        )
        .bind(product)
        .bind(other_product)
        .bind(Uuid::from(tenant))
        .bind(Uuid::from_u128(30))
        .bind(Uuid::from_u128(2))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("insert into skus values ($1,$5,$6,'A','100%_literal','needle name','NORMAL',12345,true,'ACTIVE','{}','2026-09-10 04:34:56+00','2026-09-10 04:34:57.123456+00'), ($2,$5,$6,'B',null,'plain','KIT',null,false,'ACTIVE','{}','2026-09-10 04:34:58.123+00','2026-09-10 04:34:59.120+00'), ($3,$5,$6,'C',null,'last needle','NORMAL',500,true,'ACTIVE','{}','2026-09-10 04:35:00+00','2026-09-10 04:35:01+00'), ($4,$5,$7,'D',null,'needle other parent','NORMAL',1,true,'ACTIVE','{}',now(),now()), ($8,$9,$10,'E',null,'needle other tenant','NORMAL',1,true,'ACTIVE','{}',now(),now()), ($11,$5,$6,'F',null,'needle deleted','NORMAL',1,true,'DELETED','{}',now(),now())")
            .bind(Uuid::from_u128(11)).bind(Uuid::from_u128(12)).bind(Uuid::from_u128(13))
            .bind(Uuid::from_u128(14)).bind(Uuid::from(tenant)).bind(product).bind(other_product)
            .bind(Uuid::from_u128(15)).bind(Uuid::from_u128(2)).bind(Uuid::from_u128(30))
            .bind(Uuid::from_u128(16)).execute(&pool).await.unwrap();
        sqlx::query("insert into skus values ($1,$2,$3,'COMP',null,'component','NORMAL',3000,true,'ACTIVE','{}',now(),now())")
            .bind(Uuid::from_u128(40)).bind(Uuid::from(tenant)).bind(other_product)
            .execute(&pool).await.unwrap();
        sqlx::query("insert into sku_boms values ($1,$2,$3,4)")
            .bind(Uuid::from(tenant))
            .bind(Uuid::from_u128(12))
            .bind(Uuid::from_u128(40))
            .execute(&pool)
            .await
            .unwrap();

        let mut input = request("needle", vec![ColumnId(2), ColumnId(0)]);
        let first = find(&pool, tenant, product, &input).await.unwrap().unwrap();
        assert_eq!(
            (first.row_id, first.column, first.offset, first.wrapped),
            (Uuid::from_u128(11), ColumnId(2), 0, false)
        );
        input.after = Some(FindAnchor {
            row_id: first.row_id,
            column: first.column,
        });
        let second = find(&pool, tenant, product, &input).await.unwrap().unwrap();
        assert_eq!(
            (second.row_id, second.offset, second.wrapped),
            (Uuid::from_u128(13), 2, false)
        );
        input.after = Some(FindAnchor {
            row_id: second.row_id,
            column: second.column,
        });
        let wrapped = find(&pool, tenant, product, &input).await.unwrap().unwrap();
        assert_eq!(
            (wrapped.row_id, wrapped.wrapped),
            (Uuid::from_u128(11), true)
        );
        input.backward = true;
        input.after = Some(FindAnchor {
            row_id: Uuid::from_u128(11),
            column: ColumnId(2),
        });
        let backward = find(&pool, tenant, product, &input).await.unwrap().unwrap();
        assert_eq!(
            (backward.row_id, backward.wrapped),
            (Uuid::from_u128(13), true)
        );

        assert_eq!(
            find(&pool, tenant, product, &request("100%_", vec![ColumnId(1)]))
                .await
                .unwrap()
                .unwrap()
                .row_id,
            Uuid::from_u128(11)
        );
        assert_eq!(
            find(
                &pool,
                tenant,
                product,
                &request("123.45", vec![ColumnId(4)])
            )
            .await
            .unwrap()
            .unwrap()
            .row_id,
            Uuid::from_u128(11)
        );
        assert_eq!(
            find(
                &pool,
                tenant,
                product,
                &request("120.00", vec![ColumnId(4)])
            )
            .await
            .unwrap()
            .unwrap()
            .row_id,
            Uuid::from_u128(12)
        );
        assert_eq!(
            find(&pool, tenant, product, &request("false", vec![ColumnId(5)]))
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
                product,
                &request(".123456Z", vec![ColumnId(8)])
            )
            .await
            .unwrap()
            .unwrap()
            .row_id,
            Uuid::from_u128(11)
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
