use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuSupplier {
    pub supplier_id: uuid::Uuid,
    pub name: String,
    pub contact: Option<String>,
    pub status: String,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

pub struct SkuSuppliersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkuSuppliersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_for_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<SkuSupplier>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            supplier_id: uuid::Uuid,
            name: String,
            contact: Option<String>,
            status: String,
            is_primary: bool,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select ss.supplier_id,
                   s.name,
                   s.contact,
                   s.status,
                   ss.is_primary,
                   ss.created_at
            from sku_suppliers ss
            join suppliers s
              on s.supplier_id = ss.supplier_id
             and s.tenant_id = ss.tenant_id
            where ss.tenant_id = $1 and ss.sku_id = $2
            order by ss.is_primary desc, s.name
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| SkuSupplier {
                supplier_id: row.supplier_id,
                name: row.name,
                contact: row.contact,
                status: row.status,
                is_primary: row.is_primary,
                created_at: row.created_at,
            })
            .collect())
    }

    pub async fn replace_sku_suppliers(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        supplier_ids: Vec<uuid::Uuid>,
        primary_supplier_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<()> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        sqlx::query(
            r#"
            delete from sku_suppliers
            where tenant_id = $1 and sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .execute(&mut *tx)
        .await?;

        if !supplier_ids.is_empty() {
            let mut qb = QueryBuilder::<Postgres>::new(
                "insert into sku_suppliers (tenant_id, sku_id, supplier_id, is_primary) ",
            );
            qb.push_values(supplier_ids.iter(), |mut row, supplier_id| {
                row.push_bind(uuid::Uuid::from(tenant_id));
                row.push_bind(sku_id);
                row.push_bind(*supplier_id);
                row.push_bind(
                    primary_supplier_id
                        .map(|primary| primary == *supplier_id)
                        .unwrap_or(false),
                );
            });
            qb.build().execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
