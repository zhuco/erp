use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintMapping {
    pub mapping_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub doc_type: String,
    pub template_id: uuid::Uuid,
    pub device_fingerprint: Option<String>,
    pub printer_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PrintMappingUpsert {
    pub tenant_id: TenantId,
    pub doc_type: String,
    pub template_id: uuid::Uuid,
    pub device_fingerprint: Option<String>,
    pub printer_name: Option<String>,
}

pub struct PrintMappingsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PrintMappingsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_mappings(&self, tenant_id: TenantId) -> anyhow::Result<Vec<PrintMapping>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            doc_type: String,
            template_id: uuid::Uuid,
            device_fingerprint: Option<String>,
            printer_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select mapping_id, doc_type, template_id, device_fingerprint, printer_name,
                   created_at, updated_at
            from print_mappings
            where tenant_id = $1
            order by updated_at desc, mapping_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PrintMapping {
                mapping_id: row.mapping_id,
                tenant_id,
                doc_type: row.doc_type,
                template_id: row.template_id,
                device_fingerprint: row.device_fingerprint,
                printer_name: row.printer_name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn get_mapping(
        &self,
        tenant_id: TenantId,
        doc_type: &str,
    ) -> anyhow::Result<Option<PrintMapping>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            doc_type: String,
            template_id: uuid::Uuid,
            device_fingerprint: Option<String>,
            printer_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select mapping_id, doc_type, template_id, device_fingerprint, printer_name,
                   created_at, updated_at
            from print_mappings
            where tenant_id = $1 and doc_type = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_type)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            doc_type: row.doc_type,
            template_id: row.template_id,
            device_fingerprint: row.device_fingerprint,
            printer_name: row.printer_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn upsert_mapping(&self, input: PrintMappingUpsert) -> anyhow::Result<PrintMapping> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            doc_type: String,
            template_id: uuid::Uuid,
            device_fingerprint: Option<String>,
            printer_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into print_mappings(
                tenant_id, doc_type, template_id, device_fingerprint, printer_name
            ) values ($1, $2, $3, $4, $5)
            on conflict (tenant_id, doc_type)
            do update set
                template_id = excluded.template_id,
                device_fingerprint = excluded.device_fingerprint,
                printer_name = excluded.printer_name,
                updated_at = now()
            returning mapping_id, doc_type, template_id, device_fingerprint, printer_name,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.doc_type)
        .bind(input.template_id)
        .bind(&input.device_fingerprint)
        .bind(&input.printer_name)
        .fetch_one(self.pool)
        .await?;

        Ok(PrintMapping {
            mapping_id: row.mapping_id,
            tenant_id: input.tenant_id,
            doc_type: row.doc_type,
            template_id: row.template_id,
            device_fingerprint: row.device_fingerprint,
            printer_name: row.printer_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn delete_mapping(
        &self,
        tenant_id: TenantId,
        doc_type: &str,
    ) -> anyhow::Result<Option<PrintMapping>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            doc_type: String,
            template_id: uuid::Uuid,
            device_fingerprint: Option<String>,
            printer_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from print_mappings
            where tenant_id = $1 and doc_type = $2
            returning mapping_id, doc_type, template_id, device_fingerprint, printer_name,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_type)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            doc_type: row.doc_type,
            template_id: row.template_id,
            device_fingerprint: row.device_fingerprint,
            printer_name: row.printer_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
