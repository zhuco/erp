use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductChangeHistoryItem {
    pub time: DateTime<Utc>,
    pub action: String,
    pub actor: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductChangeRequest {
    pub request_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub sku_name: Option<String>,
    pub title: String,
    pub change_type: String,
    pub status: String,
    pub applicant: String,
    pub approver: Option<String>,
    pub reason: Option<String>,
    pub history: Vec<ProductChangeHistoryItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductChangeRequestListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub change_type: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ProductChangeRequestListResult {
    pub items: Vec<ProductChangeRequest>,
    pub total: i64,
}

pub struct ProductChangeRequestCreate {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub title: String,
    pub change_type: String,
    pub status: String,
    pub applicant: String,
    pub approver: Option<String>,
    pub reason: Option<String>,
    pub history: Vec<ProductChangeHistoryItem>,
}

pub struct ProductChangeRequestUpdate {
    pub title: Option<String>,
    pub change_type: Option<String>,
    pub status: Option<String>,
    pub approver: Option<Option<String>>,
    pub reason: Option<Option<String>>,
    pub history: Option<Vec<ProductChangeHistoryItem>>,
}

pub struct ProductChangeRequestsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductChangeRequestsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_requests(
        &self,
        tenant_id: TenantId,
        filters: ProductChangeRequestListFilters,
    ) -> anyhow::Result<ProductChangeRequestListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from product_change_requests pcr where pcr.tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pcr.request_id,
                pcr.sku_id,
                s.sku_code,
                s.name as sku_name,
                pcr.title,
                pcr.change_type,
                pcr.status,
                pcr.applicant,
                pcr.approver,
                pcr.reason,
                pcr.history,
                pcr.created_at,
                pcr.updated_at
            from product_change_requests pcr
            join skus s
              on s.sku_id = pcr.sku_id
             and s.tenant_id = pcr.tenant_id
            where pcr.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by pcr.updated_at desc, pcr.request_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            title: String,
            change_type: String,
            status: String,
            applicant: String,
            approver: Option<String>,
            reason: Option<String>,
            history: Json<Vec<ProductChangeHistoryItem>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProductChangeRequest {
                request_id: row.request_id,
                tenant_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                sku_name: row.sku_name,
                title: row.title,
                change_type: row.change_type,
                status: row.status,
                applicant: row.applicant,
                approver: row.approver,
                reason: row.reason,
                history: row.history.0,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductChangeRequestListResult { items, total })
    }

    pub async fn get_request(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductChangeRequest>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            title: String,
            change_type: String,
            status: String,
            applicant: String,
            approver: Option<String>,
            reason: Option<String>,
            history: Json<Vec<ProductChangeHistoryItem>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                pcr.request_id,
                pcr.sku_id,
                s.sku_code,
                s.name as sku_name,
                pcr.title,
                pcr.change_type,
                pcr.status,
                pcr.applicant,
                pcr.approver,
                pcr.reason,
                pcr.history,
                pcr.created_at,
                pcr.updated_at
            from product_change_requests pcr
            join skus s
              on s.sku_id = pcr.sku_id
             and s.tenant_id = pcr.tenant_id
            where pcr.tenant_id = $1 and pcr.request_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductChangeRequest {
            request_id: row.request_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            title: row.title,
            change_type: row.change_type,
            status: row.status,
            applicant: row.applicant,
            approver: row.approver,
            reason: row.reason,
            history: row.history.0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_request(
        &self,
        input: ProductChangeRequestCreate,
    ) -> anyhow::Result<ProductChangeRequest> {
        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            title: String,
            change_type: String,
            status: String,
            applicant: String,
            approver: Option<String>,
            reason: Option<String>,
            history: Json<Vec<ProductChangeHistoryItem>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_change_requests(
                tenant_id,
                sku_id,
                title,
                change_type,
                status,
                applicant,
                approver,
                reason,
                history
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            returning
                request_id,
                sku_id,
                (select sku_code from skus where sku_id = $2) as sku_code,
                (select name from skus where sku_id = $2) as sku_name,
                title,
                change_type,
                status,
                applicant,
                approver,
                reason,
                history,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.title)
        .bind(&input.change_type)
        .bind(&input.status)
        .bind(&input.applicant)
        .bind(&input.approver)
        .bind(&input.reason)
        .bind(Json(input.history))
        .fetch_one(self.pool)
        .await?;

        Ok(ProductChangeRequest {
            request_id: row.request_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            title: row.title,
            change_type: row.change_type,
            status: row.status,
            applicant: row.applicant,
            approver: row.approver,
            reason: row.reason,
            history: row.history.0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_request(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
        input: ProductChangeRequestUpdate,
    ) -> anyhow::Result<Option<ProductChangeRequest>> {
        let mut qb = QueryBuilder::<Postgres>::new("update product_change_requests set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(title) = input.title {
            separated.push("title = ");
            separated.push_bind(title);
            has_updates = true;
        }
        if let Some(change_type) = input.change_type {
            separated.push("change_type = ");
            separated.push_bind(change_type);
            has_updates = true;
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(approver) = input.approver {
            separated.push("approver = ");
            separated.push_bind(approver);
            has_updates = true;
        }
        if let Some(reason) = input.reason {
            separated.push("reason = ");
            separated.push_bind(reason);
            has_updates = true;
        }
        if let Some(history) = input.history {
            separated.push("history = ");
            separated.push_bind(Json(history));
            has_updates = true;
        }

        if !has_updates {
            return self.get_request(tenant_id, request_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and request_id = ");
        qb.push_bind(request_id);
        qb.push(
            r#"
            returning
                request_id,
                sku_id,
                (select sku_code from skus where sku_id = product_change_requests.sku_id) as sku_code,
                (select name from skus where sku_id = product_change_requests.sku_id) as sku_name,
                title,
                change_type,
                status,
                applicant,
                approver,
                reason,
                history,
                created_at,
                updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            title: String,
            change_type: String,
            status: String,
            applicant: String,
            approver: Option<String>,
            reason: Option<String>,
            history: Json<Vec<ProductChangeHistoryItem>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ProductChangeRequest {
            request_id: row.request_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            title: row.title,
            change_type: row.change_type,
            status: row.status,
            applicant: row.applicant,
            approver: row.approver,
            reason: row.reason,
            history: row.history.0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_request(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductChangeRequest>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            title: String,
            change_type: String,
            status: String,
            applicant: String,
            approver: Option<String>,
            reason: Option<String>,
            history: Json<Vec<ProductChangeHistoryItem>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from product_change_requests
            where tenant_id = $1 and request_id = $2
            returning
                request_id,
                sku_id,
                (select sku_code from skus where sku_id = product_change_requests.sku_id) as sku_code,
                (select name from skus where sku_id = product_change_requests.sku_id) as sku_name,
                title,
                change_type,
                status,
                applicant,
                approver,
                reason,
                history,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductChangeRequest {
            request_id: row.request_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            title: row.title,
            change_type: row.change_type,
            status: row.status,
            applicant: row.applicant,
            approver: row.approver,
            reason: row.reason,
            history: row.history.0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProductChangeRequestListFilters,
) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and pcr.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(status) = filters.status.as_ref() {
        qb.push(" and pcr.status = ");
        qb.push_bind(status);
    }
    if let Some(change_type) = filters.change_type.as_ref() {
        qb.push(" and pcr.change_type = ");
        qb.push_bind(change_type);
    }
}
