use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsInterface {
    pub interface_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub carrier_name: String,
    pub api_base_url: String,
    pub template_name: String,
    pub status: String,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub retry_policy: String,
    pub alert_channel: String,
    pub approval_status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsInterfaceApproval {
    pub approval_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub interface_id: uuid::Uuid,
    pub action: String,
    pub operator: String,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct LogisticsInterfaceListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub approval_status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct LogisticsInterfaceListResult {
    pub items: Vec<LogisticsInterface>,
    pub total: i64,
}

pub struct LogisticsInterfaceCreate {
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub carrier_name: String,
    pub api_base_url: String,
    pub template_name: String,
    pub status: String,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub retry_policy: String,
    pub alert_channel: String,
    pub approval_status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

pub struct LogisticsInterfaceUpdate {
    pub carrier_code: Option<String>,
    pub carrier_name: Option<String>,
    pub api_base_url: Option<String>,
    pub template_name: Option<String>,
    pub status: Option<String>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub retry_policy: Option<String>,
    pub alert_channel: Option<String>,
    pub approval_status: Option<String>,
    pub approved_by: Option<Option<String>>,
    pub approved_at: Option<Option<DateTime<Utc>>>,
    pub notes: Option<Option<String>>,
}

pub struct LogisticsInterfaceApprovalCreate {
    pub tenant_id: TenantId,
    pub interface_id: uuid::Uuid,
    pub action: String,
    pub operator: String,
    pub comment: Option<String>,
}

pub struct LogisticsInterfacesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsInterfacesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_interfaces(
        &self,
        tenant_id: TenantId,
        filters: LogisticsInterfaceListFilters,
    ) -> anyhow::Result<LogisticsInterfaceListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut total_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_interfaces where tenant_id = ",
        );
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(keyword) = &keyword {
            total_qb.push(" and (carrier_name ilike ");
            total_qb.push_bind(keyword);
            total_qb.push(" or carrier_code ilike ");
            total_qb.push_bind(keyword);
            total_qb.push(")");
        }
        if let Some(status) = &filters.status {
            total_qb.push(" and status = ");
            total_qb.push_bind(status);
        }
        if let Some(approval_status) = &filters.approval_status {
            total_qb.push(" and approval_status = ");
            total_qb.push_bind(approval_status);
        }
        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select interface_id, carrier_code, carrier_name, api_base_url, template_name, status,
                   last_checked_at, retry_policy, alert_channel, approval_status, approved_by,
                   approved_at, notes, created_at, updated_at
            from logistics_interfaces
            where tenant_id = 
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and (carrier_name ilike ");
            qb.push_bind(keyword);
            qb.push(" or carrier_code ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(approval_status) = &filters.approval_status {
            qb.push(" and approval_status = ");
            qb.push_bind(approval_status);
        }

        qb.push(" order by updated_at desc, interface_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            interface_id: uuid::Uuid,
            carrier_code: String,
            carrier_name: String,
            api_base_url: String,
            template_name: String,
            status: String,
            last_checked_at: Option<DateTime<Utc>>,
            retry_policy: String,
            alert_channel: String,
            approval_status: String,
            approved_by: Option<String>,
            approved_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| LogisticsInterface {
                interface_id: row.interface_id,
                tenant_id,
                carrier_code: row.carrier_code,
                carrier_name: row.carrier_name,
                api_base_url: row.api_base_url,
                template_name: row.template_name,
                status: row.status,
                last_checked_at: row.last_checked_at,
                retry_policy: row.retry_policy,
                alert_channel: row.alert_channel,
                approval_status: row.approval_status,
                approved_by: row.approved_by,
                approved_at: row.approved_at,
                notes: row.notes,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(LogisticsInterfaceListResult { items, total })
    }

    pub async fn create_interface(
        &self,
        input: LogisticsInterfaceCreate,
    ) -> anyhow::Result<LogisticsInterface> {
        #[derive(sqlx::FromRow)]
        struct Row {
            interface_id: uuid::Uuid,
            carrier_code: String,
            carrier_name: String,
            api_base_url: String,
            template_name: String,
            status: String,
            last_checked_at: Option<DateTime<Utc>>,
            retry_policy: String,
            alert_channel: String,
            approval_status: String,
            approved_by: Option<String>,
            approved_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into logistics_interfaces(
                tenant_id, carrier_code, carrier_name, api_base_url, template_name, status,
                last_checked_at, retry_policy, alert_channel, approval_status, approved_by,
                approved_at, notes
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            returning interface_id, carrier_code, carrier_name, api_base_url, template_name, status,
                      last_checked_at, retry_policy, alert_channel, approval_status, approved_by,
                      approved_at, notes, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.carrier_code)
        .bind(&input.carrier_name)
        .bind(&input.api_base_url)
        .bind(&input.template_name)
        .bind(&input.status)
        .bind(input.last_checked_at)
        .bind(&input.retry_policy)
        .bind(&input.alert_channel)
        .bind(&input.approval_status)
        .bind(&input.approved_by)
        .bind(input.approved_at)
        .bind(&input.notes)
        .fetch_one(self.pool)
        .await?;

        Ok(LogisticsInterface {
            interface_id: row.interface_id,
            tenant_id: input.tenant_id,
            carrier_code: row.carrier_code,
            carrier_name: row.carrier_name,
            api_base_url: row.api_base_url,
            template_name: row.template_name,
            status: row.status,
            last_checked_at: row.last_checked_at,
            retry_policy: row.retry_policy,
            alert_channel: row.alert_channel,
            approval_status: row.approval_status,
            approved_by: row.approved_by,
            approved_at: row.approved_at,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_interface(
        &self,
        tenant_id: TenantId,
        interface_id: uuid::Uuid,
        input: LogisticsInterfaceUpdate,
    ) -> anyhow::Result<Option<LogisticsInterface>> {
        let mut qb = QueryBuilder::<Postgres>::new("update logistics_interfaces set ");
        let mut separated = qb.separated(", ");

        if let Some(carrier_code) = input.carrier_code {
            separated.push("carrier_code = ");
            separated.push_bind(carrier_code);
        }
        if let Some(carrier_name) = input.carrier_name {
            separated.push("carrier_name = ");
            separated.push_bind(carrier_name);
        }
        if let Some(api_base_url) = input.api_base_url {
            separated.push("api_base_url = ");
            separated.push_bind(api_base_url);
        }
        if let Some(template_name) = input.template_name {
            separated.push("template_name = ");
            separated.push_bind(template_name);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(last_checked_at) = input.last_checked_at {
            separated.push("last_checked_at = ");
            separated.push_bind(last_checked_at);
        }
        if let Some(retry_policy) = input.retry_policy {
            separated.push("retry_policy = ");
            separated.push_bind(retry_policy);
        }
        if let Some(alert_channel) = input.alert_channel {
            separated.push("alert_channel = ");
            separated.push_bind(alert_channel);
        }
        if let Some(approval_status) = input.approval_status {
            separated.push("approval_status = ");
            separated.push_bind(approval_status);
        }
        if let Some(approved_by) = input.approved_by {
            separated.push("approved_by = ");
            separated.push_bind(approved_by);
        }
        if let Some(approved_at) = input.approved_at {
            separated.push("approved_at = ");
            separated.push_bind(approved_at);
        }
        if let Some(notes) = input.notes {
            separated.push("notes = ");
            separated.push_bind(notes);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and interface_id = ");
        qb.push_bind(interface_id);
        qb.push(
            " returning interface_id, carrier_code, carrier_name, api_base_url, template_name, status,
              last_checked_at, retry_policy, alert_channel, approval_status, approved_by, approved_at,
              notes, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            interface_id: uuid::Uuid,
            carrier_code: String,
            carrier_name: String,
            api_base_url: String,
            template_name: String,
            status: String,
            last_checked_at: Option<DateTime<Utc>>,
            retry_policy: String,
            alert_channel: String,
            approval_status: String,
            approved_by: Option<String>,
            approved_at: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(LogisticsInterface {
            interface_id: row.interface_id,
            tenant_id,
            carrier_code: row.carrier_code,
            carrier_name: row.carrier_name,
            api_base_url: row.api_base_url,
            template_name: row.template_name,
            status: row.status,
            last_checked_at: row.last_checked_at,
            retry_policy: row.retry_policy,
            alert_channel: row.alert_channel,
            approval_status: row.approval_status,
            approved_by: row.approved_by,
            approved_at: row.approved_at,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn insert_approval(
        &self,
        input: LogisticsInterfaceApprovalCreate,
    ) -> anyhow::Result<LogisticsInterfaceApproval> {
        #[derive(sqlx::FromRow)]
        struct Row {
            approval_id: uuid::Uuid,
            interface_id: uuid::Uuid,
            action: String,
            operator: String,
            comment: Option<String>,
            created_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into logistics_interface_approvals(
                tenant_id, interface_id, action, operator, comment
            ) values ($1, $2, $3, $4, $5)
            returning approval_id, interface_id, action, operator, comment, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.interface_id)
        .bind(&input.action)
        .bind(&input.operator)
        .bind(&input.comment)
        .fetch_one(self.pool)
        .await?;

        Ok(LogisticsInterfaceApproval {
            approval_id: row.approval_id,
            tenant_id: input.tenant_id,
            interface_id: row.interface_id,
            action: row.action,
            operator: row.operator,
            comment: row.comment,
            created_at: row.created_at,
        })
    }

    pub async fn list_approvals(
        &self,
        tenant_id: TenantId,
        interface_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<LogisticsInterfaceApproval>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            approval_id: uuid::Uuid,
            interface_id: uuid::Uuid,
            action: String,
            operator: String,
            comment: Option<String>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select approval_id, interface_id, action, operator, comment, created_at
            from logistics_interface_approvals
            where tenant_id = $1 and interface_id = $2
            order by created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(interface_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| LogisticsInterfaceApproval {
                approval_id: row.approval_id,
                tenant_id,
                interface_id: row.interface_id,
                action: row.action,
                operator: row.operator,
                comment: row.comment,
                created_at: row.created_at,
            })
            .collect())
    }
}
