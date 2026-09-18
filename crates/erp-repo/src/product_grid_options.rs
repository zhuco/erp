//! Distinct filter options for the product grid.
//!
//! The option query deliberately reuses the product-grid predicate contract,
//! while removing the option column's own filter so a selected value does not
//! disappear from its own dropdown.

use erp_grid_core::{ColumnId, paged_query::PageQuery};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder};

const MAX_LIMIT: u32 = 100;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionQuery {
    pub query: PageQuery,
    pub column: ColumnId,
    pub search: Option<String>,
    pub offset: u64,
    pub limit: u32,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct OptionPage {
    pub values: Vec<String>,
    pub total: u64,
}

fn field(column: ColumnId) -> Result<&'static str, &'static str> {
    match column.0 {
        2 => Ok("brand"),
        3 => Ok("category"),
        4 => Ok("status"),
        _ => Err("Options are only available for brand, category, and status"),
    }
}

fn contains_pattern(text: &str) -> String {
    format!(
        "%{}%",
        text.replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
    )
}

pub fn validate(input: &OptionQuery) -> anyhow::Result<&'static str> {
    if input.limit == 0 || input.limit > MAX_LIMIT {
        anyhow::bail!("Option limit must be between 1 and 100");
    }
    if input.offset > i64::MAX as u64 - u64::from(input.limit) {
        anyhow::bail!("Invalid option page range");
    }
    crate::product_grid::validate(&input.query).map_err(anyhow::Error::msg)?;
    let name = field(input.column).map_err(anyhow::Error::msg)?;
    if input
        .search
        .as_ref()
        .is_some_and(|value| value.len() > 4096 || value.contains('\0'))
    {
        anyhow::bail!("Invalid option search");
    }
    Ok(name)
}

fn predicates_without_option(
    qb: &mut QueryBuilder<'_, Postgres>,
    tenant: TenantId,
    input: &OptionQuery,
) -> anyhow::Result<()> {
    let mut query = input.query.clone();
    query.filters.remove(&input.column);
    crate::product_grid::predicates(qb, tenant, &query)?;
    if let Some(search) = input
        .search
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" and coalesce(")
            .push(field(input.column).map_err(anyhow::Error::msg)?)
            .push(", '') ilike ")
            .push_bind(contains_pattern(search.trim()))
            .push(" escape '!'");
    }
    Ok(())
}

pub async fn query_options(
    pool: &PgPool,
    tenant: TenantId,
    input: &OptionQuery,
) -> anyhow::Result<OptionPage> {
    let name = validate(input)?;
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;

    let mut count =
        QueryBuilder::<Postgres>::new("select count(*) from (select distinct coalesce(");
    count.push(name).push(", '') as option_value");
    predicates_without_option(&mut count, tenant, input)?;
    count.push(") options");
    let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;

    let mut page =
        QueryBuilder::<Postgres>::new("select option_value from (select distinct coalesce(");
    page.push(name).push(", '') as option_value");
    predicates_without_option(&mut page, tenant, input)?;
    page.push(") options order by lower(option_value), option_value limit ")
        .push_bind(i64::from(input.limit))
        .push(" offset ")
        .push_bind(input.offset as i64);
    let values: Vec<String> = page.build_query_scalar().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(OptionPage {
        values,
        total: u64::try_from(total)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::Filter;
    use std::collections::BTreeMap;

    #[test]
    fn product_options_validate_column_limit_and_target_filter_removal() {
        let query = PageQuery {
            schema: 1,
            offset: 0,
            limit: 100,
            filters: BTreeMap::from([(ColumnId(2), Filter::Text("Nike".into()))]),
            sorts: vec![],
            search: None,
            expression: None,
        };
        let input = OptionQuery {
            query,
            column: ColumnId(2),
            search: Some("%_".into()),
            offset: 0,
            limit: 100,
        };
        assert_eq!(validate(&input).unwrap(), "brand");
        let mut qb = QueryBuilder::<Postgres>::new("select *");
        predicates_without_option(&mut qb, TenantId(uuid::Uuid::nil()), &input).unwrap();
        assert!(!qb.sql().contains("Nike"));
        assert!(qb.sql().contains("coalesce(brand, '') ilike"));
        assert!(field(ColumnId(1)).is_err());
        assert!(field(ColumnId(5)).is_err());
        let mut invalid = input.clone();
        invalid.query.schema = 2;
        assert!(validate(&invalid).is_err());
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; uses a connection-local temporary table"]
    async fn product_options_are_distinct_tenant_scoped_and_null_explicit() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table products (product_id uuid primary key, tenant_id uuid not null, product_code text, name text not null, alias text, primary_barcode text, brand text, category text, status text not null, created_at timestamptz not null, updated_at timestamptz not null)").execute(&pool).await.unwrap();
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        sqlx::query("insert into products values ($1,$2,'A','A',null,null,null,'Tools','ACTIVE',now(),now()), ($3,$4,'B','B',null,null,'Acme','Tools','ACTIVE',now(),now()), ($5,$6,'C','C',null,null,'Acme','Other','ACTIVE',now(),now())")
            .bind(uuid::Uuid::from_u128(1)).bind(uuid::Uuid::from(tenant))
            .bind(uuid::Uuid::from_u128(2)).bind(uuid::Uuid::from(tenant))
            .bind(uuid::Uuid::from_u128(3)).bind(uuid::Uuid::from_u128(2)).execute(&pool).await.unwrap();
        let input = OptionQuery {
            query: PageQuery {
                schema: 1,
                offset: 0,
                limit: 100,
                filters: BTreeMap::from([(ColumnId(2), Filter::Text("Acme".into()))]),
                sorts: vec![],
                search: None,
                expression: None,
            },
            column: ColumnId(2),
            search: None,
            offset: 0,
            limit: 100,
        };
        let result = query_options(&pool, tenant, &input).await.unwrap();
        assert_eq!(result.values, vec!["", "Acme"]);
        assert_eq!(result.total, 2);
        sqlx::query("insert into products select md5('options-' || n::text)::uuid, $1, n::text, n::text, null, null, 'Brand' || n::text, 'Bulk', 'ACTIVE', now(), now() from generate_series(1,130) n")
            .bind(uuid::Uuid::from(tenant)).execute(&pool).await.unwrap();
        let mut paged = input.clone();
        paged.offset = 100;
        let later = query_options(&pool, tenant, &paged).await.unwrap();
        assert_eq!((later.total, later.values.len()), (132, 32));
        paged.offset = 0;
        paged.search = Some(" Brand1 ".into());
        let searched = query_options(&pool, tenant, &paged).await.unwrap();
        assert_eq!((searched.total, searched.values.len()), (42, 42));
        paged.search = None;
        paged
            .query
            .filters
            .insert(ColumnId(3), Filter::Text("Tools".into()));
        assert_eq!(query_options(&pool, tenant, &paged).await.unwrap().total, 2);
        pool.close().await;
    }
}
