//! Transaction-scoped, bounded streaming export for the product grid query.
use erp_grid_core::paged_query::PageQuery;
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use crate::product_grid::{ProductGridRow, append_ordering, predicates, validate};

pub const MAX_EXPORT_ROWS: u64 = 1_000_000;
pub const MAX_FETCH_ROWS: u16 = 1_000;

fn validate_fetch_rows(rows: u16) -> anyhow::Result<()> {
    if !(1..=MAX_FETCH_ROWS).contains(&rows) {
        anyhow::bail!("Product export fetch size must be between 1 and {MAX_FETCH_ROWS}");
    }
    Ok(())
}

/// A PostgreSQL server cursor backed by one repeatable-read, read-only transaction.
/// Dropping this value drops the transaction, which queues a rollback on the connection.
pub struct ProductExportCursor {
    tx: Option<Transaction<'static, Postgres>>,
    cursor_name: String,
    total: u64,
}

impl ProductExportCursor {
    pub async fn open(pool: &PgPool, tenant: TenantId, query: &PageQuery) -> anyhow::Result<Self> {
        validate(query).map_err(anyhow::Error::msg)?;

        let mut tx = pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read, read only")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local statement_timeout = '10s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local idle_in_transaction_session_timeout = '30s'")
            .execute(&mut *tx)
            .await?;

        // Stop counting after the first disallowed row. For accepted exports this is
        // still the exact matching count and uses the same transaction snapshot.
        let mut count = QueryBuilder::<Postgres>::new("select count(*) from (select 1");
        predicates(&mut count, tenant, query)?;
        count
            .push(" limit ")
            .push_bind((MAX_EXPORT_ROWS + 1) as i64);
        count.push(") as bounded_product_export_count");
        let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
        let total = u64::try_from(total)?;
        if total > MAX_EXPORT_ROWS {
            anyhow::bail!("Product export exceeds {MAX_EXPORT_ROWS} matching rows");
        }

        let cursor_name = format!("product_export_{}", uuid::Uuid::new_v4().simple());
        let mut declaration = QueryBuilder::<Postgres>::new("declare ");
        declaration
            .push(&cursor_name)
            .push(" no scroll cursor for select product_id, tenant_id, product_code, name, brand, category, status, created_at, updated_at");
        predicates(&mut declaration, tenant, query)?;
        append_ordering(&mut declaration, query)?;
        declaration.build().execute(&mut *tx).await?;

        Ok(Self {
            tx: Some(tx),
            cursor_name,
            total,
        })
    }

    /// Streams the durable rows captured when an actor-bound selection export
    /// job was accepted. Selection-token expiry cannot change this membership.
    pub async fn open_selection(
        pool: &PgPool,
        tenant: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Self> {
        let mut tx = pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read, read only")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local statement_timeout = '10s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local idle_in_transaction_session_timeout = '30s'")
            .execute(&mut *tx)
            .await?;

        let expected_total: Option<i64> = sqlx::query_scalar(
            "select s.expected_total from export_jobs j join grid_product_selection_exports s on s.job_id=j.job_id where j.job_id=$1 and j.tenant_id=$2 and j.job_type='PRODUCT_EXPORT' and j.status='RUNNING'",
        )
        .bind(job_id)
        .bind(uuid::Uuid::from(tenant))
        .fetch_optional(&mut *tx)
        .await?;
        let Some(expected_total) = expected_total else {
            anyhow::bail!("Product selection export job is unavailable");
        };
        let total: i64 = sqlx::query_scalar(
            "select count(*) from grid_product_selection_export_items where job_id=$1",
        )
        .bind(job_id)
        .fetch_one(&mut *tx)
        .await?;
        let total = u64::try_from(total)?;
        if total != u64::try_from(expected_total)? {
            anyhow::bail!("Product selection export held membership is incomplete");
        }
        if total > MAX_EXPORT_ROWS {
            anyhow::bail!("Product export exceeds {MAX_EXPORT_ROWS} held rows");
        }

        let cursor_name = format!("product_selection_export_{}", uuid::Uuid::new_v4().simple());
        let mut declaration = QueryBuilder::<Postgres>::new("declare ");
        declaration.push(&cursor_name).push(
            " no scroll cursor for select product_id, (data->>'tenant_id')::uuid as tenant_id, data->>'product_code' as product_code, data->>'name' as name, data->>'brand' as brand, data->>'category' as category, data->>'status' as status, (data->>'created_at')::timestamptz as created_at, (data->>'updated_at')::timestamptz as updated_at from grid_product_selection_export_items where job_id=",
        );
        declaration.push_bind(job_id).push(" order by ordinal");
        declaration.build().execute(&mut *tx).await?;
        Ok(Self {
            tx: Some(tx),
            cursor_name,
            total,
        })
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub async fn fetch_next(&mut self, rows: u16) -> anyhow::Result<Vec<ProductGridRow>> {
        validate_fetch_rows(rows)?;
        let tx = self
            .tx
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Product export cursor is finished"))?;
        // The count is validated before interpolation and the identifier is generated locally.
        let sql = format!("fetch forward {rows} from {}", self.cursor_name);
        Ok(sqlx::query_as::<_, ProductGridRow>(&sql)
            .fetch_all(&mut **tx)
            .await?)
    }

    pub async fn finish(mut self) -> anyhow::Result<()> {
        if let Some(tx) = self.tx.take() {
            tx.rollback().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{ColumnId, Filter, SortRule, paged_query::FilterExpression};
    use std::collections::BTreeMap;

    fn nested_query() -> PageQuery {
        PageQuery {
            schema: 1,
            offset: 42_000,
            limit: 25,
            filters: BTreeMap::from([(ColumnId(4), Filter::Enum(["ACTIVE".into()].into()))]),
            sorts: vec![SortRule {
                column: ColumnId(1),
                descending: false,
            }],
            search: Some("needle".into()),
            expression: Some(FilterExpression::And(vec![
                FilterExpression::Condition {
                    column: ColumnId(2),
                    filter: Filter::Text("brand".into()),
                },
                FilterExpression::Or(vec![
                    FilterExpression::Condition {
                        column: ColumnId(0),
                        filter: Filter::Text("A".into()),
                    },
                    FilterExpression::Condition {
                        column: ColumnId(0),
                        filter: Filter::Text("B".into()),
                    },
                ]),
            ])),
        }
    }

    #[test]
    fn export_reuses_product_predicates_and_stable_order_without_page_scan() {
        let query = nested_query();
        validate(&query).unwrap();
        let tenant = TenantId(uuid::Uuid::nil());
        let mut export = QueryBuilder::<Postgres>::new("select product_id");
        predicates(&mut export, tenant, &query).unwrap();
        append_ordering(&mut export, &query).unwrap();
        let sql = export.sql();
        assert!(sql.contains("status <> 'DELETED'"));
        assert!(sql.contains(" and (coalesce(brand"));
        assert!(sql.contains(" or coalesce(product_code"));
        assert!(sql.contains("lower(nullif(name, '')) asc nulls last, product_id desc"));
        assert!(!sql.contains(" offset "));
        assert!(!sql.contains(" limit "));
    }

    #[test]
    fn export_fetch_size_rejects_zero_and_over_limit() {
        assert_eq!(MAX_EXPORT_ROWS, 1_000_000);
        assert!(validate_fetch_rows(1).is_ok());
        assert!(validate_fetch_rows(MAX_FETCH_ROWS).is_ok());
        assert!(validate_fetch_rows(0).is_err());
        assert!(validate_fetch_rows(MAX_FETCH_ROWS + 1).is_err());
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates and drops an isolated schema"]
    async fn postgres_cursor_binds_filters_streams_batches_and_keeps_snapshot() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let admin = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("product_export_test_{}", uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("create schema {schema}"))
            .execute(&admin)
            .await
            .unwrap();
        sqlx::query(&format!(
            "create table {schema}.products (product_id uuid primary key, tenant_id uuid not null, product_code text, name text not null, alias text, primary_barcode text, brand text, category text, status text not null, created_at timestamptz not null, updated_at timestamptz not null)"
        ))
        .execute(&admin)
        .await
        .unwrap();

        let connection_schema = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .after_connect(move |connection, _| {
                let statement = format!("set search_path to {connection_schema}");
                Box::pin(async move {
                    sqlx::query(&statement).execute(connection).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        sqlx::query("insert into products select lpad(to_hex(n), 32, '0')::uuid, $1, 'SKU-' || lpad(n::text, 5, '0'), 'Product ' || lpad(n::text, 5, '0'), null, null, 'Acme', 'Goods', 'ACTIVE', now(), now() from generate_series(1, 1001) n")
            .bind(uuid::Uuid::from(tenant))
            .execute(&pool)
            .await
            .unwrap();
        let query = nested_query();
        let mut query = PageQuery {
            offset: 900_000,
            search: Some("Product".into()),
            expression: Some(FilterExpression::And(vec![
                FilterExpression::Condition {
                    column: ColumnId(2),
                    filter: Filter::Enum(["Acme".into()].into()),
                },
                FilterExpression::Or(vec![
                    FilterExpression::Condition {
                        column: ColumnId(4),
                        filter: Filter::Enum(["ACTIVE".into()].into()),
                    },
                    FilterExpression::Condition {
                        column: ColumnId(0),
                        filter: Filter::Text("never".into()),
                    },
                ]),
            ])),
            ..query
        };
        query.filters.clear();
        let mut cursor = ProductExportCursor::open(&pool, tenant, &query)
            .await
            .unwrap();
        assert_eq!(cursor.total(), 1001);

        // This matching row commits after OPEN. Repeatable read keeps both count and
        // declared cursor on the original snapshot.
        sqlx::query("insert into products values ($1, $2, 'SKU-99999', 'Product 99999', null, null, 'Acme', 'Goods', 'ACTIVE', now(), now())")
            .bind(uuid::Uuid::from_u128(99999))
            .bind(uuid::Uuid::from(tenant))
            .execute(&pool)
            .await
            .unwrap();
        let first = cursor.fetch_next(1000).await.unwrap();
        let second = cursor.fetch_next(1000).await.unwrap();
        assert_eq!((first.len(), second.len()), (1000, 1));
        assert_eq!(first[0].name, "Product 00001");
        assert_eq!(second[0].name, "Product 01001");
        assert!(cursor.fetch_next(1001).await.is_err());
        cursor.finish().await.unwrap();

        pool.close().await;
        sqlx::query(&format!("drop schema {schema} cascade"))
            .execute(&admin)
            .await
            .unwrap();
        admin.close().await;
    }
}
