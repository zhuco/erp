use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsignmentSettlement {
    pub settlement_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub partner_name: String,
    pub agreement_no: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub sales_amount_cent: i64,
    pub commission_rate: f64,
    pub commission_cent: i64,
    pub status: String,
    pub settled_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ConsignmentSettlementListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ConsignmentSettlementListResult {
    pub items: Vec<ConsignmentSettlement>,
    pub total: i64,
}

pub struct ConsignmentSettlementCreate {
    pub tenant_id: TenantId,
    pub partner_name: String,
    pub agreement_no: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub sales_amount_cent: i64,
    pub commission_rate: f64,
    pub commission_cent: i64,
    pub status: String,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

pub struct ConsignmentSettlementUpdate {
    pub partner_name: Option<String>,
    pub agreement_no: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub sales_amount_cent: Option<i64>,
    pub commission_rate: Option<f64>,
    pub commission_cent: Option<i64>,
    pub status: Option<String>,
    pub settled_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

pub struct ConsignmentSettlementsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ConsignmentSettlementsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_settlements(
        &self,
        tenant_id: TenantId,
        filters: ConsignmentSettlementListFilters,
    ) -> anyhow::Result<ConsignmentSettlementListResult> {
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
                    from consignment_settlements
                    where tenant_id = $1
                      and (partner_name ilike $2 or agreement_no ilike $2)
                      and ($3::text is null or status = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from consignment_settlements
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select settlement_id, partner_name, agreement_no, period_start, period_end,
                   sales_amount_cent, commission_rate, commission_cent, status, settled_at, notes,
                   created_by, created_at, updated_at
            from consignment_settlements
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(keyword) = &keyword {
            qb.push(" and (partner_name ilike ");
            qb.push_bind(keyword);
            qb.push(" or agreement_no ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by period_start desc nulls last, created_at desc, settlement_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            settlement_id: uuid::Uuid,
            partner_name: String,
            agreement_no: Option<String>,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            sales_amount_cent: i64,
            commission_rate: f64,
            commission_cent: i64,
            status: String,
            settled_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ConsignmentSettlement {
                settlement_id: row.settlement_id,
                tenant_id,
                partner_name: row.partner_name,
                agreement_no: row.agreement_no,
                period_start: row.period_start,
                period_end: row.period_end,
                sales_amount_cent: row.sales_amount_cent,
                commission_rate: row.commission_rate,
                commission_cent: row.commission_cent,
                status: row.status,
                settled_at: row.settled_at,
                notes: row.notes,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ConsignmentSettlementListResult { items, total })
    }

    pub async fn create_settlement(
        &self,
        input: ConsignmentSettlementCreate,
    ) -> anyhow::Result<ConsignmentSettlement> {
        #[derive(sqlx::FromRow)]
        struct Row {
            settlement_id: uuid::Uuid,
            partner_name: String,
            agreement_no: Option<String>,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            sales_amount_cent: i64,
            commission_rate: f64,
            commission_cent: i64,
            status: String,
            settled_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into consignment_settlements(
                tenant_id, partner_name, agreement_no, period_start, period_end, sales_amount_cent,
                commission_rate, commission_cent, status, notes, created_by
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            returning settlement_id, partner_name, agreement_no, period_start, period_end,
                      sales_amount_cent, commission_rate, commission_cent, status, settled_at,
                      notes, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.partner_name)
        .bind(&input.agreement_no)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(input.sales_amount_cent)
        .bind(input.commission_rate)
        .bind(input.commission_cent)
        .bind(&input.status)
        .bind(&input.notes)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(ConsignmentSettlement {
            settlement_id: row.settlement_id,
            tenant_id: input.tenant_id,
            partner_name: row.partner_name,
            agreement_no: row.agreement_no,
            period_start: row.period_start,
            period_end: row.period_end,
            sales_amount_cent: row.sales_amount_cent,
            commission_rate: row.commission_rate,
            commission_cent: row.commission_cent,
            status: row.status,
            settled_at: row.settled_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_settlement(
        &self,
        tenant_id: TenantId,
        settlement_id: uuid::Uuid,
        input: ConsignmentSettlementUpdate,
    ) -> anyhow::Result<Option<ConsignmentSettlement>> {
        let mut qb = QueryBuilder::<Postgres>::new("update consignment_settlements set ");
        let mut separated = qb.separated(", ");

        if let Some(partner_name) = input.partner_name {
            separated.push("partner_name = ");
            separated.push_bind(partner_name);
        }
        if let Some(agreement_no) = input.agreement_no {
            separated.push("agreement_no = ");
            separated.push_bind(agreement_no);
        }
        if let Some(period_start) = input.period_start {
            separated.push("period_start = ");
            separated.push_bind(period_start);
        }
        if let Some(period_end) = input.period_end {
            separated.push("period_end = ");
            separated.push_bind(period_end);
        }
        if let Some(sales_amount_cent) = input.sales_amount_cent {
            separated.push("sales_amount_cent = ");
            separated.push_bind(sales_amount_cent);
        }
        if let Some(commission_rate) = input.commission_rate {
            separated.push("commission_rate = ");
            separated.push_bind(commission_rate);
        }
        if let Some(commission_cent) = input.commission_cent {
            separated.push("commission_cent = ");
            separated.push_bind(commission_cent);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(settled_at) = input.settled_at {
            separated.push("settled_at = ");
            separated.push_bind(settled_at);
        }
        if let Some(notes) = input.notes {
            separated.push("notes = ");
            separated.push_bind(notes);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and settlement_id = ");
        qb.push_bind(settlement_id);
        qb.push(
            r#"
            returning settlement_id, partner_name, agreement_no, period_start, period_end,
                      sales_amount_cent, commission_rate, commission_cent, status, settled_at,
                      notes, created_by, created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            settlement_id: uuid::Uuid,
            partner_name: String,
            agreement_no: Option<String>,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            sales_amount_cent: i64,
            commission_rate: f64,
            commission_cent: i64,
            status: String,
            settled_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ConsignmentSettlement {
            settlement_id: row.settlement_id,
            tenant_id,
            partner_name: row.partner_name,
            agreement_no: row.agreement_no,
            period_start: row.period_start,
            period_end: row.period_end,
            sales_amount_cent: row.sales_amount_cent,
            commission_rate: row.commission_rate,
            commission_cent: row.commission_cent,
            status: row.status,
            settled_at: row.settled_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_settlement(
        &self,
        tenant_id: TenantId,
        settlement_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ConsignmentSettlement>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            settlement_id: uuid::Uuid,
            partner_name: String,
            agreement_no: Option<String>,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            sales_amount_cent: i64,
            commission_rate: f64,
            commission_cent: i64,
            status: String,
            settled_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from consignment_settlements
            where tenant_id = $1 and settlement_id = $2
            returning settlement_id, partner_name, agreement_no, period_start, period_end,
                      sales_amount_cent, commission_rate, commission_cent, status, settled_at,
                      notes, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(settlement_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ConsignmentSettlement {
            settlement_id: row.settlement_id,
            tenant_id,
            partner_name: row.partner_name,
            agreement_no: row.agreement_no,
            period_start: row.period_start,
            period_end: row.period_end,
            sales_amount_cent: row.sales_amount_cent,
            commission_rate: row.commission_rate,
            commission_cent: row.commission_cent,
            status: row.status,
            settled_at: row.settled_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn mark_settled(
        &self,
        tenant_id: TenantId,
        settlement_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ConsignmentSettlement>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            settlement_id: uuid::Uuid,
            partner_name: String,
            agreement_no: Option<String>,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            sales_amount_cent: i64,
            commission_rate: f64,
            commission_cent: i64,
            status: String,
            settled_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update consignment_settlements
            set status = $3,
                settled_at = now(),
                updated_at = now()
            where tenant_id = $1 and settlement_id = $2
            returning settlement_id, partner_name, agreement_no, period_start, period_end,
                      sales_amount_cent, commission_rate, commission_cent, status, settled_at,
                      notes, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(settlement_id)
        .bind("SETTLED")
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ConsignmentSettlement {
            settlement_id: row.settlement_id,
            tenant_id,
            partner_name: row.partner_name,
            agreement_no: row.agreement_no,
            period_start: row.period_start,
            period_end: row.period_end,
            sales_amount_cent: row.sales_amount_cent,
            commission_rate: row.commission_rate,
            commission_cent: row.commission_cent,
            status: row.status,
            settled_at: row.settled_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
