//! Genuine account reads reconcile receipt-local source facts, not fixed examples.
use super::account_login::{AccountLogin, create_user};
use super::{Result, isolated_pool};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub(super) struct Fixture {
    pub(super) tenant: TenantId,
    pub(super) account: AccountLogin,
    pub(super) warehouse: Uuid,
    pub(super) location: Uuid,
    pub(super) sku: Uuid,
}

pub(super) async fn fixture(pool: &PgPool, case: &str) -> Result<Fixture> {
    let tenant = TenantId(Uuid::new_v4());
    let actor = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-TRADE-FACTS')")
        .bind(tenant.0)
        .execute(pool)
        .await?;
    let account = create_user(
        pool,
        tenant,
        actor,
        format!("{actor}@trade-facts.qa.invalid"),
        &["orders.read", "orders.write", "purchase_orders.write"],
    )
    .await?;
    let f = Fixture {
        tenant,
        account,
        warehouse: Uuid::new_v4(),
        location: Uuid::new_v4(),
        sku: Uuid::new_v4(),
    };
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Trade facts QA',$3)")
        .bind(tenant.0).bind(f.warehouse).bind(f.warehouse.to_string()).execute(pool).await?;
    super::masters::sku(pool, f.tenant.0, f.sku, "QA-TRADE-FACT").await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(tenant.0)
    .bind(actor)
    .bind(f.warehouse)
    .execute(pool)
    .await?;
    eprintln!(
        "QA trade_facts_fixture tenant={tenant} actor={actor} case={case} warehouse={} sku={}",
        f.warehouse, f.sku
    );
    Ok(f)
}

pub(super) struct SourceFacts {
    pub(super) arrived: i64,
    pub(super) released: i64,
    pub(super) returned: i64,
    pub(super) issued: i64,
    pub(super) unit_cent: i64,
    pub(super) release_costs: Vec<i64>,
    pub(super) return_cost: i64,
}

pub(super) async fn seed(pool: &PgPool, f: &Fixture, facts: &SourceFacts) -> Result<Uuid> {
    let quality = Uuid::new_v4();
    let receipt = Uuid::new_v4();
    let po = Uuid::new_v4();
    let revision = Uuid::new_v4();
    let supplier = format!("SUP-{receipt}");
    let batch = format!("LOT-{receipt}");
    let gross: i64 = facts.release_costs.iter().sum();
    let mut tx = pool.begin().await?;
    sqlx::query("insert into shared_purchase_revisions(tenant_id,revision_id,po_id,version_no,status,supplier_code,warehouse_code) values($1,$2,$3,1,'OPEN',$4,$5)")
        .bind(f.tenant.0).bind(revision).bind(po).bind(&supplier).bind(f.warehouse.to_string()).execute(&mut *tx).await?;
    sqlx::query("insert into shared_purchase_lines(tenant_id,revision_id,line_no,item_code,qty_micros,price_cent,tax_cent,due_date) values($1,$2,1,'QA-TRADE-FACT',$3,$4,0,'2030-01-01')")
        .bind(f.tenant.0).bind(revision).bind(facts.arrived).bind(facts.unit_cent).execute(&mut *tx).await?;
    sqlx::query("insert into shared_purchase_receipts(tenant_id,receipt_id,po_id,line_no,qty_micros) values($1,$2,$3,1,$4)")
        .bind(f.tenant.0).bind(receipt).bind(po).bind(facts.arrived).execute(&mut *tx).await?;
    sqlx::query("insert into shared_receipt_quality(tenant_id,quality_id,receipt_id,po_id,line_no,warehouse_id,location_id,sku_id,supplier_code,batch_code,arrived_micros,released_micros,issued_micros,unit_cost_cent,payable_cent) values($1,$2,$3,$4,1,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
        .bind(f.tenant.0).bind(quality).bind(receipt).bind(po).bind(f.warehouse).bind(f.location).bind(f.sku)
        .bind(&supplier).bind(&batch).bind(facts.arrived).bind(facts.released).bind(facts.issued).bind(facts.unit_cent).bind(gross-facts.return_cost).execute(&mut *tx).await?;
    for (bucket, status, delta) in [
        ("HELD", "HELD", facts.arrived),
        ("HELD", "HELD", -facts.released),
        ("ON_HAND", "GOOD", facts.released),
        ("ON_HAND", "GOOD", -facts.returned),
        ("ON_HAND", "GOOD", -facts.issued),
    ] {
        if delta == 0 {
            continue;
        }
        sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(f.tenant.0).bind(Uuid::new_v4()).bind(f.warehouse).bind(f.location).bind(f.sku).bind(&batch)
            .bind(status).bind(bucket).bind(delta).bind(quality.to_string()).bind(Uuid::new_v4().to_string()).execute(&mut *tx).await?;
    }
    for amount in &facts.release_costs {
        sqlx::query("insert into partner_balances(tenant_id,balance_id,kind,partner_code,currency,source_type,source_id,amount_cent) values($1,$2,'AP',$3,'CNY','PURCHASE_RELEASE',$4,$5)")
            .bind(f.tenant.0).bind(Uuid::new_v4()).bind(&supplier).bind(receipt.to_string()).bind(amount).execute(&mut *tx).await?;
    }
    if facts.returned > 0 {
        sqlx::query("insert into shared_supplier_returns(tenant_id,return_id,receipt_id,qty_micros,cost_cent) values($1,$2,$3,$4,$5)")
            .bind(f.tenant.0).bind(Uuid::new_v4()).bind(receipt).bind(facts.returned).bind(facts.return_cost).execute(&mut *tx).await?;
        sqlx::query("insert into partner_balances(tenant_id,balance_id,kind,partner_code,currency,source_type,source_id,amount_cent) values($1,$2,'AP',$3,'CNY','PURCHASE_RETURN',$4,$5)")
            .bind(f.tenant.0).bind(Uuid::new_v4()).bind(&supplier).bind(receipt.to_string()).bind(-facts.return_cost).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(quality)
}

async fn saved(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let mut result = serde_json::Map::new();
    for table in [
        "shared_receipt_quality",
        "shared_purchase_receipts",
        "shared_stock_ledgers",
        "shared_supplier_returns",
        "partner_balances",
        "command_receipts",
        "users",
        "user_roles",
        "role_permissions",
        "auth_sessions",
        "user_warehouse_access",
    ] {
        let rows: Value = sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"))
            .bind(f.tenant.0).fetch_one(pool).await?;
        result.insert(table.into(), rows);
    }
    Ok(Value::Object(result))
}

async fn read(pool: &PgPool, f: &Fixture, quality: Uuid) -> Result<(StatusCode, Value)> {
    let before = saved(pool, f).await?;
    let result = f
        .account
        .request(
            "GET",
            &format!(
                "/v1/tenants/{}/purchase-receipt-offsets?quality_id={quality}",
                f.tenant
            ),
            Value::Null,
        )
        .await?;
    assert!(
        saved(pool, f).await? == before,
        "receipt reconciliation read has zero writes"
    );
    Ok(result)
}

#[tokio::test]
async fn trade_facts_receipt_partial_rounding_and_same_sku_are_isolated() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "partial-rounding-same-sku").await?;
    let quality = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 7_000_000,
            released: 5_000_000,
            returned: 1_250_000,
            issued: 500_000,
            unit_cent: 1599,
            release_costs: vec![3998, 3998],
            return_cost: 1999,
        },
    )
    .await?;
    let other = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 11_000_000,
            released: 11_000_000,
            returned: 0,
            issued: 0,
            unit_cent: 1000,
            release_costs: vec![11000],
            return_cost: 0,
        },
    )
    .await?;
    sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id) values($1,'unrelated-held','owned pending fixture','RUNNING','')")
        .bind(f.tenant.0).execute(&pool).await?;
    let (status, body) = read(&independent, &f, quality).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    for (field, value) in [
        ("arrived", 7_000_000),
        ("released", 5_000_000),
        ("returned_qty", 1_250_000),
        ("issued_qty", 500_000),
        ("released_cost_cent", 7996),
        ("returned_cost", 1999),
        ("payable_cent", 5997),
        ("held_micros", 2_000_000),
        ("on_hand_micros", 3_250_000),
        ("issue_rows", 1),
        ("over_return_rows", 0),
    ] {
        assert_eq!(body[field], json!(value), "{field}: {body}");
    }
    assert_eq!(body["status"], "OFFSET");
    let other_body = read(&independent, &f, other).await?;
    assert_eq!(other_body.0, StatusCode::OK, "{}", other_body.1);
    assert_eq!(other_body.1["on_hand_micros"], 11_000_000);
    let foreign = fixture(&pool, "foreign").await?;
    let denied = foreign
        .account
        .request(
            "GET",
            &format!(
                "/v1/tenants/{}/purchase-receipt-offsets?quality_id={quality}",
                f.tenant
            ),
            Value::Null,
        )
        .await?;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    eprintln!(
        "QA trade_facts_result tenant={} case=partial-rounding-same-sku quality={quality} other_quality={other} status=200 zero_writes=true payable=5997 stock=3250000",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn trade_facts_known_zero_release_needs_its_actual_posting() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "known-zero-missing-posting").await?;
    let quality = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 2_000_000,
            released: 1_000_000,
            returned: 0,
            issued: 0,
            unit_cent: 0,
            release_costs: vec![0],
            return_cost: 0,
        },
    )
    .await?;
    let first = read(&independent, &f, quality).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    assert_eq!(first.1["status"], "NO_OFFSET");
    assert_eq!(first.1["released_cost_cent"], 0);
    let missing = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 2_000_000,
            released: 1_000_000,
            returned: 0,
            issued: 0,
            unit_cent: 0,
            release_costs: vec![],
            return_cost: 0,
        },
    )
    .await?;
    let refused = read(&independent, &f, missing).await?;
    assert_eq!(refused.0, StatusCode::CONFLICT, "{}", refused.1);
    eprintln!(
        "QA trade_facts_result tenant={} case=known-zero-missing-posting quality={quality} status=200 rejected=409 zero_writes=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn trade_facts_receipt_inconsistent_payable_is_refused_without_effects() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "payable-conflict").await?;
    let quality = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 7_000_000,
            released: 5_000_000,
            returned: 2_000_000,
            issued: 0,
            unit_cent: 1000,
            release_costs: vec![5000],
            return_cost: 2000,
        },
    )
    .await?;
    sqlx::query("update shared_receipt_quality set payable_cent=payable_cent+1 where tenant_id=$1 and quality_id=$2")
        .bind(f.tenant.0).bind(quality).execute(&pool).await?;
    let refused = read(&independent, &f, quality).await?;
    assert_eq!(refused.0, StatusCode::CONFLICT, "{}", refused.1);
    eprintln!(
        "QA trade_facts_result tenant={} case=payable-conflict quality={quality} rejected=409 zero_writes=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn trade_facts_current_warehouse_and_role_revocation_refuse_old_token() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "current-authority").await?;
    let quality = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 7_000_000,
            released: 5_000_000,
            returned: 2_000_000,
            issued: 0,
            unit_cent: 1000,
            release_costs: vec![5000],
            return_cost: 2000,
        },
    )
    .await?;
    let first = read(&independent, &f, quality).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    sqlx::query(
        "delete from user_warehouse_access where tenant_id=$1 and user_id=$2 and warehouse_id=$3",
    )
    .bind(f.tenant.0)
    .bind(f.account.actor)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    let no_warehouse = read(&independent, &f, quality).await?;
    assert_eq!(no_warehouse.0, StatusCode::NOT_FOUND, "{}", no_warehouse.1);
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant.0)
    .bind(f.account.actor)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and role_id in(select role_id from user_roles where tenant_id=$1 and user_id=$2)")
        .bind(f.tenant.0).bind(f.account.actor).execute(&pool).await?;
    let no_role = read(&independent, &f, quality).await?;
    assert_eq!(no_role.0, StatusCode::NOT_FOUND, "{}", no_role.1);
    eprintln!(
        "QA trade_facts_result tenant={} case=current-authority quality={quality} status=200 warehouse_rejected=404 role_rejected=404 old_token=true zero_writes=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn trade_facts_storage_fault_is_explicit_without_business_writes() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "storage-fault").await?;
    let quality = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 7_000_000,
            released: 5_000_000,
            returned: 2_000_000,
            issued: 0,
            unit_cent: 1000,
            release_costs: vec![5000],
            return_cost: 2000,
        },
    )
    .await?;
    let schema = format!("qa_receipt_fault_{}", f.tenant.0.simple());
    sqlx::query(&format!("create schema {schema}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("create function {schema}.fail_offset(t uuid,q uuid) returns boolean language plpgsql as $$ begin if t='{}'::uuid and q='{quality}'::uuid then raise exception 'owned receipt read fault'; end if; return true; end $$",f.tenant))
        .execute(&pool).await?;
    sqlx::query(&format!("create view {schema}.shared_receipt_quality as select * from public.shared_receipt_quality x where {schema}.fail_offset(x.tenant_id,x.quality_id)"))
        .execute(&pool).await?;
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let joiner = if url.contains('?') { '&' } else { '?' };
    let app = f.account.app(&format!(
        "{url}{joiner}options=-csearch_path%3D{schema}%2Cpublic"
    ))?;
    let before = saved(&independent, &f).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/tenants/{}/purchase-receipt-offsets?quality_id={quality}",
                    f.tenant
                ))
                .header("authorization", format!("Bearer {}", f.account.token))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "actual receipt SQL read failure must not become invalid quantity or an empty success"
    );
    let bytes = response.into_body().collect().await?.to_bytes();
    assert_eq!(bytes.as_ref(), b"receipt offset storage unavailable");
    assert!(saved(&independent, &f).await? == before);
    eprintln!(
        "QA trade_facts_result tenant={} case=storage-fault quality={quality} rejected=500 schema={schema} zero_writes=true",
        f.tenant
    );
    Ok(())
}
