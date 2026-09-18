use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::BTreeSet;

const SCHEMA: u32 = 1;
const MAX_BUCKETS: usize = 4096;
const MAX_KEY_BYTES: usize = 4096;
const MAX_KIND_BYTES: usize = 64;
const MAX_TEXT_BUDGET: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Measure {
    Quantity,
    Cost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryAggregateQuery {
    pub schema: u32,
    pub warehouse_id: Option<uuid::Uuid>,
    pub search: Option<String>,
    pub by_category: bool,
    pub measure: Measure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub count: u64,
    pub sum: String,
    pub min: Option<String>,
    pub max: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bucket {
    pub warehouse_code: String,
    pub category: Option<String>,
    pub kind: String,
    pub summary: Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryAggregateReport {
    pub buckets: Vec<Bucket>,
    pub total: Summary,
    pub as_of: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{0} inventory rows have no authoritative cost")]
    MissingCost(u64),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

pub fn validate(query: &InventoryAggregateQuery) -> Result<(), Error> {
    if query.schema != SCHEMA {
        return Err(Error::Invalid(
            "unsupported inventory aggregate schema".into(),
        ));
    }
    if query.warehouse_id.is_some_and(|id| id.is_nil()) {
        return Err(Error::Invalid("warehouse_id must not be nil".into()));
    }
    if let Some(search) = &query.search {
        if search.contains('\0') || search.chars().count() > 256 || search.len() > 1024 {
            return Err(Error::Invalid(
                "search must be at most 256 characters and 1024 bytes and contain no NUL".into(),
            ));
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct AggregateRow {
    warehouse_code: Option<String>,
    category: Option<String>,
    kind: Option<String>,
    row_count: i64,
    amount_sum: String,
    amount_min: Option<String>,
    amount_max: Option<String>,
    missing_cost_count: i64,
    is_total: bool,
    as_of: DateTime<Utc>,
}

pub async fn aggregate(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    query: &InventoryAggregateQuery,
    warehouse_ids: Option<&[uuid::Uuid]>,
) -> Result<InventoryAggregateReport, Error> {
    validate(query)?;

    let cost = crate::sku_grid::cost_expression();
    let amount = match query.measure {
        Measure::Quantity => "b.on_hand::numeric".to_string(),
        Measure::Cost => format!("b.on_hand::numeric * ({cost})::numeric"),
    };
    let missing_cost = match query.measure {
        Measure::Quantity => "false".to_string(),
        Measure::Cost => format!(
            "s.sku_id is null or ({cost}) is null or (s.sku_kind = 'KIT' and exists (select 1 from sku_boms mb left join skus mc on mc.tenant_id = mb.tenant_id and mc.sku_id = mb.component_sku_id where mb.tenant_id = s.tenant_id and mb.bundle_sku_id = s.sku_id and (mc.sku_id is null or mc.cost_cent is null)))"
        ),
    };

    let mut qb = QueryBuilder::<Postgres>::new(
        "with source as (select w.code as warehouse_code, case when ",
    );
    qb.push_bind(query.by_category)
        .push(" then p.category else null end as category, coalesce(s.sku_kind, 'MISSING_SKU') as kind, ")
        .push(amount)
        .push(" as amount, ")
        .push(missing_cost)
        .push(" as missing_cost from inventory_balances b join warehouses w on w.tenant_id = b.tenant_id and w.warehouse_id = b.warehouse_id left join skus s on s.tenant_id = b.tenant_id and s.sku_id = b.sku_id left join products p on p.tenant_id = s.tenant_id and p.product_id = s.product_id where b.tenant_id = ")
        .push_bind(tenant_id);
    if let Some(warehouse_id) = query.warehouse_id {
        qb.push(" and b.warehouse_id = ").push_bind(warehouse_id);
    }
    if let Some(ids) = warehouse_ids {
        qb.push(" and b.warehouse_id = any(")
            .push_bind(ids)
            .push(")");
    }
    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        qb.push(" and (coalesce(s.sku_code, '') ilike ")
            .push_bind(literal_contains(search))
            .push(" escape '!' or coalesce(p.name, '') ilike ")
            .push_bind(literal_contains(search))
            .push(" escape '!')");
    }
    qb.push(
        ") select warehouse_code, category, kind, count(*)::bigint as row_count, coalesce(sum(amount), 0)::text as amount_sum, min(amount)::text as amount_min, max(amount)::text as amount_max, count(*) filter (where missing_cost)::bigint as missing_cost_count, grouping(warehouse_code, category, kind) <> 0 as is_total, statement_timestamp() as as_of from source group by grouping sets ((warehouse_code, category, kind), ()) order by is_total desc, warehouse_code, category nulls first, kind limit 4098",
    );

    let mut tx = pool.begin().await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    let rows: Vec<AggregateRow> = qb.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let missing = rows
        .iter()
        .map(|row| row.missing_cost_count)
        .max()
        .unwrap_or(0);
    let missing =
        u64::try_from(missing).map_err(|_| Error::Invalid("negative missing-cost count".into()))?;
    if missing != 0 {
        return Err(Error::MissingCost(missing));
    }

    let mut buckets = Vec::new();
    let mut total = None;
    let mut as_of = None;
    let mut text_bytes = 0usize;
    for row in rows {
        as_of.get_or_insert(row.as_of);
        let summary = row_summary(&row)?;
        if row.is_total {
            total = Some(summary);
        } else {
            let warehouse_code = row
                .warehouse_code
                .ok_or_else(|| Error::Invalid("aggregate bucket has no warehouse code".into()))?;
            let kind = row
                .kind
                .ok_or_else(|| Error::Invalid("aggregate bucket has no kind".into()))?;
            validate_output_text(&warehouse_code, MAX_KEY_BYTES, "warehouse code")?;
            validate_output_text(&kind, MAX_KIND_BYTES, "SKU kind")?;
            if let Some(category) = &row.category {
                validate_output_text(category, MAX_KEY_BYTES, "category")?;
            }
            text_bytes = text_bytes
                .checked_add(warehouse_code.len())
                .and_then(|size| size.checked_add(kind.len()))
                .and_then(|size| size.checked_add(row.category.as_ref().map_or(0, String::len)))
                .ok_or_else(|| Error::Invalid("aggregate text budget overflow".into()))?;
            if text_bytes > MAX_TEXT_BUDGET {
                return Err(Error::Invalid("aggregate text exceeds 8 MiB".into()));
            }
            buckets.push(Bucket {
                warehouse_code,
                category: row.category,
                kind,
                summary,
            });
        }
    }
    if buckets.len() > MAX_BUCKETS {
        return Err(Error::Invalid(
            "inventory aggregate exceeds 4096 buckets".into(),
        ));
    }
    if buckets
        .iter()
        .map(|bucket| bucket.kind.as_str())
        .collect::<BTreeSet<_>>()
        .len()
        > 64
    {
        return Err(Error::Invalid(
            "inventory aggregate exceeds 64 SKU kinds".into(),
        ));
    }
    let report = InventoryAggregateReport {
        buckets,
        total: total.ok_or_else(|| Error::Invalid("aggregate total is missing".into()))?,
        as_of: as_of.unwrap_or_else(Utc::now),
    };
    let encoded = serde_json::to_vec(&report)
        .map_err(|_| Error::Invalid("inventory aggregate response cannot be encoded".into()))?;
    if encoded.len() > MAX_TEXT_BUDGET {
        return Err(Error::Invalid(
            "inventory aggregate response exceeds 8 MiB".into(),
        ));
    }
    Ok(report)
}

fn validate_output_text(value: &str, max: usize, label: &str) -> Result<(), Error> {
    if value.contains('\0') || value.len() > max {
        return Err(Error::Invalid(format!("{label} is invalid or too long")));
    }
    Ok(())
}

fn row_summary(row: &AggregateRow) -> Result<Summary, Error> {
    Ok(Summary {
        count: u64::try_from(row.row_count)
            .map_err(|_| Error::Invalid("negative aggregate count".into()))?,
        sum: integer(&row.amount_sum)?,
        min: row.amount_min.as_deref().map(integer).transpose()?,
        max: row.amount_max.as_deref().map(integer).transpose()?,
    })
}

fn integer(value: &str) -> Result<String, Error> {
    value
        .parse::<i128>()
        .map(|value| value.to_string())
        .map_err(|_| Error::Invalid("aggregate value exceeds signed 128-bit integer".into()))
}

fn literal_contains(value: &str) -> String {
    let escaped = value
        .replace('!', "!!")
        .replace('%', "!%")
        .replace('_', "!_");
    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> InventoryAggregateQuery {
        InventoryAggregateQuery {
            schema: 1,
            warehouse_id: None,
            search: None,
            by_category: true,
            measure: Measure::Quantity,
        }
    }

    #[test]
    fn validation_bounds_search_and_schema() {
        assert!(validate(&query()).is_ok());
        let mut bad = query();
        bad.schema = 2;
        assert!(matches!(validate(&bad), Err(Error::Invalid(_))));
        bad.schema = 1;
        bad.warehouse_id = Some(uuid::Uuid::nil());
        assert!(matches!(validate(&bad), Err(Error::Invalid(_))));
        bad.warehouse_id = None;
        bad.search = Some("x".repeat(257));
        assert!(matches!(validate(&bad), Err(Error::Invalid(_))));
        bad.search = Some("\0".into());
        assert!(matches!(validate(&bad), Err(Error::Invalid(_))));
        assert_eq!(literal_contains("a%b_c!"), "%a!%b!_c!!%");
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; uses connection-local temporary tables"]
    async fn pg_aggregate_enforces_scope_totals_costs_and_null_category() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        for ddl in [
            "create temporary table warehouses (warehouse_id uuid primary key, tenant_id uuid not null, code text not null)",
            "create temporary table products (product_id uuid primary key, tenant_id uuid not null, name text, category text)",
            "create temporary table skus (sku_id uuid primary key, tenant_id uuid not null, product_id uuid, sku_code text, sku_kind text not null, cost_cent bigint)",
            "create temporary table sku_boms (tenant_id uuid not null, bundle_sku_id uuid not null, component_sku_id uuid not null, qty bigint not null)",
            "create temporary table inventory_balances (tenant_id uuid not null, warehouse_id uuid not null, sku_id uuid not null, on_hand bigint not null, reserved bigint not null default 0)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        let tenant = uuid::Uuid::from_u128(1);
        let other = uuid::Uuid::from_u128(2);
        let w1 = uuid::Uuid::from_u128(11);
        let w2 = uuid::Uuid::from_u128(12);
        let p1 = uuid::Uuid::from_u128(21);
        let p2 = uuid::Uuid::from_u128(22);
        let normal = uuid::Uuid::from_u128(31);
        let component = uuid::Uuid::from_u128(32);
        let kit = uuid::Uuid::from_u128(33);
        let missing = uuid::Uuid::from_u128(34);
        let deleted_sku = uuid::Uuid::from_u128(39);
        sqlx::query("insert into warehouses values ($1,$2,'A'),($3,$2,'B'),($4,$5,'OTHER')")
            .bind(w1)
            .bind(tenant)
            .bind(w2)
            .bind(uuid::Uuid::from_u128(13))
            .bind(other)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("insert into products values ($1,$3,'Named','Cat'),($2,$3,'No category',null)")
            .bind(p1)
            .bind(p2)
            .bind(tenant)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("insert into skus values ($1,$5,$6,'N','NORMAL',7),($2,$5,$6,'C','NORMAL',3),($3,$5,$7,'K','KIT',999),($4,$5,$7,'M','NORMAL',null)")
            .bind(normal).bind(component).bind(kit).bind(missing).bind(tenant).bind(p1).bind(p2)
            .execute(&pool).await.unwrap();
        sqlx::query("insert into sku_boms values ($1,$2,$3,2)")
            .bind(tenant)
            .bind(kit)
            .bind(component)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("insert into inventory_balances values ($1,$2,$3,10,0),($1,$2,$4,4,0),($1,$5,$3,2,0),($1,$5,$6,1,0),($1,$5,$9,3,0),($7,$8,$3,999,0)")
            .bind(tenant).bind(w1).bind(normal).bind(kit).bind(w2).bind(missing)
            .bind(other).bind(uuid::Uuid::from_u128(13)).bind(deleted_sku)
            .execute(&pool).await.unwrap();

        let quantity = aggregate(&pool, tenant, &query(), Some(&[w1]))
            .await
            .unwrap();
        assert_eq!(
            (quantity.total.count, quantity.total.sum.as_str()),
            (2, "14")
        );
        assert_eq!(quantity.buckets.len(), 2);
        let all = aggregate(&pool, tenant, &query(), None).await.unwrap();
        assert_eq!((all.total.count, all.total.sum.as_str()), (5, "20"));
        assert!(
            all.buckets
                .iter()
                .any(|bucket| bucket.kind == "MISSING_SKU")
        );
        assert!(all.buckets.iter().any(|bucket| bucket.category.is_none()));
        let empty = aggregate(&pool, tenant, &query(), Some(&[])).await.unwrap();
        assert_eq!((empty.total.count, empty.total.sum.as_str()), (0, "0"));
        assert!(empty.buckets.is_empty());

        sqlx::query("delete from inventory_balances where sku_id = $1")
            .bind(deleted_sku)
            .execute(&pool)
            .await
            .unwrap();

        let mut cost_query = query();
        cost_query.measure = Measure::Cost;
        assert!(matches!(
            aggregate(&pool, tenant, &cost_query, None).await,
            Err(Error::MissingCost(1))
        ));
        sqlx::query("update skus set cost_cent = 5 where sku_id = $1")
            .bind(missing)
            .execute(&pool)
            .await
            .unwrap();
        let literal_product = uuid::Uuid::from_u128(23);
        let precise_sku = uuid::Uuid::from_u128(35);
        sqlx::query("insert into products values ($1,$2,'Literal category','未分类')")
            .bind(literal_product)
            .bind(tenant)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("insert into skus values ($1,$2,$3,'P','NORMAL',9007199254740993)")
            .bind(precise_sku)
            .bind(tenant)
            .bind(literal_product)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("insert into inventory_balances values ($1,$2,$3,2,0)")
            .bind(tenant)
            .bind(w1)
            .bind(precise_sku)
            .execute(&pool)
            .await
            .unwrap();
        let cost = aggregate(&pool, tenant, &cost_query, None).await.unwrap();
        assert_eq!(cost.total.sum, "18014398509482099");
        assert!(cost.buckets.iter().any(|bucket| bucket.category.is_none()));
        assert!(
            cost.buckets
                .iter()
                .any(|bucket| bucket.category.as_deref() == Some("未分类"))
        );
    }
}
