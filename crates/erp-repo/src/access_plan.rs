use chrono::{DateTime, Utc};
use erp_domain::access::{
    AccessError, FREE_LOGIN_SEAT_LIMIT, FREE_MAX_ACTIVE_SESSIONS, FREE_PLAN_CODE, PAID_PLAN_CODE,
    SessionPolicy, TenantAccessPlan,
};
use erp_types::{TenantId, UserId};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StoredAccessPlan {
    pub plan: TenantAccessPlan,
    pub credentials_epoch: i32,
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub session_id: Uuid,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub status: String,
    pub client_kind: String,
    pub client_label: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub credentials_epoch: i32,
    /// Tenant epoch read with this session. Login lookup compares it here.
    pub current_epoch: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatAdmission {
    Admitted,
    Rejected(AccessError),
}

pub struct AccessPlanRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AccessPlanRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn load_plan(&self, tenant_id: TenantId) -> anyhow::Result<Option<StoredAccessPlan>> {
        let row: Option<(String, i32, String, i32, i32)> = sqlx::query_as(
            r#"
            select coalesce(plan_code, 'FREE'),
                   coalesce(login_seat_limit, 1),
                   coalesce(session_policy, 'MULTI'),
                   coalesce(max_active_sessions, 8),
                   coalesce(credentials_epoch, 0)
            from tenants
            where tenant_id = $1
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .fetch_optional(self.pool)
        .await?;
        row.map(stored_plan)
            .transpose()
            .map_err(anyhow::Error::from)
    }

    /// Lock the tenant row, then reject a second active business login on the free plan.
    pub async fn admit_new_login_seat(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
    ) -> anyhow::Result<SeatAdmission> {
        let row: Option<(String, i32, String, i32, i32)> = sqlx::query_as(
            r#"
            select coalesce(plan_code, 'FREE'),
                   coalesce(login_seat_limit, 1),
                   coalesce(session_policy, 'MULTI'),
                   coalesce(max_active_sessions, 8),
                   coalesce(credentials_epoch, 0)
            from tenants
            where tenant_id = $1
            for update
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = row else {
            return Ok(SeatAdmission::Rejected(AccessError::SeatLimitReached));
        };
        let stored = stored_plan(row)?;
        let occupied: i64 = sqlx::query_scalar(
            r#"
            select count(*)::bigint
            from users
            where tenant_id = $1
              and status = 'ACTIVE'
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .fetch_one(&mut **tx)
        .await?;
        Ok(
            match erp_domain::access::admit_login_seat(stored.plan, occupied) {
                Ok(()) => SeatAdmission::Admitted,
                Err(error) => SeatAdmission::Rejected(error),
            },
        )
    }

    pub async fn open_session(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        verified_password_hash: &str,
        client_kind: &str,
        client_label: &str,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Result<AuthSession, AccessError>> {
        // Exactly one pooled connection. Tenant-row serialization and the
        // transaction-scoped commit lock cannot outlive a cancelled request.
        let mut tx = self.pool.begin().await?;
        sqlx::query("set local statement_timeout='10s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local lock_timeout='5s'")
            .execute(&mut *tx)
            .await?;
        let row: Option<(String, i32, String, i32, i32)> = sqlx::query_as(
            r#"
            select coalesce(plan_code, 'FREE'),
                   coalesce(login_seat_limit, 1),
                   coalesce(session_policy, 'MULTI'),
                   coalesce(max_active_sessions, 8),
                   coalesce(credentials_epoch, 0)
            from tenants
            where tenant_id = $1
            for update
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(Err(AccessError::SessionInactive));
        };
        let stored = stored_plan(row)?;

        // Password/status mutators publish their tenant epoch in the same
        // transaction. After taking that commit boundary, READ COMMITTED sees
        // any completed replacement. Do not row-lock users here: the shipped
        // mutators update user first and then tenant, so reversing that order
        // would create a deadlock. An unfinished mutation waits on this tenant
        // and can invalidate this session only after its own later commit.
        let current_user: Option<(String, String)> = sqlx::query_as(
            "select status,password_hash from users where tenant_id=$1 and user_id=$2",
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .fetch_optional(&mut *tx)
        .await?;
        match current_user {
            Some((status, hash)) if status == "ACTIVE" && hash == verified_password_hash => {}
            Some((status, _)) if status == "ACTIVE" => {
                return Ok(Err(AccessError::CredentialsStale));
            }
            _ => return Ok(Err(AccessError::SessionInactive)),
        }
        sqlx::query(
            r#"
            update auth_sessions
            set status = 'EXPIRED'
            where tenant_id = $1
              and user_id = $2
              and status = 'ACTIVE'
              and expires_at <= now()
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .execute(&mut *tx)
        .await?;
        // A successful password change invalidates these sessions even if
        // their wall-clock expiry is still in the future. Keep their history,
        // exclude them from reuse and free their quota before opening a new one.
        sqlx::query(
            r#"
            update auth_sessions
            set status = 'REVOKED', revoked_at = now(), revoke_reason = 'CREDENTIALS_CHANGED'
            where tenant_id = $1 and user_id = $2 and status = 'ACTIVE'
              and credentials_epoch <> $3
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .bind(stored.credentials_epoch)
        .execute(&mut *tx)
        .await?;
        if stored.plan.session_policy == SessionPolicy::Single {
            sqlx::query(
                r#"
                update auth_sessions
                set status = 'REVOKED',
                    revoked_at = now(),
                    revoke_reason = 'REPLACED'
                where tenant_id = $1
                  and user_id = $2
                  and status = 'ACTIVE'
                "#,
            )
            .bind(Uuid::from(tenant_id))
            .bind(Uuid::from(user_id))
            .execute(&mut *tx)
            .await?;
        }
        // The tenant row serializes openings, including an initially empty set.
        // Hold the active session rows while deciding reuse and admission.
        sqlx::query(
            r#"
            select session_id
            from auth_sessions
            where tenant_id = $1
              and user_id = $2
              and status = 'ACTIVE'
              and expires_at > now()
            for update
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .execute(&mut *tx)
        .await?;
        // A second browser tab of the same client reuses the open session.
        // Reuse is not a new session, so it must not consume the active limit.
        // A blank label is a new client and must not collapse unrelated logins.
        if !client_label.trim().is_empty() {
            let reused: Option<SessionRow> = sqlx::query_as(
                r#"
                select s.session_id, s.tenant_id, s.user_id, s.status, s.client_kind,
                       s.client_label, s.created_at, s.last_seen_at, s.expires_at,
                       s.credentials_epoch, t.credentials_epoch as current_epoch
                from auth_sessions s
                join tenants t on t.tenant_id = s.tenant_id
                where s.tenant_id = $1
                  and s.user_id = $2
                  and s.status = 'ACTIVE'
                  and s.expires_at > now()
                  and s.client_kind = $3
                  and (s.client_label = $4 or s.client_label = $5)
                  and s.credentials_epoch = $6
                order by s.created_at
                limit 1
                for update of s
                "#,
            )
            .bind(Uuid::from(tenant_id))
            .bind(Uuid::from(user_id))
            .bind(match client_kind {
                "DESKTOP" | "WEB" => client_kind,
                _ => "UNKNOWN",
            })
            .bind(client_label)
            .bind(client_label.chars().take(80).collect::<String>())
            .bind(stored.credentials_epoch)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(mut existing) = reused {
                let (expires, seen): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
                    "update auth_sessions set expires_at=greatest(expires_at,$3),last_seen_at=now()
                     where tenant_id=$1 and session_id=$2 returning expires_at,last_seen_at",
                )
                .bind(Uuid::from(tenant_id))
                .bind(existing.session_id)
                .bind(expires_at)
                .fetch_one(&mut *tx)
                .await?;
                existing.expires_at = expires;
                existing.last_seen_at = seen;
                tx.commit().await?;
                return Ok(Ok(existing.into_session()));
            }
        }
        let active: i64 = sqlx::query_scalar(
            r#"
            select count(*)::bigint
            from auth_sessions
            where tenant_id = $1
              and user_id = $2
              and status = 'ACTIVE'
              and expires_at > now()
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .fetch_one(&mut *tx)
        .await?;
        if let Err(error) = erp_domain::access::admit_session(stored.plan, active) {
            tx.rollback().await?;
            return Ok(Err(error));
        }
        let created = insert_session(
            &mut tx,
            tenant_id,
            user_id,
            client_kind,
            client_label,
            expires_at,
            stored.credentials_epoch,
        )
        .await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1::text))")
            .bind(format!("session-commit:{tenant_id}:{user_id}"))
            .execute(&mut *tx)
            .await?;
        let occupied: i64 = sqlx::query_scalar(
            r#"
            select count(*)::bigint
            from auth_sessions
            where tenant_id = $1
              and user_id = $2
              and status = 'ACTIVE'
              and expires_at > now()
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .fetch_one(&mut *tx)
        .await?;
        if occupied > i64::from(stored.plan.max_active_sessions) {
            tx.rollback().await?;
            return Ok(Err(AccessError::SessionLimitReached));
        }
        tx.commit().await?;
        Ok(Ok(created))
    }

    pub async fn active_session(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        session_id: Uuid,
    ) -> anyhow::Result<Option<AuthSession>> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            select s.session_id, s.tenant_id, s.user_id, s.status, s.client_kind, s.client_label,
                   s.created_at, s.last_seen_at, s.expires_at, s.credentials_epoch,
                   coalesce(t.credentials_epoch, 0) as current_epoch
            from auth_sessions s
            join tenants t on t.tenant_id = s.tenant_id
            where s.tenant_id = $1
              and s.user_id = $2
              and s.session_id = $3
              and s.status = 'ACTIVE'
              and s.expires_at > now()
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .bind(session_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(SessionRow::into_session))
    }

    pub async fn list_own_sessions(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<Vec<AuthSession>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            r#"
            select s.session_id, s.tenant_id, s.user_id, s.status, s.client_kind, s.client_label,
                   s.created_at, s.last_seen_at, s.expires_at, s.credentials_epoch,
                   coalesce(t.credentials_epoch, 0) as current_epoch
            from auth_sessions s
            join tenants t on t.tenant_id = s.tenant_id
            where s.tenant_id = $1
              and s.user_id = $2
            order by s.created_at desc
            limit 32
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .fetch_all(self.pool)
        .await?;
        Ok(rows.into_iter().map(SessionRow::into_session).collect())
    }

    /// Extends one active formal session. Renewal is not a new seat and not a new session.
    pub async fn renew_session(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        session_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Result<AuthSession, AccessError>> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, i32)> = sqlx::query_as(
            r#"
            select u.status, coalesce(t.credentials_epoch, 0)
            from users u
            join tenants t on t.tenant_id = u.tenant_id
            where u.tenant_id = $1 and u.user_id = $2
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .fetch_optional(&mut *tx)
        .await?;
        let Some((status, current_epoch)) = row else {
            tx.rollback().await?;
            return Ok(Err(AccessError::SessionInactive));
        };
        if status != "ACTIVE" {
            tx.rollback().await?;
            return Ok(Err(AccessError::SessionInactive));
        }
        let renewed: Option<SessionRow> = sqlx::query_as(
            r#"
            update auth_sessions
            set expires_at = $4, last_seen_at = now()
            where tenant_id = $1
              and user_id = $2
              and session_id = $3
              and status = 'ACTIVE'
              and credentials_epoch = $5
            returning session_id, tenant_id, user_id, status, client_kind, client_label,
                      created_at, last_seen_at, expires_at, credentials_epoch,
                      $5::integer as current_epoch
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .bind(session_id)
        .bind(expires_at)
        .bind(current_epoch)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(renewed) = renewed else {
            tx.rollback().await?;
            return Ok(Err(AccessError::SessionInactive));
        };
        let active_users: i64 = sqlx::query_scalar(
            "select count(*)::bigint from users where tenant_id = $1 and status = 'ACTIVE'",
        )
        .bind(Uuid::from(tenant_id))
        .fetch_one(&mut *tx)
        .await?;
        if active_users != 1 {
            tx.rollback().await?;
            return Ok(Err(AccessError::SeatLimitReached));
        }
        let business: i64 = sqlx::query_scalar(
            "select count(*) from inventory_ledgers where tenant_id = $1 and ref_id = $2",
        )
        .bind(Uuid::from(tenant_id))
        .bind(session_id.to_string())
        .fetch_one(&mut *tx)
        .await?;
        if business != 0 {
            tx.rollback().await?;
            anyhow::bail!("session renewal must not replay a business command");
        }
        tx.commit().await?;
        Ok(Ok(renewed.into_session()))
    }

    /// Replaces the formal password hash and advances the tenant epoch.
    /// The recovery token is consumed in the same transaction. A second use finds no row.
    pub async fn recover_formal_password(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        token_hash: &str,
        password_hash: &str,
    ) -> anyhow::Result<Result<i32, AccessError>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1::text), hashtext($2))")
            .bind(Uuid::from(tenant_id).to_string())
            .bind(format!("password-recover:{token_hash}"))
            .execute(&mut *tx)
            .await?;
        let consumed = sqlx::query(
            r#"
            update password_reset_tokens
            set used_at = now()
            where tenant_id = $1
              and user_id = $2
              and token_hash = $3
              and used_at is null
              and expires_at > now()
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .bind(token_hash)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if consumed != 1 {
            tx.rollback().await?;
            return Ok(Err(AccessError::SessionInactive));
        }
        let updated = sqlx::query(
            r#"
            update users
            set password_hash = $3, updated_at = now()
            where tenant_id = $1 and user_id = $2 and status = 'ACTIVE'
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .bind(password_hash)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if updated != 1 {
            tx.rollback().await?;
            return Ok(Err(AccessError::SessionInactive));
        }
        let epoch = self
            .bump_credentials_epoch_in_tx(&mut tx, tenant_id)
            .await?;
        let seats: i64 = sqlx::query_scalar(
            "select count(*)::bigint from users where tenant_id = $1 and status = 'ACTIVE'",
        )
        .bind(Uuid::from(tenant_id))
        .fetch_one(&mut *tx)
        .await?;
        if seats != 1 {
            tx.rollback().await?;
            return Ok(Err(AccessError::SeatLimitReached));
        }
        tx.commit().await?;
        Ok(Ok(epoch))
    }

    pub async fn revoke_own_session(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        session_id: Uuid,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            r#"
            update auth_sessions
            set status = 'REVOKED',
                revoked_at = now(),
                revoke_reason = 'USER'
            where tenant_id = $1
              and user_id = $2
              and session_id = $3
              and status = 'ACTIVE'
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(Uuid::from(user_id))
        .bind(session_id)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Advances the tenant credential epoch inside the caller's transaction.
    /// The password hash update must already be in that same transaction.
    pub async fn bump_credentials_epoch_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: TenantId,
    ) -> anyhow::Result<i32> {
        let epoch: i32 = sqlx::query_scalar(
            r#"
            update tenants
            set credentials_epoch = coalesce(credentials_epoch, 0) + 1,
                updated_at = now()
            where tenant_id = $1
            returning credentials_epoch
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .fetch_one(&mut **tx)
        .await?;
        Ok(epoch)
    }

    pub async fn bump_credentials_epoch(&self, tenant_id: TenantId) -> anyhow::Result<i32> {
        let epoch: i32 = sqlx::query_scalar(
            r#"
            update tenants
            set credentials_epoch = coalesce(credentials_epoch, 0) + 1,
                updated_at = now()
            where tenant_id = $1
            returning credentials_epoch
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .fetch_one(self.pool)
        .await?;
        Ok(epoch)
    }
}

fn stored_plan(row: (String, i32, String, i32, i32)) -> Result<StoredAccessPlan, AccessError> {
    let (plan_code, login_seat_limit, session_policy, max_active_sessions, credentials_epoch) = row;
    if login_seat_limit < 1 || !(1..=32).contains(&max_active_sessions) {
        return Err(AccessError::InvalidPolicy);
    }
    if plan_code != FREE_PLAN_CODE && plan_code != PAID_PLAN_CODE {
        return Err(AccessError::InvalidPolicy);
    }
    if plan_code == FREE_PLAN_CODE
        && (login_seat_limit != FREE_LOGIN_SEAT_LIMIT
            || max_active_sessions > FREE_MAX_ACTIVE_SESSIONS)
    {
        return Err(AccessError::InvalidPolicy);
    }
    Ok(StoredAccessPlan {
        plan: TenantAccessPlan {
            plan_code_free: plan_code == FREE_PLAN_CODE,
            login_seat_limit,
            session_policy: SessionPolicy::parse(&session_policy)?,
            max_active_sessions,
        },
        credentials_epoch,
    })
}

async fn insert_session(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    user_id: UserId,
    client_kind: &str,
    client_label: &str,
    expires_at: DateTime<Utc>,
    credentials_epoch: i32,
) -> anyhow::Result<AuthSession> {
    let kind = match client_kind {
        "DESKTOP" | "WEB" => client_kind,
        _ => "UNKNOWN",
    };
    let label: String = client_label.chars().take(80).collect();
    let row = sqlx::query_as::<_, SessionRow>(
        r#"
        insert into auth_sessions(
            session_id, tenant_id, user_id, status, client_kind, client_label, expires_at, credentials_epoch
        )
        values ($1, $2, $3, 'ACTIVE', $4, $5, $6, $7)
        returning session_id, tenant_id, user_id, status, client_kind, client_label,
                  created_at, last_seen_at, expires_at, credentials_epoch,
                  credentials_epoch as current_epoch
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::from(tenant_id))
    .bind(Uuid::from(user_id))
    .bind(kind)
    .bind(label)
    .bind(expires_at)
    .bind(credentials_epoch)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.into_session())
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,
    status: String,
    client_kind: String,
    client_label: String,
    created_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    credentials_epoch: i32,
    current_epoch: i32,
}

impl SessionRow {
    fn into_session(self) -> AuthSession {
        AuthSession {
            session_id: self.session_id,
            tenant_id: TenantId(self.tenant_id),
            user_id: UserId(self.user_id),
            status: self.status,
            client_kind: self.client_kind,
            client_label: self.client_label,
            created_at: self.created_at,
            last_seen_at: self.last_seen_at,
            expires_at: self.expires_at,
            credentials_epoch: self.credentials_epoch,
            current_epoch: self.current_epoch,
        }
    }
}
