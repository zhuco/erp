use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApPaymentPlan {
    pub plan_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub ap_id: uuid::Uuid,
    pub supplier_id: Option<uuid::Uuid>,
    pub supplier_name: Option<String>,
    pub planned_date: Option<NaiveDate>,
    pub amount_cent: i64,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ApPaymentPlanListFilters {
    pub status: Option<String>,
    pub supplier_id: Option<uuid::Uuid>,
    pub ap_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ApPaymentPlanListResult {
    pub items: Vec<ApPaymentPlan>,
    pub total: i64,
}

pub struct ApPaymentPlanCreate {
    pub tenant_id: TenantId,
    pub ap_id: uuid::Uuid,
    pub supplier_id: Option<uuid::Uuid>,
    pub planned_date: Option<NaiveDate>,
    pub amount_cent: i64,
    pub status: String,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

pub struct ApPaymentPlanUpdate {
    pub planned_date: Option<NaiveDate>,
    pub amount_cent: Option<i64>,
    pub status: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

pub struct ApPaymentPlansRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ApPaymentPlansRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_plans(
        &self,
        tenant_id: TenantId,
        filters: ApPaymentPlanListFilters,
    ) -> anyhow::Result<ApPaymentPlanListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from ap_payment_plans p
            where p.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                p.plan_id,
                p.ap_id,
                p.supplier_id,
                s.name as supplier_name,
                p.planned_date,
                p.amount_cent,
                p.status,
                p.paid_at,
                p.notes,
                p.created_by,
                p.created_at,
                p.updated_at
            from ap_payment_plans p
            left join suppliers s
              on s.supplier_id = p.supplier_id
             and s.tenant_id = p.tenant_id
            where p.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by p.created_at desc, p.plan_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            ap_id: uuid::Uuid,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            planned_date: Option<NaiveDate>,
            amount_cent: i64,
            status: String,
            paid_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ApPaymentPlan {
                plan_id: row.plan_id,
                tenant_id,
                ap_id: row.ap_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                planned_date: row.planned_date,
                amount_cent: row.amount_cent,
                status: row.status,
                paid_at: row.paid_at,
                notes: row.notes,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ApPaymentPlanListResult { items, total })
    }

    pub async fn create_plan(&self, input: ApPaymentPlanCreate) -> anyhow::Result<ApPaymentPlan> {
        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            ap_id: uuid::Uuid,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            planned_date: Option<NaiveDate>,
            amount_cent: i64,
            status: String,
            paid_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into ap_payment_plans(
                tenant_id, ap_id, supplier_id, planned_date, amount_cent, status, notes, created_by
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning plan_id, ap_id, supplier_id,
                      (select name from suppliers s where s.supplier_id = ap_payment_plans.supplier_id
                       and s.tenant_id = ap_payment_plans.tenant_id) as supplier_name,
                      planned_date, amount_cent, status, paid_at, notes, created_by, created_at,
                      updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.ap_id)
        .bind(input.supplier_id)
        .bind(input.planned_date)
        .bind(input.amount_cent)
        .bind(&input.status)
        .bind(&input.notes)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(ApPaymentPlan {
            plan_id: row.plan_id,
            tenant_id: input.tenant_id,
            ap_id: row.ap_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            planned_date: row.planned_date,
            amount_cent: row.amount_cent,
            status: row.status,
            paid_at: row.paid_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_plan(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
        input: ApPaymentPlanUpdate,
    ) -> anyhow::Result<Option<ApPaymentPlan>> {
        let mut qb = QueryBuilder::<Postgres>::new("update ap_payment_plans set ");
        let mut separated = qb.separated(", ");

        if let Some(planned_date) = input.planned_date {
            separated.push("planned_date = ");
            separated.push_bind(planned_date);
        }
        if let Some(amount_cent) = input.amount_cent {
            separated.push("amount_cent = ");
            separated.push_bind(amount_cent);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(paid_at) = input.paid_at {
            separated.push("paid_at = ");
            separated.push_bind(paid_at);
        }
        if let Some(notes) = input.notes {
            separated.push("notes = ");
            separated.push_bind(notes);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and plan_id = ");
        qb.push_bind(plan_id);
        qb.push(
            r#"
            returning plan_id, ap_id, supplier_id,
                      (select name from suppliers s where s.supplier_id = ap_payment_plans.supplier_id
                       and s.tenant_id = ap_payment_plans.tenant_id) as supplier_name,
                      planned_date, amount_cent, status, paid_at, notes, created_by, created_at,
                      updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            ap_id: uuid::Uuid,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            planned_date: Option<NaiveDate>,
            amount_cent: i64,
            status: String,
            paid_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ApPaymentPlan {
            plan_id: row.plan_id,
            tenant_id,
            ap_id: row.ap_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            planned_date: row.planned_date,
            amount_cent: row.amount_cent,
            status: row.status,
            paid_at: row.paid_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_plan(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ApPaymentPlan>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            ap_id: uuid::Uuid,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            planned_date: Option<NaiveDate>,
            amount_cent: i64,
            status: String,
            paid_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from ap_payment_plans
            where tenant_id = $1 and plan_id = $2
            returning plan_id, ap_id, supplier_id,
                      (select name from suppliers s where s.supplier_id = ap_payment_plans.supplier_id
                       and s.tenant_id = ap_payment_plans.tenant_id) as supplier_name,
                      planned_date, amount_cent, status, paid_at, notes, created_by, created_at,
                      updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ApPaymentPlan {
            plan_id: row.plan_id,
            tenant_id,
            ap_id: row.ap_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            planned_date: row.planned_date,
            amount_cent: row.amount_cent,
            status: row.status,
            paid_at: row.paid_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn mark_paid(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ApPaymentPlan>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            ap_id: uuid::Uuid,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            planned_date: Option<NaiveDate>,
            amount_cent: i64,
            status: String,
            paid_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update ap_payment_plans
            set status = $3,
                paid_at = now(),
                updated_at = now()
            where tenant_id = $1 and plan_id = $2
            returning plan_id, ap_id, supplier_id,
                      (select name from suppliers s where s.supplier_id = ap_payment_plans.supplier_id
                       and s.tenant_id = ap_payment_plans.tenant_id) as supplier_name,
                      planned_date, amount_cent, status, paid_at, notes, created_by, created_at,
                      updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .bind("PAID")
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ApPaymentPlan {
            plan_id: row.plan_id,
            tenant_id,
            ap_id: row.ap_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            planned_date: row.planned_date,
            amount_cent: row.amount_cent,
            status: row.status,
            paid_at: row.paid_at,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a ApPaymentPlanListFilters) {
    if let Some(status) = &filters.status {
        qb.push(" and p.status = ");
        qb.push_bind(status);
    }
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and p.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(ap_id) = filters.ap_id {
        qb.push(" and p.ap_id = ");
        qb.push_bind(ap_id);
    }
}
