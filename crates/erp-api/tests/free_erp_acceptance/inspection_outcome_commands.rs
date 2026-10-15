//! Actual deferred command identity and composite reviewer ownership failures.
use super::{
    Result,
    inspection_outcome_edges::{failed, owned_facts, passing_reinspection},
    inspection_outcomes::frozen,
    isolated_pool,
    production_receipt_release::Receipt,
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

async fn insert_reading(
    tx: &mut Transaction<'_, Postgres>,
    r: &Receipt,
    freeze: Uuid,
    item: Uuid,
) -> Result<Value> {
    let count=sqlx::query("insert into mfg_quality_items(tenant_id,item_id,work_order_id,batch_code,item_code,required,min_micros,max_micros,reading_micros,reading_present,passed,inspector,freeze_id,source_kind,source_id) select f.tenant_id,$3,r.work_order_id,f.batch_code,l.item_code,l.required,l.min_micros,l.max_micros,1500000,true,true,'QA-INSPECTOR',f.freeze_id,f.source_kind,f.source_id from shared_inspection_freezes f join shared_inspection_freeze_lines l on l.tenant_id=f.tenant_id and l.freeze_id=f.freeze_id join mfg_receipts r on r.tenant_id=f.tenant_id and r.receipt_id=f.source_id where f.tenant_id=$1 and f.freeze_id=$2 and l.item_code='WIDTH'")
        .bind(r.f.tenant.0).bind(freeze).bind(item).execute(&mut **tx).await?.rows_affected();
    assert_eq!(count, 1);
    let q: Value = sqlx::query_scalar(
        "select to_jsonb(q) from mfg_quality_items q where tenant_id=$1 and item_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(item)
    .fetch_one(&mut **tx)
    .await?;
    Ok(
        json!({"schema":1,"tenant_id":r.f.tenant,"item_id":item,"freeze_id":freeze,"item_code":q["item_code"],
        "source_kind":q["source_kind"],"source_id":q["source_id"],"batch_code":q["batch_code"],
        "required":q["required"],"min_micros":q["min_micros"].as_i64().ok_or_else(||anyhow::anyhow!("frozen minimum"))?.to_string(),
        "max_micros":q["max_micros"].as_i64().ok_or_else(||anyhow::anyhow!("frozen maximum"))?.to_string(),
        "reading_micros":"1500000","reading_present":true,"passed":true,"verdict":"PASSED",
        "inspector":"QA-INSPECTOR","declared_reviewer":"DECLARED","actor_source":"USER","actor_user_id":r.f.actor,
        "attempt_no":1,"previous_item_id":Value::Null,"reason":"","review_required":false}),
    )
}

async fn insert_event(
    tx: &mut Transaction<'_, Postgres>,
    r: &Receipt,
    snapshot: &Value,
    request: &str,
    payload: &str,
) -> Result<()> {
    sqlx::query("insert into shared_inspection_reading_events(tenant_id,item_id,freeze_id,item_code,attempt_no,previous_item_id,review_required,verdict,declared_reviewer,actor_source,actor_user_id,reason,request_id,payload_hash,snapshot) values($1,$2,$3,'WIDTH',1,null,false,'PASSED','DECLARED','USER',$4,'',$5,$6,$7)")
        .bind(r.f.tenant.0).bind(Uuid::parse_str(snapshot["item_id"].as_str().ok_or_else(||anyhow::anyhow!("item"))?)?)
        .bind(Uuid::parse_str(snapshot["freeze_id"].as_str().ok_or_else(||anyhow::anyhow!("freeze"))?)?)
        .bind(r.f.actor).bind(request).bind(payload).bind(snapshot).execute(&mut **tx).await?;
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_deferred_original_command_missing_payload_or_effect_cannot_commit()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-original-command-guard").await?;
    let before = owned_facts(&pool, &r).await?;
    for kind in ["missing-command", "wrong-payload", "wrong-effect"] {
        let item = Uuid::new_v4();
        let request = Uuid::new_v4().to_string();
        let payload =
            serde_json::to_string(&(1, freeze, "WIDTH", 1_500_000, "QA-INSPECTOR", "DECLARED"))?;
        let mut tx = pool.begin().await?;
        let snapshot = insert_reading(&mut tx, &r, freeze, item).await?;
        insert_event(&mut tx, &r, &snapshot, &request, &payload).await?;
        if kind != "missing-command" {
            sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id) values($1,$2,$3,'SUCCEEDED',$4)")
                .bind(r.f.tenant.0).bind(&request).bind(if kind=="wrong-payload" { "different-full-canonical-payload" } else { &payload })
                .bind(if kind=="wrong-effect" { Uuid::new_v4().to_string() } else { item.to_string() }).execute(&mut *tx).await?;
        }
        let committed = tx.commit().await;
        eprintln!(
            "QA inspection_outcome_command_guard tenant={} case={kind} request={request} item={item} committed={} error={:?}",
            r.f.tenant,
            committed.is_ok(),
            committed.as_ref().err()
        );
        assert!(committed.is_err(), "{kind} must fail at actual commit");
        assert_eq!(before, owned_facts(&pool, &r).await?);
    }
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_missing_snapshot_identity_cannot_insert_a_pass() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-snapshot-identity-guard").await?;
    let before = owned_facts(&pool, &r).await?;
    let mut tx = pool.begin().await?;
    let item = Uuid::new_v4();
    let request = Uuid::new_v4().to_string();
    let mut snapshot = insert_reading(&mut tx, &r, freeze, item).await?;
    snapshot
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("snapshot object"))?
        .remove("actor_user_id");
    let inserted = insert_event(&mut tx, &r, &snapshot, &request, "original-full-payload").await;
    tx.rollback().await?;
    eprintln!(
        "QA inspection_outcome_command_guard tenant={} case=missing-snapshot-actor request={request} item={item} committed=false refused={}",
        r.f.tenant,
        inserted.is_err()
    );
    assert!(
        inserted.is_err(),
        "omitted identity cannot exploit SQL NULL comparisons"
    );
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_review_cannot_borrow_another_enterprises_user() -> Result<()> {
    let pool: PgPool = isolated_pool().await?;
    let (foreign, _) = frozen(&pool, "outcome-foreign-review-user").await?;
    let (r, freeze) = frozen(&pool, "outcome-owned-review-user").await?;
    let first = failed(&r, freeze).await?;
    let item = passing_reinspection(&r, freeze, first).await?;
    let before = owned_facts(&pool, &r).await?;
    let foreign_before = owned_facts(&pool, &foreign).await?;
    let reading: Value = sqlx::query_scalar(
        "select snapshot from shared_inspection_reading_events where tenant_id=$1 and item_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(item)
    .fetch_one(&pool)
    .await?;
    let id = Uuid::new_v4();
    let request = Uuid::new_v4().to_string();
    let snapshot = json!({"schema":1,"tenant_id":r.f.tenant,"review_id":id,"item_id":item,"freeze_id":freeze,
        "reviewer_user_id":foreign.f.actor,"decision":"APPROVE","reason":"","reading":reading});
    let mut tx = pool.begin().await?;
    let inserted=sqlx::query("insert into shared_inspection_reading_reviews(tenant_id,review_id,item_id,reviewer_user_id,decision,reason,request_id,payload_hash,snapshot) values($1,$2,$3,$4,'APPROVE','',$5,'full-original-payload',$6)")
        .bind(r.f.tenant.0).bind(id).bind(item).bind(foreign.f.actor).bind(&request).bind(snapshot).execute(&mut *tx).await;
    tx.rollback().await?;
    eprintln!(
        "QA inspection_outcome_command_guard tenant={} case=foreign-review-user request={request} item={item} committed=false refused={}",
        r.f.tenant,
        inserted.is_err()
    );
    assert!(
        inserted.is_err(),
        "reviewer ownership is a real composite foreign key"
    );
    assert_eq!(before, owned_facts(&pool, &r).await?);
    assert_eq!(foreign_before, owned_facts(&pool, &foreign).await?);
    Ok(())
}
