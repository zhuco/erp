use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supplier {
    pub supplier_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub contact: Option<String>,
    pub settlement_terms: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SupplierListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SupplierListResult {
    pub items: Vec<Supplier>,
    pub total: i64,
}

pub struct SupplierCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub contact: Option<String>,
    pub settlement_terms: Option<String>,
    pub status: String,
}

pub struct SupplierUpdate {
    pub name: Option<String>,
    pub contact: Option<String>,
    pub settlement_terms: Option<String>,
    pub status: Option<String>,
}

pub struct SuppliersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SuppliersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_suppliers(
        &self,
        tenant_id: TenantId,
        filters: SupplierListFilters,
    ) -> anyhow::Result<SupplierListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let total: i64 = match &keyword {
            Some(keyword) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from suppliers
                    where tenant_id = $1
                      and name ilike $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from suppliers
                    where tenant_id = $1
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select supplier_id, name, contact, settlement_terms, status, created_at, updated_at
            from suppliers
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }

        qb.push(" order by updated_at desc, supplier_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            supplier_id: uuid::Uuid,
            name: String,
            contact: Option<String>,
            settlement_terms: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Supplier {
                supplier_id: row.supplier_id,
                tenant_id,
                name: row.name,
                contact: row.contact,
                settlement_terms: row.settlement_terms,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SupplierListResult { items, total })
    }

    pub async fn create_supplier(&self, input: SupplierCreate) -> anyhow::Result<Supplier> {
        #[derive(sqlx::FromRow)]
        struct Row {
            supplier_id: uuid::Uuid,
            name: String,
            contact: Option<String>,
            settlement_terms: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into suppliers (tenant_id, name, contact, settlement_terms, status)
            values ($1, $2, $3, $4, $5)
            on conflict (tenant_id, name)
            do update set
                contact = excluded.contact,
                settlement_terms = excluded.settlement_terms,
                status = excluded.status,
                updated_at = now()
            returning supplier_id, name, contact, settlement_terms, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.contact)
        .bind(&input.settlement_terms)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(Supplier {
            supplier_id: row.supplier_id,
            tenant_id: input.tenant_id,
            name: row.name,
            contact: row.contact,
            settlement_terms: row.settlement_terms,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_supplier(
        &self,
        tenant_id: TenantId,
        supplier_id: uuid::Uuid,
        input: SupplierUpdate,
    ) -> anyhow::Result<Option<Supplier>> {
        let mut qb = QueryBuilder::<Postgres>::new("update suppliers set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(contact) = input.contact {
            separated.push("contact = ");
            separated.push_bind(contact);
        }
        if let Some(terms) = input.settlement_terms {
            separated.push("settlement_terms = ");
            separated.push_bind(terms);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and supplier_id = ");
        qb.push_bind(supplier_id);
        qb.push(" returning supplier_id, name, contact, settlement_terms, status, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            supplier_id: uuid::Uuid,
            name: String,
            contact: Option<String>,
            settlement_terms: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(Supplier {
            supplier_id: row.supplier_id,
            tenant_id,
            name: row.name,
            contact: row.contact,
            settlement_terms: row.settlement_terms,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_supplier(
        &self,
        tenant_id: TenantId,
        supplier_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Supplier>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            supplier_id: uuid::Uuid,
            name: String,
            contact: Option<String>,
            settlement_terms: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from suppliers
            where tenant_id = $1 and supplier_id = $2
            returning supplier_id, name, contact, settlement_terms, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(supplier_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Supplier {
            supplier_id: row.supplier_id,
            tenant_id,
            name: row.name,
            contact: row.contact,
            settlement_terms: row.settlement_terms,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
