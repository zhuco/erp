use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsInstallationTask {
    pub task_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: String,
    pub customer_name: String,
    pub address: String,
    pub service_type: String,
    pub assigned_team: Option<String>,
    pub status: String,
    pub remedy_ticket_id: Option<String>,
    pub exception_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsInstallationAppointment {
    pub appointment_id: uuid::Uuid,
    pub task_id: uuid::Uuid,
    pub scheduled_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LogisticsInstallationTaskCreate {
    pub tenant_id: TenantId,
    pub order_id: String,
    pub customer_name: String,
    pub address: String,
    pub service_type: String,
    pub assigned_team: Option<String>,
    pub status: String,
    pub remedy_ticket_id: Option<String>,
    pub exception_note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogisticsInstallationTaskUpdate {
    pub customer_name: Option<String>,
    pub address: Option<String>,
    pub service_type: Option<String>,
    pub assigned_team: Option<String>,
    pub status: Option<String>,
    pub remedy_ticket_id: Option<String>,
    pub exception_note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogisticsInstallationTaskListFilters {
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsInstallationTaskListResult {
    pub items: Vec<LogisticsInstallationTask>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsInstallationAppointmentCreate {
    pub task_id: uuid::Uuid,
    pub scheduled_at: DateTime<Utc>,
    pub status: String,
}

pub struct LogisticsInstallationTasksRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsInstallationTasksRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_tasks(
        &self,
        tenant_id: TenantId,
        filters: LogisticsInstallationTaskListFilters,
    ) -> anyhow::Result<LogisticsInstallationTaskListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_installation_tasks where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                task_id, order_id, customer_name, address, service_type,
                assigned_team, status, remedy_ticket_id, exception_note, created_at, updated_at
            from logistics_installation_tasks
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by updated_at desc, task_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<LogisticsInstallationTaskRow> =
            qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_task(tenant_id))
            .collect();

        Ok(LogisticsInstallationTaskListResult { items, total })
    }

    pub async fn get_task(
        &self,
        tenant_id: TenantId,
        task_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsInstallationTask>> {
        let row: Option<LogisticsInstallationTaskRow> = sqlx::query_as(
            r#"
            select
                task_id, order_id, customer_name, address, service_type,
                assigned_team, status, remedy_ticket_id, exception_note, created_at, updated_at
            from logistics_installation_tasks
            where tenant_id = $1 and task_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(task_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_task(tenant_id)))
    }

    pub async fn create_task(
        &self,
        input: LogisticsInstallationTaskCreate,
    ) -> anyhow::Result<LogisticsInstallationTask> {
        let row: LogisticsInstallationTaskRow = sqlx::query_as(
            r#"
            insert into logistics_installation_tasks(
                tenant_id, order_id, customer_name, address, service_type,
                assigned_team, status, remedy_ticket_id, exception_note
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            returning
                task_id, order_id, customer_name, address, service_type,
                assigned_team, status, remedy_ticket_id, exception_note, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.order_id)
        .bind(&input.customer_name)
        .bind(&input.address)
        .bind(&input.service_type)
        .bind(&input.assigned_team)
        .bind(&input.status)
        .bind(&input.remedy_ticket_id)
        .bind(&input.exception_note)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_task(input.tenant_id))
    }

    pub async fn update_task(
        &self,
        tenant_id: TenantId,
        task_id: uuid::Uuid,
        input: LogisticsInstallationTaskUpdate,
    ) -> anyhow::Result<Option<LogisticsInstallationTask>> {
        let mut qb = QueryBuilder::<Postgres>::new("update logistics_installation_tasks set ");
        let mut separated = qb.separated(", ");

        if let Some(customer_name) = input.customer_name {
            separated.push("customer_name = ");
            separated.push_bind(customer_name);
        }
        if let Some(address) = input.address {
            separated.push("address = ");
            separated.push_bind(address);
        }
        if let Some(service_type) = input.service_type {
            separated.push("service_type = ");
            separated.push_bind(service_type);
        }
        if let Some(assigned_team) = input.assigned_team {
            separated.push("assigned_team = ");
            separated.push_bind(assigned_team);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(remedy_ticket_id) = input.remedy_ticket_id {
            separated.push("remedy_ticket_id = ");
            separated.push_bind(remedy_ticket_id);
        }
        if let Some(exception_note) = input.exception_note {
            separated.push("exception_note = ");
            separated.push_bind(exception_note);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and task_id = ");
        qb.push_bind(task_id);
        qb.push(
            " returning task_id, order_id, customer_name, address, service_type,
              assigned_team, status, remedy_ticket_id, exception_note, created_at, updated_at",
        );

        let row: Option<LogisticsInstallationTaskRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_task(tenant_id)))
    }
}

pub struct LogisticsInstallationAppointmentsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsInstallationAppointmentsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_task_ids(
        &self,
        tenant_id: TenantId,
        task_ids: &[uuid::Uuid],
    ) -> anyhow::Result<Vec<LogisticsInstallationAppointment>> {
        if task_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                a.appointment_id, a.task_id, a.scheduled_at, a.status, a.created_at
            from logistics_installation_appointments a
            join logistics_installation_tasks t on a.task_id = t.task_id
            where t.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and a.task_id in (");
        let mut separated = qb.separated(", ");
        for task_id in task_ids {
            separated.push_bind(task_id);
        }
        qb.push(")");
        qb.push(" order by a.created_at asc, a.appointment_id asc");

        let rows: Vec<LogisticsInstallationAppointmentRow> =
            qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(|row| row.into_appointment()).collect())
    }

    pub async fn list_by_task(
        &self,
        tenant_id: TenantId,
        task_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<LogisticsInstallationAppointment>> {
        let rows: Vec<LogisticsInstallationAppointmentRow> = sqlx::query_as(
            r#"
            select a.appointment_id, a.task_id, a.scheduled_at, a.status, a.created_at
            from logistics_installation_appointments a
            join logistics_installation_tasks t on a.task_id = t.task_id
            where t.tenant_id = $1 and a.task_id = $2
            order by a.created_at asc, a.appointment_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(task_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.into_appointment()).collect())
    }

    pub async fn create_appointment(
        &self,
        input: LogisticsInstallationAppointmentCreate,
    ) -> anyhow::Result<LogisticsInstallationAppointment> {
        let row: LogisticsInstallationAppointmentRow = sqlx::query_as(
            r#"
            insert into logistics_installation_appointments(
                task_id, scheduled_at, status
            ) values ($1, $2, $3)
            returning appointment_id, task_id, scheduled_at, status, created_at
            "#,
        )
        .bind(input.task_id)
        .bind(input.scheduled_at)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_appointment())
    }
}

#[derive(sqlx::FromRow)]
struct LogisticsInstallationTaskRow {
    task_id: uuid::Uuid,
    order_id: String,
    customer_name: String,
    address: String,
    service_type: String,
    assigned_team: Option<String>,
    status: String,
    remedy_ticket_id: Option<String>,
    exception_note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl LogisticsInstallationTaskRow {
    fn into_task(self, tenant_id: TenantId) -> LogisticsInstallationTask {
        LogisticsInstallationTask {
            task_id: self.task_id,
            tenant_id,
            order_id: self.order_id,
            customer_name: self.customer_name,
            address: self.address,
            service_type: self.service_type,
            assigned_team: self.assigned_team,
            status: self.status,
            remedy_ticket_id: self.remedy_ticket_id,
            exception_note: self.exception_note,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct LogisticsInstallationAppointmentRow {
    appointment_id: uuid::Uuid,
    task_id: uuid::Uuid,
    scheduled_at: DateTime<Utc>,
    status: String,
    created_at: DateTime<Utc>,
}

impl LogisticsInstallationAppointmentRow {
    fn into_appointment(self) -> LogisticsInstallationAppointment {
        LogisticsInstallationAppointment {
            appointment_id: self.appointment_id,
            task_id: self.task_id,
            scheduled_at: self.scheduled_at,
            status: self.status,
            created_at: self.created_at,
        }
    }
}
