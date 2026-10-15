//! Real work order selection preserves full query semantics and missing source facts.
use super::{Result, inventory_commands::Fixture, isolated_pool, material_documents::context};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{AppState, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn get(f: &Fixture, path: &str) -> Result<(StatusCode, Value)> {
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let result = build_app(AppState::new(
        Db::connect_lazy(&database)?,
        f.login.jwt.clone(),
    ))
    .oneshot(
        Request::builder()
            .method("GET")
            .uri(format!("/v1/tenants/{}/manufacturing/{path}", f.tenant))
            .header("authorization", format!("Bearer {}", f.login.token))
            .body(Body::empty())?,
    )
    .await?;
    let status = result.status();
    let bytes = result.into_body().collect().await?.to_bytes();
    let response = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA manufacturing_catalog tenant={} path={path} status={} response={response}",
        f.tenant,
        status.as_u16()
    );
    Ok((status, response))
}
async fn setup(pool: &PgPool) -> Result<(Fixture, Uuid)> {
    let (f, order) = context(pool).await?;
    sqlx::query("update tenants set name='QA-MANUFACTURING-CATALOG' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!(
        "QA manufacturing_catalog_fixture tenant={} order={order}",
        f.tenant
    );
    Ok((f, order))
}
async fn source(pool: &PgPool, f: &Fixture, order: Uuid, missing: bool) -> Result<Uuid> {
    let bom = Uuid::new_v4();
    let version = Uuid::new_v4();
    sqlx::query("insert into mfg_boms(tenant_id,bom_id,product_code,status,version,base_qty_micros) values($1,$2,'QA-P','PUBLISHED',1,1000000)").bind(f.tenant.0).bind(bom).execute(pool).await?;
    sqlx::query("insert into shared_mfg_bom_versions(tenant_id,version_id,series_code,version_no,status,start_date,source_bom_id) values($1,$2,'QA-P',1,'PUBLISHED','2026-10-01',$3)").bind(f.tenant.0).bind(version).bind(bom).execute(pool).await?;
    sqlx::query("insert into shared_mfg_bom_version_lines(tenant_id,version_id,line_no,parent_code,component_code,qty_per_micros,unit_code,scrap_ppm,supply_kind) values($1,$2,1,'QA-P','QA-COMMAND',1000000,'kg',0,'PURCHASE')").bind(f.tenant.0).bind(version).execute(pool).await?;
    if missing {
        sqlx::query("insert into shared_mfg_bom_version_lines(tenant_id,version_id,line_no,parent_code,component_code,qty_per_micros,unit_code,scrap_ppm,supply_kind) values($1,$2,2,'QA-P','UNMAPPED-MATERIAL',2000000,'kg',0,'PURCHASE')").bind(f.tenant.0).bind(version).execute(pool).await?;
    }
    sqlx::query("update mfg_work_orders set bom_id=$3 where tenant_id=$1 and work_order_id=$2")
        .bind(f.tenant.0)
        .bind(order)
        .bind(version)
        .execute(pool)
        .await?;
    Ok(version)
}

#[tokio::test]
async fn manufacturing_catalog_literal_search_full_range_and_business_ids_are_real() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, _) = setup(&pool).await?;
    for (index, name, state) in [
        (1, "WO_%_ONE", "RELEASED"),
        (2, "WO_%_TWO", "PAUSED"),
        (3, "WO_%_THREE", "PLANNED"),
        (4, "WO_XX_FOUR", "RELEASED"),
        (5, "WO_%_CLOSED", "CLOSED"),
    ] {
        sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,$3,'P',$4,$5)")
            .bind(f.tenant.0).bind(Uuid::new_v4()).bind(name).bind(state).bind(9007199254740993_i64+index).execute(&pool).await?;
    }
    let (status, a) = get(&f, "work-order-options?query=WO_%25_&offset=0&limit=2").await?;
    assert_eq!(status, StatusCode::OK, "{a}");
    let (_, b) = get(&f, "work-order-options?query=WO_%25_&offset=2&limit=2").await?;
    assert_eq!(a["total"], 3);
    assert_eq!(b["total"], 3);
    assert_eq!(a["data"].as_array().map(Vec::len), Some(2));
    assert_eq!(b["data"].as_array().map(Vec::len), Some(1));
    let ids = a["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("missing page"))?
        .iter()
        .chain(
            b["data"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("missing page"))?,
        )
        .map(|r| r["work_order_id"].as_str().unwrap_or_default())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 3);
    for page in [&a, &b] {
        for row in page["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing data"))?
        {
            assert!(
                row["work_order_no"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("WO_%_"))
            );
            assert!(
                row["order_qty_micros"]
                    .as_str()
                    .is_some_and(|s| s.parse::<i64>().is_ok())
            );
        }
    }
    assert_eq!(
        get(
            &f,
            "work-order-options?query=WO_%25_&open_only=false&offset=0&limit=10"
        )
        .await?
        .1["total"],
        4
    );
    assert_eq!(
        get(
            &f,
            "work-order-options?query=WO_%25_&status=PAUSED&offset=0&limit=10"
        )
        .await?
        .1["total"],
        1
    );
    Ok(())
}

#[tokio::test]
async fn manufacturing_catalog_missing_mapping_and_current_warehouse_scope_remain_visible_facts()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    source(&pool, &f, order, true).await?;
    let (status, value) = get(&f, &format!("work-orders/{order}/material-context")).await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["materials"].as_array().map(Vec::len), Some(2));
    assert_eq!(value["materials"][0]["sku_id"], json!(f.sku));
    assert!(value["materials"][1]["sku_id"].is_null());
    assert_eq!(value["materials"][1]["component_code"], "UNMAPPED-MATERIAL");
    assert_eq!(value["needs_reconciliation"], true);
    assert_eq!(value["warehouses"].as_array().map(Vec::len), Some(1));
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    let (_, empty) = get(&f, &format!("work-orders/{order}/material-context")).await?;
    assert_eq!(empty["warehouses"], json!([]));
    assert_eq!(
        get(
            &f,
            &format!("work-orders/{}/material-context", Uuid::new_v4())
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    Ok(())
}

#[tokio::test]
async fn manufacturing_catalog_known_context_rechecks_read_role_and_active_session() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    source(&pool, &f, order, false).await?;
    let (_, value) = get(&f, &format!("work-orders/{order}/material-context")).await?;
    assert_eq!(value["needs_reconciliation"], false);
    sqlx::query(
        "delete from role_permissions where tenant_id=$1 and perm_code='manufacturing.read'",
    )
    .bind(f.tenant.0)
    .execute(&pool)
    .await?;
    assert_eq!(
        get(&f, "work-order-options?offset=0&limit=10").await?.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&f, &format!("work-orders/{order}/material-context"))
            .await?
            .0,
        StatusCode::FORBIDDEN
    );
    let (other, other_order) = setup(&pool).await?;
    source(&pool, &other, other_order, false).await?;
    sqlx::query("update auth_sessions set status='REVOKED' where tenant_id=$1 and session_id=$2")
        .bind(other.tenant.0)
        .bind(other.login.session)
        .execute(&pool)
        .await?;
    assert_eq!(
        get(&other, "work-order-options?offset=0&limit=10").await?.0,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[tokio::test]
async fn manufacturing_catalog_complete_context_budget_rejects_1001_rows_instead_of_truncating()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let version = source(&pool, &f, order, false).await?;
    sqlx::query("insert into shared_mfg_bom_version_lines(tenant_id,version_id,line_no,parent_code,component_code,qty_per_micros,unit_code,scrap_ppm,supply_kind) select $1,$2,n,'QA-P','UNMAPPED-'||n::text,1000000,'kg',0,'PURCHASE' from generate_series(2,1001) n")
        .bind(f.tenant.0).bind(version).execute(&pool).await?;
    assert_eq!(
        get(&f, &format!("work-orders/{order}/material-context"))
            .await?
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        get(&f, "work-order-options?offset=0&limit=2001").await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    Ok(())
}
