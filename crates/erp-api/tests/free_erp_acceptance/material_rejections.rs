//! Definite new-command rejections differ from unknown SQL/auth/replay outcomes.
use super::{
    Result, isolated_pool,
    material_confirmation::{draft, line, owned, setup},
    material_documents::{read, save},
    production_receipt_release::http,
};
use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

#[tokio::test]
async fn material_rejection_stale_version_and_source_adoption_preserve_actual_draft() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let doc = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
    let path = format!("work-orders/{order}/material-documents/{doc}");
    let before = owned(&pool, &f).await?;
    let version_body = json!({"request_id":"version-reject","expected_version":2});
    let (status, response) = http(
        &f,
        &format!("{path}/confirm?rejection_envelope=true"),
        version_body.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["code"], "material_document_rejected_before_commit");
    assert_eq!(response["fields"], version_body);
    assert_eq!(response["committed"], false);
    assert_eq!(owned(&pool, &f).await?, before);
    let (status, legacy) = http(
        &f,
        &format!("{path}/confirm"),
        json!({"request_id":"legacy-version","expected_version":2}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(legacy["code"], "VERSION_CONFLICT");
    let (status, review) =
        super::material_source_review::get(&f, &format!("{path}/source-review")).await?;
    assert_eq!(status, StatusCode::OK, "{review}");
    sqlx::query("update mfg_work_orders set order_qty_micros=20000000 where tenant_id=$1 and work_order_id=$2").bind(f.tenant.0).bind(order).execute(&pool).await?;
    let before = owned(&pool, &f).await?;
    let body = json!({"request_id":"adoption-reject","expected_version":1,"review_token":review["review_token"]});
    let (status, response) = http(
        &f,
        &format!("{path}/source-adoptions?rejection_envelope=true"),
        body.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["code"], "material_document_rejected_before_commit");
    assert_eq!(response["fields"], body);
    assert_eq!(response["committed"], false);
    assert_eq!(
        response["context"],
        json!({"tenant_id":f.tenant,"actor_user_id":f.actor,"work_order_id":order,"document_id":doc})
    );
    assert_eq!(owned(&pool, &f).await?, before);
    assert_eq!(
        read(&f, order, doc, None).await?.1["snapshot"]["version_no"],
        1
    );
    Ok(())
}

#[tokio::test]
async fn material_rejection_source_change_binds_exact_fields_and_allows_fresh_correction()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let doc = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
    sqlx::query("update mfg_work_orders set order_qty_micros=20000000 where tenant_id=$1 and work_order_id=$2").bind(f.tenant.0).bind(order).execute(&pool).await?;
    let body = json!({"request_id":"reject-source","document_id":doc,"expected_version":1,"lines":[line(&f,"ISSUE","0.5",None)]});
    let mut body = body;
    body["lines"][0]["batch_id"] = Value::Null;
    let before = owned(&pool, &f).await?;
    let (status, response) = http(
        &f,
        &format!("work-orders/{order}/material-documents?rejection_envelope=true"),
        body.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["code"], "material_document_rejected_before_commit");
    assert_eq!(response["committed"], false);
    assert_eq!(response["request_id"], "reject-source");
    assert_eq!(response["fields"], body);
    assert_eq!(
        response["context"],
        json!({"tenant_id":f.tenant,"actor_user_id":f.actor,"work_order_id":order,"document_id":doc})
    );
    assert_eq!(owned(&pool, &f).await?, before);
    sqlx::query("update mfg_work_orders set order_qty_micros=10000000 where tenant_id=$1 and work_order_id=$2").bind(f.tenant.0).bind(order).execute(&pool).await?;
    let mut correction = body;
    correction["request_id"] = json!("fresh-correction");
    assert_eq!(save(&f, order, correction).await?.0, StatusCode::OK);
    assert_eq!(
        read(&f, order, doc, None).await?.1["snapshot"]["version_no"],
        2
    );
    Ok(())
}

#[tokio::test]
async fn material_rejection_last_line_rolls_back_and_preserves_original_confirm_fields()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let doc = draft(
        &f,
        order,
        vec![
            line(&f, "ISSUE", "0.25", None),
            line(&f, "SUPPLEMENT", "0.8", None),
        ],
    )
    .await?;
    let body = json!({"request_id":"confirm-reject","expected_version":1});
    let before = owned(&pool, &f).await?;
    let (status, response) = http(
        &f,
        &format!("work-orders/{order}/material-documents/{doc}/confirm?rejection_envelope=true"),
        body.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["code"], "material_document_rejected_before_commit");
    assert_eq!(response["fields"], body);
    assert_eq!(response["context"]["document_id"], json!(doc));
    assert_eq!(response["committed"], false);
    assert_eq!(owned(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn material_rejection_auth_denial_and_changed_original_command_never_unlock_as_rejected()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let doc = Uuid::new_v4();
    let body = json!({"request_id":"known-save","document_id":doc,"expected_version":0,"lines":[line(&f,"ISSUE","0.25",None)]});
    assert_eq!(save(&f, order, body.clone()).await?.0, StatusCode::OK);
    let mut changed = body.clone();
    changed["lines"][0]["qty"] = json!("0.5");
    let (status, response) = http(
        &f,
        &format!("work-orders/{order}/material-documents?rejection_envelope=true"),
        changed,
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_ne!(response["code"], "material_document_rejected_before_commit");
    sqlx::query(
        "delete from role_permissions where tenant_id=$1 and perm_code='MFG.PRODUCTION.WRITE'",
    )
    .bind(f.tenant.0)
    .execute(&pool)
    .await?;
    let (status, response) = http(
        &f,
        &format!("work-orders/{order}/material-documents?rejection_envelope=true"),
        body,
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_ne!(response["code"], "material_document_rejected_before_commit");
    Ok(())
}

#[tokio::test]
async fn material_rejection_real_sql_failure_has_no_definite_rejection_declaration() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let doc = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
    let before = owned(&pool, &f).await?;
    let name = format!("qa_material_reject_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if NEW.tenant_id='{}'::uuid and NEW.action='manufacturing.material.document.confirm' then raise exception 'owned SQL failure'; end if; return NEW; end $$",f.tenant.0)).execute(&pool).await?;
    sqlx::query(&format!(
        "create trigger {name} before insert on audit_logs for each row execute function {name}()"
    ))
    .execute(&pool)
    .await?;
    let result = http(
        &f,
        &format!("work-orders/{order}/material-documents/{doc}/confirm?rejection_envelope=true"),
        json!({"request_id":"sql-failure","expected_version":1}),
    )
    .await;
    sqlx::query(&format!("drop trigger {name} on audit_logs"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop function {name}()"))
        .execute(&pool)
        .await?;
    let (status, response) = result?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_ne!(response["code"], "material_document_rejected_before_commit");
    assert_eq!(owned(&pool, &f).await?, before);
    Ok(())
}
