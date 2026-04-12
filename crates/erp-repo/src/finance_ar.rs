use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArDoc {
    pub ar_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub customer_id: Option<uuid::Uuid>,
    pub customer_name: Option<String>,
    pub ref_type: String,
    pub ref_id: String,
    pub amount_cent: i64,
    pub currency: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub written_off_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ArDocListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ArDocListResult {
    pub items: Vec<ArDoc>,
    pub total: i64,
}

pub struct ArDocCreate {
    pub tenant_id: TenantId,
    pub customer_id: Option<uuid::Uuid>,
    pub ref_type: String,
    pub ref_id: String,
    pub amount_cent: i64,
    pub currency: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub created_by: Option<String>,
}

pub struct ArDocUpdate {
    pub customer_id: Option<uuid::Uuid>,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub amount_cent: Option<i64>,
    pub currency: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub written_off_at: Option<DateTime<Utc>>,
}

pub struct ArDocsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ArDocsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_docs(
        &self,
        tenant_id: TenantId,
        filters: ArDocListFilters,
    ) -> anyhow::Result<ArDocListResult> {
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
                    from ar_docs ar
                    left join customers c
                      on c.customer_id = ar.customer_id
                     and c.tenant_id = ar.tenant_id
                    where ar.tenant_id = $1
                      and (ar.ref_id ilike $2 or c.name ilike $2)
                      and ($3::text is null or ar.status = $3)
                      and ($4::uuid is null or ar.customer_id = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .bind(filters.customer_id)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from ar_docs ar
                    where ar.tenant_id = $1
                      and ($2::text is null or ar.status = $2)
                      and ($3::uuid is null or ar.customer_id = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .bind(filters.customer_id)
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                ar.ar_id,
                ar.customer_id,
                c.name as customer_name,
                ar.ref_type,
                ar.ref_id,
                ar.amount_cent,
                ar.currency,
                ar.status,
                ar.due_date,
                ar.written_off_at,
                ar.created_by,
                ar.created_at,
                ar.updated_at
            from ar_docs ar
            left join customers c
              on c.customer_id = ar.customer_id
             and c.tenant_id = ar.tenant_id
            where ar.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and (ar.ref_id ilike ");
            qb.push_bind(keyword);
            qb.push(" or c.name ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }
        if let Some(status) = &filters.status {
            qb.push(" and ar.status = ");
            qb.push_bind(status);
        }
        if let Some(customer_id) = filters.customer_id {
            qb.push(" and ar.customer_id = ");
            qb.push_bind(customer_id);
        }

        qb.push(" order by ar.created_at desc, ar.ar_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            ar_id: uuid::Uuid,
            customer_id: Option<uuid::Uuid>,
            customer_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            currency: String,
            status: String,
            due_date: Option<NaiveDate>,
            written_off_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ArDoc {
                ar_id: row.ar_id,
                tenant_id,
                customer_id: row.customer_id,
                customer_name: row.customer_name,
                ref_type: row.ref_type,
                ref_id: row.ref_id,
                amount_cent: row.amount_cent,
                currency: row.currency,
                status: row.status,
                due_date: row.due_date,
                written_off_at: row.written_off_at,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ArDocListResult { items, total })
    }

    pub async fn get_doc(
        &self,
        tenant_id: TenantId,
        ar_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ArDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ar_id: uuid::Uuid,
            customer_id: Option<uuid::Uuid>,
            customer_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            currency: String,
            status: String,
            due_date: Option<NaiveDate>,
            written_off_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                ar.ar_id,
                ar.customer_id,
                c.name as customer_name,
                ar.ref_type,
                ar.ref_id,
                ar.amount_cent,
                ar.currency,
                ar.status,
                ar.due_date,
                ar.written_off_at,
                ar.created_by,
                ar.created_at,
                ar.updated_at
            from ar_docs ar
            left join customers c
              on c.customer_id = ar.customer_id
             and c.tenant_id = ar.tenant_id
            where ar.tenant_id = $1 and ar.ar_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ar_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ArDoc {
            ar_id: row.ar_id,
            tenant_id,
            customer_id: row.customer_id,
            customer_name: row.customer_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            status: row.status,
            due_date: row.due_date,
            written_off_at: row.written_off_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_doc(&self, input: ArDocCreate) -> anyhow::Result<ArDoc> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ar_id: uuid::Uuid,
            customer_id: Option<uuid::Uuid>,
            customer_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            currency: String,
            status: String,
            due_date: Option<NaiveDate>,
            written_off_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into ar_docs(
                tenant_id, customer_id, ref_type, ref_id, amount_cent, currency, status, due_date,
                created_by
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            returning ar_id, customer_id,
                      (select name from customers c where c.customer_id = ar_docs.customer_id
                       and c.tenant_id = ar_docs.tenant_id) as customer_name,
                      ref_type, ref_id, amount_cent, currency, status, due_date, written_off_at,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.customer_id)
        .bind(&input.ref_type)
        .bind(&input.ref_id)
        .bind(input.amount_cent)
        .bind(&input.currency)
        .bind(&input.status)
        .bind(input.due_date)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(ArDoc {
            ar_id: row.ar_id,
            tenant_id: input.tenant_id,
            customer_id: row.customer_id,
            customer_name: row.customer_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            status: row.status,
            due_date: row.due_date,
            written_off_at: row.written_off_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_doc(
        &self,
        tenant_id: TenantId,
        ar_id: uuid::Uuid,
        input: ArDocUpdate,
    ) -> anyhow::Result<Option<ArDoc>> {
        let mut qb = QueryBuilder::<Postgres>::new("update ar_docs set ");
        let mut separated = qb.separated(", ");

        if let Some(customer_id) = input.customer_id {
            separated.push("customer_id = ");
            separated.push_bind(customer_id);
        }
        if let Some(ref_type) = input.ref_type {
            separated.push("ref_type = ");
            separated.push_bind(ref_type);
        }
        if let Some(ref_id) = input.ref_id {
            separated.push("ref_id = ");
            separated.push_bind(ref_id);
        }
        if let Some(amount_cent) = input.amount_cent {
            separated.push("amount_cent = ");
            separated.push_bind(amount_cent);
        }
        if let Some(currency) = input.currency {
            separated.push("currency = ");
            separated.push_bind(currency);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(due_date) = input.due_date {
            separated.push("due_date = ");
            separated.push_bind(due_date);
        }
        if let Some(written_off_at) = input.written_off_at {
            separated.push("written_off_at = ");
            separated.push_bind(written_off_at);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and ar_id = ");
        qb.push_bind(ar_id);
        qb.push(
            r#"
            returning ar_id, customer_id,
                      (select name from customers c where c.customer_id = ar_docs.customer_id
                       and c.tenant_id = ar_docs.tenant_id) as customer_name,
                      ref_type, ref_id, amount_cent, currency, status, due_date, written_off_at,
                      created_by, created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            ar_id: uuid::Uuid,
            customer_id: Option<uuid::Uuid>,
            customer_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            currency: String,
            status: String,
            due_date: Option<NaiveDate>,
            written_off_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ArDoc {
            ar_id: row.ar_id,
            tenant_id,
            customer_id: row.customer_id,
            customer_name: row.customer_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            status: row.status,
            due_date: row.due_date,
            written_off_at: row.written_off_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_doc(
        &self,
        tenant_id: TenantId,
        ar_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ArDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ar_id: uuid::Uuid,
            customer_id: Option<uuid::Uuid>,
            customer_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            currency: String,
            status: String,
            due_date: Option<NaiveDate>,
            written_off_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from ar_docs
            where tenant_id = $1 and ar_id = $2
            returning ar_id, customer_id,
                      (select name from customers c where c.customer_id = ar_docs.customer_id
                       and c.tenant_id = ar_docs.tenant_id) as customer_name,
                      ref_type, ref_id, amount_cent, currency, status, due_date, written_off_at,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ar_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ArDoc {
            ar_id: row.ar_id,
            tenant_id,
            customer_id: row.customer_id,
            customer_name: row.customer_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            status: row.status,
            due_date: row.due_date,
            written_off_at: row.written_off_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn mark_written_off(
        &self,
        tenant_id: TenantId,
        ar_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ArDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ar_id: uuid::Uuid,
            customer_id: Option<uuid::Uuid>,
            customer_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            currency: String,
            status: String,
            due_date: Option<NaiveDate>,
            written_off_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update ar_docs
            set status = $3,
                written_off_at = now(),
                updated_at = now()
            where tenant_id = $1 and ar_id = $2
            returning ar_id, customer_id,
                      (select name from customers c where c.customer_id = ar_docs.customer_id
                       and c.tenant_id = ar_docs.tenant_id) as customer_name,
                      ref_type, ref_id, amount_cent, currency, status, due_date, written_off_at,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ar_id)
        .bind("WRITTEN_OFF")
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ArDoc {
            ar_id: row.ar_id,
            tenant_id,
            customer_id: row.customer_id,
            customer_name: row.customer_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            status: row.status,
            due_date: row.due_date,
            written_off_at: row.written_off_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
