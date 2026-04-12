use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsExceptionType {
    pub tenant_id: TenantId,
    pub type_code: String,
    pub name: String,
    pub sla_hours: i32,
    pub severity: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsExceptionWorkOrder {
    pub ticket_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: String,
    pub tracking_no: String,
    pub carrier_code: String,
    pub exception_type: String,
    pub status: String,
    pub assignee: Option<String>,
    pub created_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub escalation_level: i32,
    pub resolution: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
}

pub struct LogisticsExceptionTypeFilters {
    pub enabled: Option<bool>,
}

pub struct LogisticsExceptionWorkOrderFilters {
    pub status: Option<String>,
    pub exception_type: Option<String>,
    pub assignee: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct LogisticsExceptionWorkOrderListResult {
    pub items: Vec<LogisticsExceptionWorkOrder>,
    pub total: i64,
}

pub struct LogisticsExceptionTypeCreate {
    pub tenant_id: TenantId,
    pub type_code: String,
    pub name: String,
    pub sla_hours: i32,
    pub severity: String,
    pub enabled: bool,
}

pub struct LogisticsExceptionTypeUpdate {
    pub name: Option<String>,
    pub sla_hours: Option<i32>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
}

pub struct LogisticsExceptionWorkOrderCreate {
    pub tenant_id: TenantId,
    pub order_id: String,
    pub tracking_no: String,
    pub carrier_code: String,
    pub exception_type: String,
    pub status: String,
    pub assignee: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
}

pub struct LogisticsExceptionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsExceptionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_exception_types(
        &self,
        tenant_id: TenantId,
        filters: LogisticsExceptionTypeFilters,
    ) -> anyhow::Result<Vec<LogisticsExceptionType>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select type_code, name, sla_hours, severity, enabled, created_at, updated_at
            from logistics_exception_types
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(enabled) = filters.enabled {
            qb.push(" and enabled = ");
            qb.push_bind(enabled);
        }

        qb.push(" order by updated_at desc, type_code asc");

        #[derive(sqlx::FromRow)]
        struct Row {
            type_code: String,
            name: String,
            sla_hours: i32,
            severity: String,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| LogisticsExceptionType {
                tenant_id,
                type_code: row.type_code,
                name: row.name,
                sla_hours: row.sla_hours,
                severity: row.severity,
                enabled: row.enabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn upsert_exception_type(
        &self,
        input: LogisticsExceptionTypeCreate,
    ) -> anyhow::Result<LogisticsExceptionType> {
        #[derive(sqlx::FromRow)]
        struct Row {
            type_code: String,
            name: String,
            sla_hours: i32,
            severity: String,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into logistics_exception_types(
                tenant_id, type_code, name, sla_hours, severity, enabled
            ) values ($1, $2, $3, $4, $5, $6)
            on conflict (tenant_id, type_code)
            do update set
                name = excluded.name,
                sla_hours = excluded.sla_hours,
                severity = excluded.severity,
                enabled = excluded.enabled,
                updated_at = now()
            returning type_code, name, sla_hours, severity, enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.type_code)
        .bind(&input.name)
        .bind(input.sla_hours)
        .bind(&input.severity)
        .bind(input.enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(LogisticsExceptionType {
            tenant_id: input.tenant_id,
            type_code: row.type_code,
            name: row.name,
            sla_hours: row.sla_hours,
            severity: row.severity,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_exception_type(
        &self,
        tenant_id: TenantId,
        type_code: &str,
        input: LogisticsExceptionTypeUpdate,
    ) -> anyhow::Result<Option<LogisticsExceptionType>> {
        let mut qb = QueryBuilder::<Postgres>::new("update logistics_exception_types set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(sla_hours) = input.sla_hours {
            separated.push("sla_hours = ");
            separated.push_bind(sla_hours);
        }
        if let Some(severity) = input.severity {
            separated.push("severity = ");
            separated.push_bind(severity);
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and type_code = ");
        qb.push_bind(type_code);
        qb.push(" returning type_code, name, sla_hours, severity, enabled, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            type_code: String,
            name: String,
            sla_hours: i32,
            severity: String,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(LogisticsExceptionType {
            tenant_id,
            type_code: row.type_code,
            name: row.name,
            sla_hours: row.sla_hours,
            severity: row.severity,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_work_orders(
        &self,
        tenant_id: TenantId,
        filters: LogisticsExceptionWorkOrderFilters,
    ) -> anyhow::Result<LogisticsExceptionWorkOrderListResult> {
        let mut total_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_exception_work_orders where tenant_id = ",
        );
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            total_qb.push(" and status = ");
            total_qb.push_bind(status);
        }
        if let Some(exception_type) = &filters.exception_type {
            total_qb.push(" and exception_type = ");
            total_qb.push_bind(exception_type);
        }
        if let Some(assignee) = &filters.assignee {
            total_qb.push(" and assignee = ");
            total_qb.push_bind(assignee);
        }
        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select ticket_id, order_id, tracking_no, carrier_code, exception_type, status,
                   assignee, created_at, due_at, escalation_level, resolution, resolved_at
            from logistics_exception_work_orders
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(exception_type) = &filters.exception_type {
            qb.push(" and exception_type = ");
            qb.push_bind(exception_type);
        }
        if let Some(assignee) = &filters.assignee {
            qb.push(" and assignee = ");
            qb.push_bind(assignee);
        }

        qb.push(" order by created_at desc, ticket_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            ticket_id: uuid::Uuid,
            order_id: String,
            tracking_no: String,
            carrier_code: String,
            exception_type: String,
            status: String,
            assignee: Option<String>,
            created_at: DateTime<Utc>,
            due_at: Option<DateTime<Utc>>,
            escalation_level: i32,
            resolution: Option<String>,
            resolved_at: Option<DateTime<Utc>>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| LogisticsExceptionWorkOrder {
                ticket_id: row.ticket_id,
                tenant_id,
                order_id: row.order_id,
                tracking_no: row.tracking_no,
                carrier_code: row.carrier_code,
                exception_type: row.exception_type,
                status: row.status,
                assignee: row.assignee,
                created_at: row.created_at,
                due_at: row.due_at,
                escalation_level: row.escalation_level,
                resolution: row.resolution,
                resolved_at: row.resolved_at,
            })
            .collect();

        Ok(LogisticsExceptionWorkOrderListResult { items, total })
    }

    pub async fn create_work_order(
        &self,
        input: LogisticsExceptionWorkOrderCreate,
    ) -> anyhow::Result<LogisticsExceptionWorkOrder> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ticket_id: uuid::Uuid,
            order_id: String,
            tracking_no: String,
            carrier_code: String,
            exception_type: String,
            status: String,
            assignee: Option<String>,
            created_at: DateTime<Utc>,
            due_at: Option<DateTime<Utc>>,
            escalation_level: i32,
            resolution: Option<String>,
            resolved_at: Option<DateTime<Utc>>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into logistics_exception_work_orders(
                tenant_id, order_id, tracking_no, carrier_code, exception_type, status, assignee,
                due_at
            ) values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning ticket_id, order_id, tracking_no, carrier_code, exception_type, status, assignee,
                      created_at, due_at, escalation_level, resolution, resolved_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.order_id)
        .bind(&input.tracking_no)
        .bind(&input.carrier_code)
        .bind(&input.exception_type)
        .bind(&input.status)
        .bind(&input.assignee)
        .bind(input.due_at)
        .fetch_one(self.pool)
        .await?;

        Ok(LogisticsExceptionWorkOrder {
            ticket_id: row.ticket_id,
            tenant_id: input.tenant_id,
            order_id: row.order_id,
            tracking_no: row.tracking_no,
            carrier_code: row.carrier_code,
            exception_type: row.exception_type,
            status: row.status,
            assignee: row.assignee,
            created_at: row.created_at,
            due_at: row.due_at,
            escalation_level: row.escalation_level,
            resolution: row.resolution,
            resolved_at: row.resolved_at,
        })
    }

    pub async fn assign_work_order(
        &self,
        tenant_id: TenantId,
        ticket_id: uuid::Uuid,
        assignee: String,
    ) -> anyhow::Result<Option<LogisticsExceptionWorkOrder>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ticket_id: uuid::Uuid,
            order_id: String,
            tracking_no: String,
            carrier_code: String,
            exception_type: String,
            status: String,
            assignee: Option<String>,
            created_at: DateTime<Utc>,
            due_at: Option<DateTime<Utc>>,
            escalation_level: i32,
            resolution: Option<String>,
            resolved_at: Option<DateTime<Utc>>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update logistics_exception_work_orders
            set assignee = $1, status = 'IN_PROGRESS', updated_at = now()
            where tenant_id = $2 and ticket_id = $3
            returning ticket_id, order_id, tracking_no, carrier_code, exception_type,
                      status, assignee, created_at, due_at, escalation_level, resolution, resolved_at
            "#,
        )
        .bind(&assignee)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ticket_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| LogisticsExceptionWorkOrder {
            ticket_id: row.ticket_id,
            tenant_id,
            order_id: row.order_id,
            tracking_no: row.tracking_no,
            carrier_code: row.carrier_code,
            exception_type: row.exception_type,
            status: row.status,
            assignee: row.assignee,
            created_at: row.created_at,
            due_at: row.due_at,
            escalation_level: row.escalation_level,
            resolution: row.resolution,
            resolved_at: row.resolved_at,
        }))
    }

    pub async fn resolve_work_order(
        &self,
        tenant_id: TenantId,
        ticket_id: uuid::Uuid,
        resolution: String,
    ) -> anyhow::Result<Option<LogisticsExceptionWorkOrder>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ticket_id: uuid::Uuid,
            order_id: String,
            tracking_no: String,
            carrier_code: String,
            exception_type: String,
            status: String,
            assignee: Option<String>,
            created_at: DateTime<Utc>,
            due_at: Option<DateTime<Utc>>,
            escalation_level: i32,
            resolution: Option<String>,
            resolved_at: Option<DateTime<Utc>>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update logistics_exception_work_orders
            set status = 'RESOLVED',
                resolution = $1,
                resolved_at = now(),
                updated_at = now()
            where tenant_id = $2 and ticket_id = $3
            returning ticket_id, order_id, tracking_no, carrier_code, exception_type,
                      status, assignee, created_at, due_at, escalation_level, resolution, resolved_at
            "#,
        )
        .bind(&resolution)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ticket_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| LogisticsExceptionWorkOrder {
            ticket_id: row.ticket_id,
            tenant_id,
            order_id: row.order_id,
            tracking_no: row.tracking_no,
            carrier_code: row.carrier_code,
            exception_type: row.exception_type,
            status: row.status,
            assignee: row.assignee,
            created_at: row.created_at,
            due_at: row.due_at,
            escalation_level: row.escalation_level,
            resolution: row.resolution,
            resolved_at: row.resolved_at,
        }))
    }

    pub async fn escalate_work_order(
        &self,
        tenant_id: TenantId,
        ticket_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsExceptionWorkOrder>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ticket_id: uuid::Uuid,
            order_id: String,
            tracking_no: String,
            carrier_code: String,
            exception_type: String,
            status: String,
            assignee: Option<String>,
            created_at: DateTime<Utc>,
            due_at: Option<DateTime<Utc>>,
            escalation_level: i32,
            resolution: Option<String>,
            resolved_at: Option<DateTime<Utc>>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update logistics_exception_work_orders
            set status = 'ESCALATED',
                escalation_level = escalation_level + 1,
                due_at = coalesce(due_at, now()) + interval '4 hours',
                updated_at = now()
            where tenant_id = $1 and ticket_id = $2
            returning ticket_id, order_id, tracking_no, carrier_code, exception_type,
                      status, assignee, created_at, due_at, escalation_level, resolution, resolved_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ticket_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| LogisticsExceptionWorkOrder {
            ticket_id: row.ticket_id,
            tenant_id,
            order_id: row.order_id,
            tracking_no: row.tracking_no,
            carrier_code: row.carrier_code,
            exception_type: row.exception_type,
            status: row.status,
            assignee: row.assignee,
            created_at: row.created_at,
            due_at: row.due_at,
            escalation_level: row.escalation_level,
            resolution: row.resolution,
            resolved_at: row.resolved_at,
        }))
    }
}
