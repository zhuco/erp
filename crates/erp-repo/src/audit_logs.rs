use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

fn ensure_non_blank(field: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{field} must not be blank"));
    }
    Ok(())
}

fn normalize_non_blank(field: &str, value: &str) -> anyhow::Result<String> {
    ensure_non_blank(field, value)?;
    Ok(value.trim().to_string())
}

fn normalize_optional_non_blank(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let normalized = text.trim().to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub audit_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub user_id: Option<UserId>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub ip: Option<String>,
    pub ua: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AuditLogCreate {
    pub tenant_id: TenantId,
    pub user_id: Option<UserId>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub ip: Option<String>,
    pub ua: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditLogCursor {
    pub created_at: DateTime<Utc>,
    pub audit_id: uuid::Uuid,
}

impl AuditLogCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.audit_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let audit_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            audit_id,
        })
    }
}

pub struct AuditLogListFilters {
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub user_id: Option<UserId>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub cursor: Option<AuditLogCursor>,
    pub limit: i64,
}

pub struct AuditLogListResult {
    pub items: Vec<AuditLog>,
    pub next_cursor: Option<AuditLogCursor>,
}

pub struct AuditLogsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AuditLogsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, input: AuditLogCreate) -> anyhow::Result<AuditLog> {
        Self::insert_with_tx(self.pool, input).await
    }

    pub async fn insert_with_tx<'t, T>(
        executor: T,
        input: AuditLogCreate,
    ) -> anyhow::Result<AuditLog>
    where
        T: sqlx::Executor<'t, Database = Postgres>,
    {
        let action = normalize_non_blank("action", &input.action)?;
        let entity_type = normalize_non_blank("entity_type", &input.entity_type)?;
        let entity_id = normalize_optional_non_blank(input.entity_id);
        let ip = normalize_optional_non_blank(input.ip);
        let ua = normalize_optional_non_blank(input.ua);
        let row: AuditLogRow = sqlx::query_as(
            r#"
            insert into audit_logs(
                tenant_id,
                user_id,
                action,
                entity_type,
                entity_id,
                before_json,
                after_json,
                ip,
                ua
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            returning
                audit_id,
                tenant_id,
                user_id,
                action,
                entity_type,
                entity_id,
                before_json,
                after_json,
                ip,
                ua,
                created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.user_id.map(uuid::Uuid::from))
        .bind(action)
        .bind(entity_type)
        .bind(entity_id)
        .bind(input.before_json)
        .bind(input.after_json)
        .bind(ip)
        .bind(ua)
        .fetch_one(executor)
        .await?;

        Ok(row.into_log())
    }

    pub async fn list_logs(
        &self,
        tenant_id: TenantId,
        filters: AuditLogListFilters,
    ) -> anyhow::Result<AuditLogListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                audit_id,
                tenant_id,
                user_id,
                action,
                entity_type,
                entity_id,
                before_json,
                after_json,
                ip,
                ua,
                created_at
            from audit_logs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(action) = filters.action {
            qb.push(" and action = ");
            qb.push_bind(action);
        }
        if let Some(entity_type) = filters.entity_type {
            qb.push(" and entity_type = ");
            qb.push_bind(entity_type);
        }
        if let Some(entity_id) = filters.entity_id {
            qb.push(" and entity_id = ");
            qb.push_bind(entity_id);
        }
        if let Some(user_id) = filters.user_id {
            qb.push(" and user_id = ");
            qb.push_bind(uuid::Uuid::from(user_id));
        }
        if let Some(created_from) = filters.created_from {
            qb.push(" and created_at >= ");
            qb.push_bind(created_from);
        }
        if let Some(created_to) = filters.created_to {
            qb.push(" and created_at <= ");
            qb.push_bind(created_to);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, audit_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.audit_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, audit_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<AuditLogRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| AuditLogCursor {
            created_at: row.created_at,
            audit_id: row.audit_id,
        });
        let items = rows.into_iter().map(AuditLogRow::into_log).collect();

        Ok(AuditLogListResult { items, next_cursor })
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_non_blank, normalize_optional_non_blank};

    #[test]
    fn ensure_non_blank_rejects_blank_values() {
        assert!(ensure_non_blank("action", " ").is_err());
        assert!(ensure_non_blank("action", "\t").is_err());
        assert!(ensure_non_blank("action", "OMS.ORDER_EDIT").is_ok());
    }

    #[test]
    fn normalize_optional_non_blank_trims_and_drops_empty() {
        assert_eq!(normalize_optional_non_blank(Some("".to_string())), None);
        assert_eq!(normalize_optional_non_blank(Some("  ".to_string())), None);
        assert_eq!(
            normalize_optional_non_blank(Some("  order-1 ".to_string())),
            Some("order-1".to_string())
        );
    }
}

#[derive(sqlx::FromRow)]
struct AuditLogRow {
    audit_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    user_id: Option<uuid::Uuid>,
    action: String,
    entity_type: String,
    entity_id: Option<String>,
    before_json: Option<Value>,
    after_json: Option<Value>,
    ip: Option<String>,
    ua: Option<String>,
    created_at: DateTime<Utc>,
}

impl AuditLogRow {
    fn into_log(self) -> AuditLog {
        AuditLog {
            audit_id: self.audit_id,
            tenant_id: TenantId(self.tenant_id),
            user_id: self.user_id.map(UserId),
            action: self.action,
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            before_json: self.before_json,
            after_json: self.after_json,
            ip: self.ip,
            ua: self.ua,
            created_at: self.created_at,
        }
    }
}
