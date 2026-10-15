//! Fresh review snapshots for the rejected rows of a completed product-copy batch.
use crate::product_selection::SelectionToken;
use erp_types::{TenantId, UserId};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

const MAX_SNAPSHOT_ROWS: u64 = 100_000;
const MAX_ACTIVE_SNAPSHOTS: i64 = 4;

#[derive(Debug, Serialize)]
pub struct RetryReview {
    pub batch_id: Uuid,
    pub request_id: Uuid,
    pub selection: SelectionToken,
}

#[derive(Debug, thiserror::Error)]
pub enum RetryError {
    #[error("invalid product copy retry review: {0}")]
    Invalid(String),
    #[error("product copy batch is unavailable")]
    Unavailable,
    #[error("retry review identity was already used for another batch")]
    IdentityConflict,
    #[error("product copy batch still has unresolved items")]
    NotTerminal,
    #[error("product copy batch has no rejected items to review")]
    NoRejected,
    #[error("rejected source product is no longer available: {0}")]
    SourceUnavailable(Uuid),
    #[error("product selection capacity exceeded")]
    Capacity,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

pub async fn prepare_retry(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    original_batch: Uuid,
    request_id: Uuid,
) -> Result<RetryReview, RetryError> {
    if original_batch.is_nil() || request_id.is_nil() {
        return Err(RetryError::Invalid(
            "batch and request identities must be non-nil".into(),
        ));
    }
    let tenant_id = Uuid::from(tenant);
    let actor_id = Uuid::from(actor);
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level serializable")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout='30s'")
        .execute(&mut *tx)
        .await?;
    // Share the selection issuer's quota lock. Text parameters ensure this
    // hashes the UUID text rather than relying on an implicit binary cast.
    sqlx::query("select pg_advisory_xact_lock(hashtextextended($1::text || ':' || $2::text,0))")
        .bind(tenant_id.to_string())
        .bind(actor_id.to_string())
        .execute(&mut *tx)
        .await?;

    let previous: Option<(Uuid, Uuid, String, i64, i64)> = sqlx::query_as(
        "select original_batch_id,selection_token,query_fingerprint,total,floor(extract(epoch from expires_at))::bigint from grid_product_copy_batch_retry_requests where tenant_id=$1 and actor_user_id=$2 and request_id=$3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(request_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((stored_batch, token, fingerprint, total, expires_at)) = previous {
        if stored_batch != original_batch {
            return Err(RetryError::IdentityConflict);
        }
        let review = review(
            original_batch,
            request_id,
            token,
            fingerprint,
            total,
            expires_at,
        )?;
        tx.commit().await?;
        return Ok(review);
    }

    sqlx::query("delete from grid_product_selections where tenant_id=$1 and actor_user_id=$2 and expires_at<=clock_timestamp()")
        .bind(tenant_id)
        .bind(actor_id)
        .execute(&mut *tx)
        .await?;
    let active: i64 = sqlx::query_scalar(
        "select count(*) from grid_product_selections where tenant_id=$1 and actor_user_id=$2",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await?;
    if active >= MAX_ACTIVE_SNAPSHOTS {
        return Err(RetryError::Capacity);
    }

    let batch: Option<(i64, i64, i64, bool)> = sqlx::query_as(
        "select rejected,pending,unknown,completed_at is not null from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 for share",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(original_batch)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((rejected, pending, unknown, completed)) = batch else {
        return Err(RetryError::Unavailable);
    };
    if !completed || pending != 0 || unknown != 0 {
        return Err(RetryError::NotTerminal);
    }
    let total = u64::try_from(rejected)
        .map_err(|_| RetryError::Invalid("invalid stored rejected count".into()))?;
    if total == 0 {
        return Err(RetryError::NoRejected);
    }
    if total > MAX_SNAPSHOT_ROWS {
        return Err(RetryError::Capacity);
    }
    let frozen_rows: i64 = sqlx::query_scalar(
        "select count(*) from grid_product_selection_items i join grid_product_selections s on s.token=i.selection_token where s.tenant_id=$1 and s.actor_user_id=$2",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await?;
    let frozen_rows = u64::try_from(frozen_rows)
        .map_err(|_| RetryError::Invalid("invalid stored selection count".into()))?;
    if frozen_rows
        .checked_add(total)
        .is_none_or(|rows| rows > MAX_SNAPSHOT_ROWS)
    {
        return Err(RetryError::Capacity);
    }

    let unavailable: Option<Uuid> = sqlx::query_scalar(
        "select i.source_product_id from grid_product_copy_batch_items i left join products p on p.tenant_id=i.tenant_id and p.product_id=i.source_product_id and p.status<>'DELETED' where i.tenant_id=$1 and i.actor_user_id=$2 and i.batch_id=$3 and i.state='REJECTED' and p.product_id is null order by i.ordinal limit 1",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(original_batch)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(source) = unavailable {
        return Err(RetryError::SourceUnavailable(source));
    }
    let distinct: i64 = sqlx::query_scalar(
        "select count(distinct source_product_id) from grid_product_copy_batch_items where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 and state='REJECTED'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(original_batch)
    .fetch_one(&mut *tx)
    .await?;
    if u64::try_from(distinct).ok() != Some(total) {
        return Err(RetryError::Invalid(
            "rejected batch items contain duplicate sources".into(),
        ));
    }

    let token = Uuid::new_v4();
    let normalized = serde_json::json!({
        "kind": "product_copy_retry_review",
        "original_batch_id": original_batch,
    });
    let fingerprint: String = sqlx::query_scalar("select md5($1::jsonb::text)")
        .bind(&normalized)
        .fetch_one(&mut *tx)
        .await?;
    let expires_at: i64 = sqlx::query_scalar(
        "insert into grid_product_selections(token,tenant_id,actor_user_id,normalized_query,query_fingerprint,total,expires_at) values($1,$2,$3,$4,$5,$6,clock_timestamp()+interval '10 minutes') returning floor(extract(epoch from expires_at))::bigint",
    )
    .bind(token)
    .bind(tenant_id)
    .bind(actor_id)
    .bind(&normalized)
    .bind(&fingerprint)
    .bind(rejected)
    .fetch_one(&mut *tx)
    .await?;
    let inserted = sqlx::query(
        "insert into grid_product_selection_items(selection_token,ordinal,product_id,product_version,data) select $4,(row_number() over(order by p.product_id)-1)::bigint,p.product_id,p.updated_at,jsonb_build_object('product_id',p.product_id,'tenant_id',p.tenant_id,'product_code',p.product_code,'name',p.name,'brand',p.brand,'category',p.category,'status',p.status,'created_at',p.created_at,'updated_at',p.updated_at) from grid_product_copy_batch_items i join products p on p.tenant_id=i.tenant_id and p.product_id=i.source_product_id and p.status<>'DELETED' where i.tenant_id=$1 and i.actor_user_id=$2 and i.batch_id=$3 and i.state='REJECTED' order by p.product_id",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(original_batch)
    .bind(token)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if inserted != total {
        return Err(RetryError::Storage(sqlx::Error::Protocol(
            "rejected product membership changed while freezing retry review".into(),
        )));
    }
    sqlx::query(
        "insert into grid_product_copy_batch_retry_requests(tenant_id,actor_user_id,request_id,original_batch_id,selection_token,query_fingerprint,total,expires_at) select $1,$2,$3,$4,$5,$6,$7,expires_at from grid_product_selections where token=$5",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(request_id)
    .bind(original_batch)
    .bind(token)
    .bind(&fingerprint)
    .bind(rejected)
    .execute(&mut *tx)
    .await?;
    let result = review(
        original_batch,
        request_id,
        token,
        fingerprint,
        rejected,
        expires_at,
    )?;
    tx.commit().await?;
    Ok(result)
}

fn review(
    batch_id: Uuid,
    request_id: Uuid,
    token: Uuid,
    query_fingerprint: String,
    total: i64,
    expires_at: i64,
) -> Result<RetryReview, RetryError> {
    Ok(RetryReview {
        batch_id,
        request_id,
        selection: SelectionToken {
            token,
            query_fingerprint,
            total: u64::try_from(total)
                .map_err(|_| RetryError::Invalid("invalid stored retry count".into()))?,
            expires_at: u64::try_from(expires_at)
                .map_err(|_| RetryError::Invalid("invalid stored retry expiry".into()))?,
        },
    })
}
