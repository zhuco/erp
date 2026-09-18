//! Frozen product-grid membership for actor-bound all-matching selections.
//!
//! Snapshot membership and field values remain frozen. Pages use snapshot
//! ordinal order by default and can sort the frozen fields with a stable
//! `product_id` tie-breaker. The fingerprint is a non-security cache identity;
//! database tenant/actor binding is the authority.
use crate::product_grid;
use erp_grid_core::SortRule;
use erp_grid_core::paged_query::PageQuery;
use erp_types::{TenantId, UserId};
use serde::Serialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::BTreeSet;

const MAX_SNAPSHOT_ROWS: u64 = 100_000;
const MAX_ACTIVE_SNAPSHOTS: i64 = 4;
const MAX_EXCLUSIONS: usize = 10_000;

#[derive(Debug, Serialize)]
pub struct SelectionToken {
    pub token: uuid::Uuid,
    pub query_fingerprint: String,
    pub total: u64,
    pub expires_at: u64,
}

#[derive(Debug, Serialize)]
pub struct SelectionPage {
    pub data: Vec<serde_json::Value>,
    pub total: u64,
    pub token: uuid::Uuid,
    pub expires_at: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SelectionError {
    #[error("invalid selection request: {0}")]
    Invalid(String),
    #[error("selection is unavailable")]
    Unavailable,
    #[error("selection capacity exceeded")]
    Capacity,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

pub async fn freeze(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    query: &PageQuery,
) -> Result<SelectionToken, SelectionError> {
    match freeze_inner(pool, tenant, actor, query).await {
        Err(SelectionError::Storage(error)) if is_serialization_failure(&error) => {
            Err(SelectionError::Capacity)
        }
        result => result,
    }
}

async fn freeze_inner(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    query: &PageQuery,
) -> Result<SelectionToken, SelectionError> {
    let mut normalized = query.clone();
    normalized.offset = 0;
    normalized.limit = 100;
    normalized.sorts.clear();
    product_grid::validate(&normalized)
        .map_err(|message| SelectionError::Invalid(message.to_string()))?;

    let normalized_json = serde_json::to_value(&normalized)
        .map_err(|error| SelectionError::Invalid(error.to_string()))?;
    let tenant_id = uuid::Uuid::from(tenant);
    let actor_id = uuid::Uuid::from(actor);
    let token = uuid::Uuid::new_v4();

    let mut tx = pool.begin().await?;
    // SERIALIZABLE retains repeatable-read snapshot semantics and additionally
    // makes stale quota predicate reads fail rather than permit over-issuance.
    sqlx::query("set transaction isolation level serializable")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '30s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("select pg_advisory_xact_lock(hashtextextended($1::text || ':' || $2::text, 0))")
        .bind(tenant_id)
        .bind(actor_id)
        .execute(&mut *tx)
        .await?;

    // Cleanup is deliberately confined to the two selection tables; items
    // leave through the selection foreign key's ON DELETE CASCADE.
    sqlx::query("delete from grid_product_selections where expires_at <= clock_timestamp()")
        .execute(&mut *tx)
        .await?;

    let active: i64 = sqlx::query_scalar(
        "select count(*) from grid_product_selections where tenant_id = $1 and actor_user_id = $2",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await?;
    if active >= MAX_ACTIVE_SNAPSHOTS {
        return Err(SelectionError::Capacity);
    }

    let frozen_rows: i64 = sqlx::query_scalar(
        "select count(*) from grid_product_selection_items i join grid_product_selections s on s.token = i.selection_token where s.tenant_id = $1 and s.actor_user_id = $2",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await?;
    let mut count = QueryBuilder::<Postgres>::new("select count(*)");
    product_grid::predicates(&mut count, tenant, &normalized)
        .map_err(|error| SelectionError::Invalid(error.to_string()))?;
    let total_i64: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let total = u64::try_from(total_i64)
        .map_err(|_| SelectionError::Invalid("negative selection count".into()))?;
    let frozen_rows = u64::try_from(frozen_rows)
        .map_err(|_| SelectionError::Invalid("negative frozen row count".into()))?;
    if total > MAX_SNAPSHOT_ROWS
        || frozen_rows
            .checked_add(total)
            .is_none_or(|rows| rows > MAX_SNAPSHOT_ROWS)
    {
        return Err(SelectionError::Capacity);
    }

    let query_fingerprint: String = sqlx::query_scalar("select md5($1::jsonb::text)")
        .bind(&normalized_json)
        .fetch_one(&mut *tx)
        .await?;
    let expires_at_i64: i64 = sqlx::query_scalar(
        "insert into grid_product_selections(token, tenant_id, actor_user_id, normalized_query, query_fingerprint, total, expires_at) values ($1, $2, $3, $4, $5, $6, clock_timestamp() + interval '10 minutes') returning floor(extract(epoch from expires_at))::bigint",
    )
    .bind(token)
    .bind(tenant_id)
    .bind(actor_id)
    .bind(&normalized_json)
    .bind(&query_fingerprint)
    .bind(total_i64)
    .fetch_one(&mut *tx)
    .await?;

    let mut insert = QueryBuilder::<Postgres>::new(
        "insert into grid_product_selection_items(selection_token, ordinal, product_id, product_version, data) select ",
    );
    insert
        .push_bind(token)
        .push(", (row_number() over (order by product_id) - 1)::bigint, product_id, updated_at, jsonb_build_object('product_id', product_id, 'tenant_id', tenant_id, 'product_code', product_code, 'name', name, 'brand', brand, 'category', category, 'status', status, 'created_at', created_at, 'updated_at', updated_at)");
    product_grid::predicates(&mut insert, tenant, &normalized)
        .map_err(|error| SelectionError::Invalid(error.to_string()))?;
    insert.push(" order by product_id");
    let inserted = insert.build().execute(&mut *tx).await?.rows_affected();
    if inserted != total {
        return Err(SelectionError::Storage(sqlx::Error::Protocol(
            "selection snapshot count changed inside repeatable-read transaction".into(),
        )));
    }

    tx.commit().await?;
    Ok(SelectionToken {
        token,
        query_fingerprint,
        total,
        expires_at: u64::try_from(expires_at_i64)
            .map_err(|_| SelectionError::Invalid("invalid selection expiry".into()))?,
    })
}

fn is_serialization_failure(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.code().as_deref() == Some("40001"))
}

pub async fn page(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    token: uuid::Uuid,
    offset: u64,
    limit: u16,
    sorts: &[SortRule],
) -> Result<SelectionPage, SelectionError> {
    page_inner(pool, tenant, actor, token, offset, limit, sorts, false).await
}

pub async fn page_query_order(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    token: uuid::Uuid,
    offset: u64,
    limit: u16,
    sorts: &[SortRule],
) -> Result<SelectionPage, SelectionError> {
    page_inner(pool, tenant, actor, token, offset, limit, sorts, true).await
}

async fn page_inner(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    token: uuid::Uuid,
    offset: u64,
    limit: u16,
    sorts: &[SortRule],
    query_order: bool,
) -> Result<SelectionPage, SelectionError> {
    if !(1..=500).contains(&limit) || offset > i64::MAX as u64 - u64::from(limit) {
        return Err(SelectionError::Invalid("invalid page range".into()));
    }
    validate_sorts(sorts)?;
    let end = offset + u64::from(limit);
    let mut tx = pool.begin().await?;
    sqlx::query("set local statement_timeout = '30s'")
        .execute(&mut *tx)
        .await?;
    let header = header(&mut tx, tenant, actor, token).await?;
    let data = if sorts.is_empty() && !query_order {
        sqlx::query_scalar::<_, serde_json::Value>(
            "select data from grid_product_selection_items where selection_token = $1 and ordinal >= $2 and ordinal < $3 order by ordinal",
        )
        .bind(token)
        .bind(offset as i64)
        .bind(end as i64)
        .fetch_all(&mut *tx)
        .await?
    } else {
        let mut query = QueryBuilder::<Postgres>::new(
            "select data from grid_product_selection_items where selection_token = ",
        );
        query.push_bind(token).push(" order by ");
        if sorts.is_empty() {
            query.push("(data ->> 'created_at')::timestamptz desc nulls last, ");
        } else {
            for sort in sorts {
                query.push(selection_sort_expression(sort.column.0)?);
                query
                    .push(if sort.descending { " desc" } else { " asc" })
                    .push(" nulls last, ");
            }
        }
        query
            .push(if query_order {
                "product_id desc limit "
            } else {
                "product_id asc limit "
            })
            .push_bind(i64::from(limit))
            .push(" offset ")
            .push_bind(offset as i64);
        query
            .build_query_scalar::<serde_json::Value>()
            .fetch_all(&mut *tx)
            .await?
    };
    tx.commit().await?;
    Ok(SelectionPage {
        data,
        total: header.total,
        token,
        expires_at: header.expires_at,
    })
}

pub fn validate_sorts(sorts: &[SortRule]) -> Result<(), SelectionError> {
    if sorts.len() > 8 {
        return Err(SelectionError::Invalid("too many sort columns".into()));
    }
    let mut seen = BTreeSet::new();
    for sort in sorts {
        selection_sort_expression(sort.column.0)?;
        if !seen.insert(sort.column) {
            return Err(SelectionError::Invalid("duplicate sort column".into()));
        }
    }
    Ok(())
}

pub(crate) fn selection_sort_expression(column: u64) -> Result<&'static str, SelectionError> {
    match column {
        0 => Ok("lower(nullif(data ->> 'product_code', ''))"),
        1 => Ok("lower(nullif(data ->> 'name', ''))"),
        2 => Ok("lower(nullif(data ->> 'brand', ''))"),
        3 => Ok("lower(nullif(data ->> 'category', ''))"),
        4 => Ok("lower(nullif(data ->> 'status', ''))"),
        5 => Ok("(data ->> 'created_at')::timestamptz"),
        6 => Ok("(data ->> 'updated_at')::timestamptz"),
        _ => Err(SelectionError::Invalid(
            "unknown product selection sort column".into(),
        )),
    }
}

pub async fn affected_count(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    token: uuid::Uuid,
    excluded: &[uuid::Uuid],
) -> Result<u64, SelectionError> {
    if excluded.len() > MAX_EXCLUSIONS {
        return Err(SelectionError::Invalid("too many exclusions".into()));
    }
    let unique: BTreeSet<_> = excluded.iter().copied().collect();
    if unique.len() != excluded.len() || unique.iter().any(uuid::Uuid::is_nil) {
        return Err(SelectionError::Invalid(
            "exclusions must be unique, non-nil product IDs".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("set local statement_timeout = '30s'")
        .execute(&mut *tx)
        .await?;
    let header = header(&mut tx, tenant, actor, token).await?;
    if unique.is_empty() {
        tx.commit().await?;
        return Ok(header.total);
    }
    let ids: Vec<_> = unique.into_iter().collect();
    let members: i64 = sqlx::query_scalar(
        "select count(*) from grid_product_selection_items where selection_token = $1 and product_id = any($2)",
    )
    .bind(token)
    .bind(&ids)
    .fetch_one(&mut *tx)
    .await?;
    let members = usize::try_from(members)
        .map_err(|_| SelectionError::Invalid("invalid exclusion count".into()))?;
    if members != ids.len() {
        return Err(SelectionError::Invalid(
            "an excluded product is outside the frozen selection".into(),
        ));
    }
    let members = u64::try_from(members)
        .map_err(|_| SelectionError::Invalid("invalid exclusion count".into()))?;
    let affected = header
        .total
        .checked_sub(members)
        .ok_or_else(|| SelectionError::Invalid("exclusions exceed selection".into()))?;
    tx.commit().await?;
    Ok(affected)
}

pub(crate) struct Header {
    total: u64,
    expires_at: u64,
}

pub(crate) async fn header(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    token: uuid::Uuid,
) -> Result<Header, SelectionError> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "select total, floor(extract(epoch from expires_at))::bigint from grid_product_selections where token = $1 and tenant_id = $2 and actor_user_id = $3 and expires_at > clock_timestamp() for share",
    )
    .bind(token)
    .bind(uuid::Uuid::from(tenant))
    .bind(uuid::Uuid::from(actor))
    .fetch_optional(&mut **tx)
    .await?;
    let Some((total, expires_at)) = row else {
        return Err(SelectionError::Unavailable);
    };
    Ok(Header {
        total: u64::try_from(total)
            .map_err(|_| SelectionError::Invalid("invalid stored selection count".into()))?,
        expires_at: u64::try_from(expires_at)
            .map_err(|_| SelectionError::Invalid("invalid stored selection expiry".into()))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{ColumnId, SortRule};
    use std::collections::BTreeMap;

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates connection-local temporary tables only"]
    async fn product_selection_preserves_rows_and_enforces_binding_expiry_and_exclusions() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table products (product_id uuid primary key, tenant_id uuid not null, product_code text, name text not null, alias text, primary_barcode text, brand text, category text, status text not null, created_at timestamptz not null, updated_at timestamptz not null)").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selections (token uuid primary key, tenant_id uuid not null, actor_user_id uuid not null, normalized_query jsonb not null, query_fingerprint text not null, total bigint not null, expires_at timestamptz not null, created_at timestamptz not null default now())").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_items (selection_token uuid not null references grid_product_selections(token) on delete cascade, ordinal bigint not null, product_id uuid not null, product_version timestamptz not null, data jsonb not null, primary key(selection_token, ordinal), unique(selection_token, product_id))").execute(&pool).await.unwrap();
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        let actor = UserId(uuid::Uuid::from_u128(2));
        sqlx::query("insert into products values ($1, $3, 'A', 'First', null, null, 'Brand', 'Goods', 'ACTIVE', now(), now()), ($2, $3, 'B', 'Second', null, null, null, 'Goods', 'ACTIVE', now(), now())")
            .bind(uuid::Uuid::from_u128(10)).bind(uuid::Uuid::from_u128(11)).bind(uuid::Uuid::from(tenant)).execute(&pool).await.unwrap();
        let query = PageQuery {
            schema: 1,
            offset: 99,
            limit: 1,
            filters: BTreeMap::new(),
            sorts: vec![SortRule {
                column: ColumnId(0),
                descending: true,
            }],
            search: None,
            expression: None,
        };
        let frozen = freeze(&pool, tenant, actor, &query).await.unwrap();
        assert_eq!(frozen.total, 2);
        let normalized: serde_json::Value = sqlx::query_scalar(
            "select normalized_query from grid_product_selections where token = $1",
        )
        .bind(frozen.token)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(normalized["offset"], 0);
        assert_eq!(normalized["limit"], 100);
        assert_eq!(normalized["sorts"], serde_json::json!([]));

        sqlx::query("update products set name = 'Changed', status = 'DELETED'")
            .execute(&pool)
            .await
            .unwrap();
        let result = page(&pool, tenant, actor, frozen.token, 0, 500, &[])
            .await
            .unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.data.len(), 2);
        assert_eq!(result.data[0]["name"], "First");
        let keys = result.data[0]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            keys,
            BTreeSet::from([
                "brand".into(),
                "category".into(),
                "created_at".into(),
                "name".into(),
                "product_code".into(),
                "product_id".into(),
                "status".into(),
                "tenant_id".into(),
                "updated_at".into()
            ])
        );
        assert!(matches!(
            page(
                &pool,
                tenant,
                UserId(uuid::Uuid::from_u128(3)),
                frozen.token,
                0,
                1,
                &[]
            )
            .await,
            Err(SelectionError::Unavailable)
        ));
        assert!(matches!(
            page(
                &pool,
                TenantId(uuid::Uuid::from_u128(4)),
                actor,
                frozen.token,
                0,
                1,
                &[]
            )
            .await,
            Err(SelectionError::Unavailable)
        ));
        assert_eq!(
            affected_count(
                &pool,
                tenant,
                actor,
                frozen.token,
                &[uuid::Uuid::from_u128(10)]
            )
            .await
            .unwrap(),
            1
        );
        assert!(matches!(
            affected_count(
                &pool,
                tenant,
                actor,
                frozen.token,
                &[uuid::Uuid::from_u128(99)]
            )
            .await,
            Err(SelectionError::Invalid(_))
        ));
        sqlx::query("update grid_product_selections set expires_at = clock_timestamp() - interval '1 second' where token = $1")
            .bind(frozen.token).execute(&pool).await.unwrap();
        assert!(matches!(
            page(&pool, tenant, actor, frozen.token, 0, 1, &[]).await,
            Err(SelectionError::Unavailable)
        ));
        assert!(matches!(
            affected_count(&pool, tenant, actor, frozen.token, &[]).await,
            Err(SelectionError::Unavailable)
        ));
        // Issuance removes only expired derived snapshots and enforces the
        // per-scope active quota even when the new matches are empty.
        for _ in 0..MAX_ACTIVE_SNAPSHOTS {
            assert_eq!(freeze(&pool, tenant, actor, &query).await.unwrap().total, 0);
        }
        assert!(matches!(
            freeze(&pool, tenant, actor, &query).await,
            Err(SelectionError::Capacity)
        ));
        let other_tenant = TenantId(uuid::Uuid::from_u128(5));
        assert_eq!(
            freeze(&pool, other_tenant, actor, &query)
                .await
                .unwrap()
                .total,
            0
        );
        let expired_items: i64 = sqlx::query_scalar(
            "select count(*) from grid_product_selection_items where selection_token = $1",
        )
        .bind(frozen.token)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(expired_items, 0);
        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates connection-local temporary tables only"]
    async fn product_selection_sorting() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table products (product_id uuid primary key, tenant_id uuid not null, product_code text, name text not null, alias text, primary_barcode text, brand text, category text, status text not null, created_at timestamptz not null, updated_at timestamptz not null)").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selections (token uuid primary key, tenant_id uuid not null, actor_user_id uuid not null, normalized_query jsonb not null, query_fingerprint text not null, total bigint not null, expires_at timestamptz not null, created_at timestamptz not null default now())").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_items (selection_token uuid not null references grid_product_selections(token) on delete cascade, ordinal bigint not null, product_id uuid not null, product_version timestamptz not null, data jsonb not null, primary key(selection_token, ordinal), unique(selection_token, product_id))").execute(&pool).await.unwrap();

        let tenant = TenantId(uuid::Uuid::from_u128(101));
        let actor = UserId(uuid::Uuid::from_u128(102));
        let fixtures = [
            (
                10,
                None,
                "Alpha",
                None,
                Some("Group"),
                "ACTIVE",
                "2026-01-01T00:00:00Z",
                "2026-01-03T00:00:00Z",
            ),
            (
                11,
                Some("b"),
                "same",
                Some("Z"),
                Some("Group"),
                "active",
                "2026-01-02T00:00:00Z",
                "2026-01-02T00:00:00Z",
            ),
            (
                12,
                Some("A"),
                "Same",
                Some("Z"),
                Some("Group"),
                "PAUSED",
                "2026-01-02T00:00:00Z",
                "2026-01-01T00:00:00Z",
            ),
            (
                13,
                Some(""),
                "beta",
                Some("a"),
                Some("Other"),
                "",
                "2026-01-03T00:00:00Z",
                "2026-01-03T00:00:00Z",
            ),
            (
                14,
                Some("a"),
                "Same",
                Some("Z"),
                Some("Group"),
                "PAUSED",
                "2026-01-02T00:00:00Z",
                "2026-01-01T00:00:00Z",
            ),
            (
                15,
                Some("C"),
                "Zulu",
                None,
                None,
                "ACTIVE",
                "2026-01-04T00:00:00Z",
                "2026-01-04T00:00:00Z",
            ),
        ];
        for (id, code, name, brand, category, status, created, updated) in fixtures {
            sqlx::query("insert into products(product_id, tenant_id, product_code, name, alias, primary_barcode, brand, category, status, created_at, updated_at) values ($1, $2, $3, $4, null, null, $5, $6, $7, $8::timestamptz, $9::timestamptz)")
                .bind(uuid::Uuid::from_u128(id))
                .bind(uuid::Uuid::from(tenant))
                .bind(code)
                .bind(name)
                .bind(brand)
                .bind(category)
                .bind(status)
                .bind(created)
                .bind(updated)
                .execute(&pool)
                .await
                .unwrap();
        }
        let query = PageQuery {
            schema: 1,
            offset: 0,
            limit: 100,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
            expression: None,
        };
        let frozen = freeze(&pool, tenant, actor, &query).await.unwrap();
        sqlx::query("update products set name = 'Changed', product_code = 'changed'")
            .execute(&pool)
            .await
            .unwrap();

        fn ids(page: &SelectionPage) -> Vec<u128> {
            page.data
                .iter()
                .map(|row| {
                    uuid::Uuid::parse_str(row["product_id"].as_str().unwrap())
                        .unwrap()
                        .as_u128()
                })
                .collect()
        }

        let code_ascending = [SortRule {
            column: ColumnId(0),
            descending: false,
        }];
        let ascending = page(&pool, tenant, actor, frozen.token, 0, 500, &code_ascending)
            .await
            .unwrap();
        assert_eq!(ids(&ascending), [12, 14, 11, 15, 10, 13]);
        assert!(ascending.data.iter().all(|row| row["name"] != "Changed"));

        let query_default = page_query_order(&pool, tenant, actor, frozen.token, 0, 500, &[])
            .await
            .unwrap();
        assert_eq!(ids(&query_default), [15, 13, 14, 12, 11, 10]);
        let query_code = page_query_order(
            &pool,
            tenant,
            actor,
            frozen.token,
            0,
            500,
            &code_ascending,
        )
        .await
        .unwrap();
        assert_eq!(ids(&query_code), [14, 12, 11, 15, 13, 10]);

        let code_descending = [SortRule {
            column: ColumnId(0),
            descending: true,
        }];
        let descending = page(&pool, tenant, actor, frozen.token, 0, 500, &code_descending)
            .await
            .unwrap();
        assert_eq!(ids(&descending), [15, 11, 12, 14, 10, 13]);

        let statuses = page(
            &pool,
            tenant,
            actor,
            frozen.token,
            0,
            500,
            &[SortRule {
                column: ColumnId(4),
                descending: false,
            }],
        )
        .await
        .unwrap();
        assert_eq!(ids(&statuses), [10, 11, 15, 12, 14, 13]);

        let bounded = page(&pool, tenant, actor, frozen.token, 1, 2, &code_ascending)
            .await
            .unwrap();
        assert_eq!(ids(&bounded), [14, 11]);
        assert_eq!(bounded.data.len(), 2);

        let multi = [
            SortRule {
                column: ColumnId(3),
                descending: false,
            },
            SortRule {
                column: ColumnId(5),
                descending: true,
            },
            SortRule {
                column: ColumnId(6),
                descending: false,
            },
        ];
        let multi_page = page(&pool, tenant, actor, frozen.token, 0, 500, &multi)
            .await
            .unwrap();
        assert_eq!(ids(&multi_page), [12, 14, 11, 10, 13, 15]);

        let names = page(
            &pool,
            tenant,
            actor,
            frozen.token,
            0,
            500,
            &[SortRule {
                column: ColumnId(1),
                descending: false,
            }],
        )
        .await
        .unwrap();
        assert_eq!(names.data[0]["name"], "Alpha");
        assert_eq!(ids(&names), [10, 13, 11, 12, 14, 15]);

        pool.close().await;
        let duplicate = [
            SortRule {
                column: ColumnId(1),
                descending: false,
            },
            SortRule {
                column: ColumnId(1),
                descending: true,
            },
        ];
        assert!(matches!(
            page(&pool, tenant, actor, frozen.token, 0, 10, &duplicate).await,
            Err(SelectionError::Invalid(_))
        ));
        assert!(matches!(
            page(
                &pool,
                tenant,
                actor,
                frozen.token,
                0,
                10,
                &[SortRule {
                    column: ColumnId(7),
                    descending: false,
                }]
            )
            .await,
            Err(SelectionError::Invalid(_))
        ));
        let too_many = vec![
            SortRule {
                column: ColumnId(0),
                descending: false,
            };
            9
        ];
        assert!(matches!(
            page(&pool, tenant, actor, frozen.token, 0, 10, &too_many).await,
            Err(SelectionError::Invalid(_))
        ));
    }
}
