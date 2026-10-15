//! Named row conversion preserves nullable exports, identity and scoped batch counts.
use super::{
    Result,
    inventory_commands::{Fixture, fixture},
    isolated_pool,
};
use erp_repo::{
    export_jobs::{ExportJobCreate, ExportJobsRepo, ProductExportCreateError},
    product_copy_batches,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn owned(pool: &PgPool) -> Result<Fixture> {
    let f = fixture(pool).await?;
    sqlx::query("update tenants set name='QA-QUALITY-NAMED-ROWS' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!("QA quality_rows tenant={} actor={}", f.tenant, f.actor);
    Ok(f)
}

#[tokio::test]
async fn named_export_row_keeps_nullable_fields_replay_and_cross_tenant_identity() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = owned(&pool).await?;
    let other = owned(&pool).await?;
    let request = Uuid::new_v4();
    let query = json!({"schema":1,"offset":0,"limit":100});
    let repo = ExportJobsRepo::new(&pool);
    let first = repo
        .create_product_job_idempotent(f.tenant, erp_types::UserId(f.actor), request, query.clone())
        .await?;
    assert_eq!(first.job_id, request);
    assert_eq!(first.tenant_id, f.tenant);
    assert_eq!(first.job_type, "PRODUCT_EXPORT");
    assert!(first.output_url.is_none());
    assert_eq!(first.progress, 0);
    assert_eq!(first.attempts, 0);
    assert_eq!(first.last_error, "");
    sqlx::query("update export_jobs set status='RUNNING',output_url='qa-owned://pending.csv',progress=37,attempts=2,last_error='QA retry detail',updated_at=clock_timestamp() where tenant_id=$1 and job_id=$2")
        .bind(f.tenant.0).bind(request).execute(&pool).await?;
    let replay = repo
        .create_product_job_idempotent(f.tenant, erp_types::UserId(f.actor), request, query.clone())
        .await?;
    assert_eq!(replay.status, "RUNNING");
    assert_eq!(replay.output_url.as_deref(), Some("qa-owned://pending.csv"));
    assert_eq!(replay.progress, 37);
    assert_eq!(replay.attempts, 2);
    assert_eq!(replay.last_error, "QA retry detail");
    assert_eq!(replay.created_at, first.created_at);
    assert!(replay.updated_at >= first.updated_at);
    assert_eq!(replay.params_json, first.params_json);
    assert!(matches!(
        repo.create_product_job_idempotent(
            other.tenant,
            erp_types::UserId(other.actor),
            request,
            query
        )
        .await,
        Err(ProductExportCreateError::Conflict)
    ));
    let events:i64=sqlx::query_scalar("select count(*) from audit_logs where tenant_id=$1 and action='EXPORT.JOB.CREATE' and entity_id=$2")
        .bind(f.tenant.0).bind(request.to_string()).fetch_one(&pool).await?;
    assert_eq!(events, 1);
    // The checked-selection path shares the same named persistence conversion.
    let checked = repo
        .create_product_checked_job_idempotent(
            f.tenant,
            erp_types::UserId(f.actor),
            Uuid::new_v4(),
            &[f.sku],
            &[],
        )
        .await;
    assert!(matches!(
        checked,
        Err(ProductExportCreateError::Invalid(ref reason)) if reason == "one or more checked products are unavailable"
    ));
    let product: Uuid =
        sqlx::query_scalar("select product_id from skus where tenant_id=$1 and sku_id=$2")
            .bind(f.tenant.0)
            .bind(f.sku)
            .fetch_one(&pool)
            .await?;
    let selected_id = Uuid::new_v4();
    let selected = repo
        .create_product_checked_job_idempotent(
            f.tenant,
            erp_types::UserId(f.actor),
            selected_id,
            &[product],
            &[],
        )
        .await?;
    let repeated = repo
        .create_product_checked_job_idempotent(
            f.tenant,
            erp_types::UserId(f.actor),
            selected_id,
            &[product],
            &[],
        )
        .await?;
    assert_eq!(selected.job_id, selected_id);
    assert_eq!(selected.tenant_id, f.tenant);
    assert_eq!(selected.params_json, repeated.params_json);
    assert_eq!(selected.created_at, repeated.created_at);
    assert_eq!(selected.status, "PENDING");
    assert!(selected.output_url.is_none());
    let membership: Vec<Uuid> = sqlx::query_scalar("select product_id from grid_product_selection_export_items where job_id=$1 order by ordinal")
        .bind(selected_id).fetch_all(&pool).await?;
    assert_eq!(membership, vec![product]);
    let ordinary = repo
        .create_job(ExportJobCreate {
            tenant_id: f.tenant,
            job_type: "QA-QUALITY-ROWS".into(),
            params_json: json!({"owned":true}),
        })
        .await?;
    assert_eq!(ordinary.tenant_id, f.tenant);
    Ok(())
}

#[tokio::test]
async fn named_batch_status_keeps_each_counter_unknown_and_current_actor_scope() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = owned(&pool).await?;
    let other = owned(&pool).await?;
    let batch = Uuid::new_v4();
    for source in [&f, &other] {
        sqlx::query("insert into grid_product_copy_batches(tenant_id,actor_user_id,batch_id,request,total,applied,rejected,cancelled,pending,unknown,cancel_requested) values($1,$2,$3,$4,10,2,1,1,4,2,true)")
            .bind(source.tenant.0).bind(source.actor).bind(batch).bind(json!({"batch_id":batch,"owned":true})).execute(&pool).await?;
    }
    let data = product_copy_batches::list(&pool, f.tenant, erp_types::UserId(f.actor)).await?;
    assert_eq!(data.len(), 1);
    let item = &data[0];
    assert_eq!(item.batch_id, batch);
    assert_eq!(
        (
            item.total,
            item.applied,
            item.rejected,
            item.cancelled,
            item.pending,
            item.unknown
        ),
        (10, 2, 1, 1, 4, 2)
    );
    assert!(item.cancel_requested);
    assert!(
        product_copy_batches::list(&pool, f.tenant, erp_types::UserId(other.actor))
            .await?
            .is_empty()
    );
    assert_eq!(
        product_copy_batches::list(&pool, other.tenant, erp_types::UserId(other.actor))
            .await?
            .len(),
        1
    );
    eprintln!(
        "QA quality_batch_counts tenant={} batch={batch} total=10 applied=2 rejected=1 cancelled=1 pending=4 unknown=2",
        f.tenant
    );
    Ok(())
}
