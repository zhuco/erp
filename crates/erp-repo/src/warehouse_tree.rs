use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

const SCHEMA: u32 = 1;
const MAX_LIMIT: u32 = 200;
const MAX_SEARCH_CHARS: usize = 256;
const MAX_SEARCH_BYTES: usize = 1024;
const MAX_CURSOR_CODE_BYTES: usize = 4096;
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeQuery {
    pub schema: u32,
    pub parent: Option<Uuid>,
    pub after: Option<TreeCursor>,
    pub limit: u32,
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeCursor {
    pub code: String,
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreePage {
    pub items: Vec<TreeItem>,
    pub total: u64,
    pub next: Option<TreeCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeItem {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub kind: TreeKind,
    pub code: String,
    pub name: Option<String>,
    pub status: String,
    pub has_children: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeKind {
    Warehouse,
    Location,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("warehouse parent was not found")]
    NotFound,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

pub fn validate(query: &TreeQuery) -> Result<(), Error> {
    if query.schema != SCHEMA {
        return Err(Error::Invalid("unsupported warehouse tree schema".into()));
    }
    if query.limit == 0 || query.limit > MAX_LIMIT {
        return Err(Error::Invalid("limit must be between 1 and 200".into()));
    }
    if query.parent.is_some_and(|id| id.is_nil()) {
        return Err(Error::Invalid("parent must not be nil".into()));
    }
    if let Some(after) = &query.after {
        if after.id.is_nil() {
            return Err(Error::Invalid("cursor id must not be nil".into()));
        }
        if after.code.contains('\0') || after.code.len() > MAX_CURSOR_CODE_BYTES {
            return Err(Error::Invalid(
                "cursor code must contain no NUL and be at most 4096 bytes".into(),
            ));
        }
    }
    if let Some(search) = &query.search {
        if search.contains('\0')
            || search.chars().count() > MAX_SEARCH_CHARS
            || search.len() > MAX_SEARCH_BYTES
        {
            return Err(Error::Invalid(
                "search must contain no NUL and be at most 256 characters and 1024 bytes".into(),
            ));
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    parent_id: Option<Uuid>,
    code: String,
    name: Option<String>,
    status: String,
    has_children: bool,
}

pub async fn query(
    pool: &PgPool,
    tenant: Uuid,
    query: &TreeQuery,
    warehouse_ids: Option<&[Uuid]>,
) -> Result<TreePage, Error> {
    validate(query)?;
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(literal_contains);

    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;

    let (total, rows) = if let Some(parent) = query.parent {
        let mut exists = QueryBuilder::<Postgres>::new(
            "select exists(select 1 from warehouses w where w.tenant_id = ",
        );
        exists
            .push_bind(tenant)
            .push(" and w.warehouse_id = ")
            .push_bind(parent);
        push_scope(&mut exists, "w.warehouse_id", warehouse_ids);
        exists.push(")");
        let allowed: bool = exists.build_query_scalar().fetch_one(&mut *tx).await?;
        if !allowed {
            return Err(Error::NotFound);
        }
        child_queries(
            &mut tx,
            tenant,
            parent,
            query,
            warehouse_ids,
            search.as_deref(),
        )
        .await?
    } else {
        root_queries(&mut tx, tenant, query, warehouse_ids, search.as_deref()).await?
    };
    tx.commit().await?;

    let total = u64::try_from(total)
        .map_err(|_| Error::Invalid("warehouse tree returned a negative total".into()))?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        validate_row(&row)?;
        items.push(TreeItem {
            id: row.id,
            parent_id: row.parent_id,
            kind: if row.parent_id.is_some() {
                TreeKind::Location
            } else {
                TreeKind::Warehouse
            },
            code: row.code,
            name: row.name,
            status: row.status,
            has_children: row.has_children,
        });
    }
    let has_more = items.len() > query.limit as usize;
    items.truncate(query.limit as usize);
    let next = has_more.then(|| {
        let last = &items[items.len() - 1];
        TreeCursor {
            code: last.code.clone(),
            id: last.id,
        }
    });
    let page = TreePage { items, total, next };
    let encoded = serde_json::to_vec(&page)
        .map_err(|_| Error::Invalid("warehouse tree response cannot be encoded".into()))?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Error::Invalid(
            "warehouse tree response exceeds 8 MiB".into(),
        ));
    }
    Ok(page)
}

fn validate_row(row: &Row) -> Result<(), Error> {
    if row.id.is_nil() || row.parent_id.is_some_and(|id| id.is_nil()) {
        return Err(Error::Invalid(
            "warehouse tree contains a nil node id".into(),
        ));
    }
    validate_result_text(&row.code, "node code")?;
    if let Some(name) = &row.name {
        validate_result_text(name, "node name")?;
    }
    validate_result_text(&row.status, "node status")?;
    Ok(())
}

fn validate_result_text(value: &str, field: &str) -> Result<(), Error> {
    if value.contains('\0') || value.len() > MAX_CURSOR_CODE_BYTES {
        return Err(Error::Invalid(format!(
            "warehouse tree {field} contains NUL or exceeds 4096 bytes"
        )));
    }
    Ok(())
}

async fn root_queries(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    tenant: Uuid,
    query: &TreeQuery,
    warehouse_ids: Option<&[Uuid]>,
    search: Option<&str>,
) -> Result<(i64, Vec<Row>), sqlx::Error> {
    let mut count = QueryBuilder::<Postgres>::new(
        "select count(*)::bigint from warehouses w where w.tenant_id = ",
    );
    count.push_bind(tenant);
    push_scope(&mut count, "w.warehouse_id", warehouse_ids);
    push_root_search(&mut count, search);
    let total = count.build_query_scalar().fetch_one(&mut **tx).await?;

    let mut page = QueryBuilder::<Postgres>::new(
        "select w.warehouse_id as id, null::uuid as parent_id, w.code, w.name, w.status, exists(select 1 from warehouse_locations hc where hc.tenant_id = w.tenant_id and hc.warehouse_id = w.warehouse_id",
    );
    if let Some(pattern) = search {
        page.push(" and ((w.code ilike ")
            .push_bind(pattern)
            .push(" escape '!' or w.name ilike ")
            .push_bind(pattern)
            .push(" escape '!') or (hc.code ilike ")
            .push_bind(pattern)
            .push(" escape '!' or coalesce(hc.name, '') ilike ")
            .push_bind(pattern)
            .push(" escape '!'))");
    }
    page.push(") as has_children from warehouses w where w.tenant_id = ")
        .push_bind(tenant);
    push_scope(&mut page, "w.warehouse_id", warehouse_ids);
    push_root_search(&mut page, search);
    push_cursor(&mut page, "w.code", "w.warehouse_id", query.after.as_ref());
    page.push(" order by w.code collate \"C\" asc, w.warehouse_id asc limit ")
        .push_bind(i64::from(query.limit) + 1);
    let rows = page.build_query_as().fetch_all(&mut **tx).await?;
    Ok((total, rows))
}

async fn child_queries(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    tenant: Uuid,
    parent: Uuid,
    query: &TreeQuery,
    warehouse_ids: Option<&[Uuid]>,
    search: Option<&str>,
) -> Result<(i64, Vec<Row>), sqlx::Error> {
    let mut count = QueryBuilder::<Postgres>::new(
        "select count(*)::bigint from warehouse_locations l join warehouses w on w.tenant_id = l.tenant_id and w.warehouse_id = l.warehouse_id where l.tenant_id = ",
    );
    count
        .push_bind(tenant)
        .push(" and l.warehouse_id = ")
        .push_bind(parent);
    push_scope(&mut count, "l.warehouse_id", warehouse_ids);
    push_child_search(&mut count, search);
    let total = count.build_query_scalar().fetch_one(&mut **tx).await?;

    let mut page = QueryBuilder::<Postgres>::new(
        "select l.location_id as id, l.warehouse_id as parent_id, l.code, l.name, case when l.is_disabled then 'DISABLED' else 'ACTIVE' end as status, false as has_children from warehouse_locations l join warehouses w on w.tenant_id = l.tenant_id and w.warehouse_id = l.warehouse_id where l.tenant_id = ",
    );
    page.push_bind(tenant)
        .push(" and l.warehouse_id = ")
        .push_bind(parent);
    push_scope(&mut page, "l.warehouse_id", warehouse_ids);
    push_child_search(&mut page, search);
    push_cursor(&mut page, "l.code", "l.location_id", query.after.as_ref());
    page.push(" order by l.code collate \"C\" asc, l.location_id asc limit ")
        .push_bind(i64::from(query.limit) + 1);
    let rows = page.build_query_as().fetch_all(&mut **tx).await?;
    Ok((total, rows))
}

fn push_scope<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    column: &str,
    warehouse_ids: Option<&'a [Uuid]>,
) {
    if let Some(ids) = warehouse_ids {
        qb.push(" and ")
            .push(column)
            .push(" = any(")
            .push_bind(ids)
            .push(")");
    }
}

fn push_root_search<'a>(qb: &mut QueryBuilder<'a, Postgres>, search: Option<&'a str>) {
    if let Some(pattern) = search {
        qb.push(" and (w.code ilike ")
            .push_bind(pattern)
            .push(" escape '!' or w.name ilike ")
            .push_bind(pattern)
            .push(" escape '!' or exists(select 1 from warehouse_locations sl where sl.tenant_id = w.tenant_id and sl.warehouse_id = w.warehouse_id and (sl.code ilike ")
            .push_bind(pattern)
            .push(" escape '!' or coalesce(sl.name, '') ilike ")
            .push_bind(pattern)
            .push(" escape '!')))");
    }
}

fn push_child_search<'a>(qb: &mut QueryBuilder<'a, Postgres>, search: Option<&'a str>) {
    if let Some(pattern) = search {
        qb.push(" and (w.code ilike ")
            .push_bind(pattern)
            .push(" escape '!' or w.name ilike ")
            .push_bind(pattern)
            .push(" escape '!' or l.code ilike ")
            .push_bind(pattern)
            .push(" escape '!' or coalesce(l.name, '') ilike ")
            .push_bind(pattern)
            .push(" escape '!')");
    }
}

fn push_cursor<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    code_column: &str,
    id_column: &str,
    cursor: Option<&'a TreeCursor>,
) {
    if let Some(cursor) = cursor {
        qb.push(" and (")
            .push(code_column)
            .push(" collate \"C\", ")
            .push(id_column)
            .push(") > (")
            .push_bind(cursor.code.as_str())
            .push(" collate \"C\", ")
            .push_bind(cursor.id)
            .push(")");
    }
}

fn literal_contains(value: &str) -> String {
    format!(
        "%{}%",
        value
            .replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> TreeQuery {
        TreeQuery {
            schema: 1,
            parent: None,
            after: None,
            limit: 20,
            search: None,
        }
    }

    #[test]
    fn validation_enforces_schema_limits_ids_and_text_budgets() {
        assert!(validate(&input()).is_ok());
        for limit in [0, 201] {
            let mut invalid = input();
            invalid.limit = limit;
            assert!(matches!(validate(&invalid), Err(Error::Invalid(_))));
        }
        let mut invalid = input();
        invalid.schema = 2;
        assert!(matches!(validate(&invalid), Err(Error::Invalid(_))));
        invalid.schema = 1;
        invalid.parent = Some(Uuid::nil());
        assert!(matches!(validate(&invalid), Err(Error::Invalid(_))));
        invalid.parent = None;
        invalid.after = Some(TreeCursor {
            code: "ok".into(),
            id: Uuid::nil(),
        });
        assert!(matches!(validate(&invalid), Err(Error::Invalid(_))));
        invalid.after = Some(TreeCursor {
            code: "x".repeat(4097),
            id: Uuid::from_u128(1),
        });
        assert!(matches!(validate(&invalid), Err(Error::Invalid(_))));
        invalid.after = None;
        invalid.search = Some("界".repeat(257));
        assert!(matches!(validate(&invalid), Err(Error::Invalid(_))));
        invalid.search = Some("a\0b".into());
        assert!(matches!(validate(&invalid), Err(Error::Invalid(_))));
        assert_eq!(literal_contains("a%b_c!"), "%a!%b!_c!!%");
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; uses connection-local temporary tables"]
    async fn pg_tree_is_stable_scoped_searchable_typed_and_pure_read() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table warehouses (warehouse_id uuid primary key, tenant_id uuid not null, code text not null, name text not null, status text not null default 'ACTIVE')").execute(&pool).await.unwrap();
        sqlx::query("create temporary table warehouse_locations (location_id uuid primary key, tenant_id uuid not null, warehouse_id uuid not null, code text not null, name text, is_disabled boolean not null default false)").execute(&pool).await.unwrap();
        let tenant = Uuid::from_u128(1);
        let foreign_tenant = Uuid::from_u128(2);
        let w1 = Uuid::from_u128(11);
        let w2 = Uuid::from_u128(12);
        let foreign = Uuid::from_u128(13);
        let duplicate_entity_id = Uuid::from_u128(11);
        sqlx::query("insert into warehouses values ($1,$2,'A','Alpha','ACTIVE'),($3,$2,'B','Beta','DISABLED'),($4,$5,'Z','Foreign','ACTIVE')")
            .bind(w1).bind(tenant).bind(w2).bind(foreign).bind(foreign_tenant)
            .execute(&pool).await.unwrap();
        sqlx::query("insert into warehouse_locations values ($1,$2,$3,'A0','Needle location',false),($4,$2,$3,'A1','Second',true),($5,$2,$3,'A2',null,false),($6,$7,$8,'Z0','Foreign child',false)")
            .bind(duplicate_entity_id).bind(tenant).bind(w1)
            .bind(Uuid::from_u128(21)).bind(Uuid::from_u128(22))
            .bind(Uuid::from_u128(23)).bind(foreign_tenant).bind(foreign)
            .execute(&pool).await.unwrap();

        let roots = query(&pool, tenant, &input(), None).await.unwrap();
        assert_eq!(roots.total, 2);
        assert_eq!(roots.items.len(), 2);
        assert!(roots.items[0].has_children);
        assert_eq!(roots.items[0].kind, TreeKind::Warehouse);
        let first_root = query(
            &pool,
            tenant,
            &TreeQuery {
                limit: 1,
                ..input()
            },
            None,
        )
        .await
        .unwrap();
        let second_root = query(
            &pool,
            tenant,
            &TreeQuery {
                after: first_root.next,
                limit: 1,
                ..input()
            },
            None,
        )
        .await
        .unwrap();
        assert_eq!(first_root.items[0].id, w1);
        assert_eq!(second_root.items[0].id, w2);
        assert_eq!(second_root.total, 2);
        let warehouse_count: i64 = sqlx::query_scalar("select count(*) from warehouses")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            warehouse_count, 3,
            "root reads must not create a default warehouse"
        );

        let empty_scope = query(&pool, tenant, &input(), Some(&[])).await.unwrap();
        assert!(empty_scope.items.is_empty());
        assert_eq!(empty_scope.total, 0);
        let scoped = query(&pool, tenant, &input(), Some(&[w2])).await.unwrap();
        assert_eq!(
            scoped.items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![w2]
        );
        assert!(matches!(
            query(
                &pool,
                tenant,
                &TreeQuery {
                    parent: Some(foreign),
                    ..input()
                },
                None
            )
            .await,
            Err(Error::NotFound)
        ));
        assert!(matches!(
            query(
                &pool,
                tenant,
                &TreeQuery {
                    parent: Some(w1),
                    ..input()
                },
                Some(&[w2])
            )
            .await,
            Err(Error::NotFound)
        ));

        let first = query(
            &pool,
            tenant,
            &TreeQuery {
                parent: Some(w1),
                limit: 1,
                ..input()
            },
            None,
        )
        .await
        .unwrap();
        assert_eq!(first.total, 3);
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].id, duplicate_entity_id);
        assert_eq!(first.items[0].kind, TreeKind::Location);
        assert_eq!(first.items[0].status, "ACTIVE");
        let second = query(
            &pool,
            tenant,
            &TreeQuery {
                parent: Some(w1),
                after: first.next.clone(),
                limit: 1,
                ..input()
            },
            None,
        )
        .await
        .unwrap();
        assert_eq!(second.items[0].code, "A1");
        assert_eq!(second.items[0].status, "DISABLED");
        assert_eq!(second.total, 3);

        let found_root = query(
            &pool,
            tenant,
            &TreeQuery {
                search: Some("Needle".into()),
                ..input()
            },
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            found_root
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![w1]
        );
        assert!(found_root.items[0].has_children);
        let found_child = query(
            &pool,
            tenant,
            &TreeQuery {
                parent: Some(w1),
                search: Some("Needle".into()),
                ..input()
            },
            None,
        )
        .await
        .unwrap();
        assert_eq!(found_child.items.len(), 1);
        assert_eq!(found_child.items[0].code, "A0");
        let parent_match = query(
            &pool,
            tenant,
            &TreeQuery {
                parent: Some(w1),
                search: Some("Alpha".into()),
                ..input()
            },
            None,
        )
        .await
        .unwrap();
        assert_eq!(parent_match.items.len(), 3);
    }
}
