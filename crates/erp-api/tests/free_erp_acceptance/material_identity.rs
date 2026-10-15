//! Formal material HTTP requests retain physical and original issue identity.

use super::account_login::{AccountLogin, STOCK_PERMISSIONS, create_user};
use super::{Result, isolated_pool};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, build_app};
use erp_domain::manufacturing::{MaterialMove, MaterialMoveKind};
use erp_repo::{
    db::Db,
    shared_contract::{MaterialMoveInput, SharedContractRepo, SharedWrite},
};
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    tenant: Uuid,
    warehouse: Uuid,
    sku_a: Uuid,
    sku_b: Uuid,
    order: Uuid,
    login: Option<AccountLogin>,
}

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let mut f = Fixture {
        tenant: Uuid::new_v4(),
        warehouse: Uuid::new_v4(),
        sku_a: Uuid::new_v4(),
        sku_b: Uuid::new_v4(),
        order: Uuid::new_v4(),
        login: None,
    };
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-MATERIAL-IDENTITY')")
        .bind(f.tenant)
        .execute(pool)
        .await?;
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Material QA','QA-MAT')")
        .bind(f.tenant).bind(f.warehouse).execute(pool).await?;
    for (sku, code) in [(f.sku_a, "QA-A"), (f.sku_b, "QA-B")] {
        let opening = Uuid::new_v4();
        super::masters::sku(pool, f.tenant, sku, code).await?;
        sqlx::query("insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand) values($1,$2,$3,1)")
            .bind(f.tenant).bind(f.warehouse).bind(sku).execute(pool).await?;
        sqlx::query("insert into mfg_qty_balances(tenant_id,warehouse_id,sku_id,on_hand_micros) values($1,$2,$3,1000000)")
            .bind(f.tenant).bind(f.warehouse).bind(sku).execute(pool).await?;
        sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_in_transit,delta_frozen,reason_type,ref_type,ref_id) values($1,$2,$3,1,0,0,0,'QA_OPENING','mfg_material_move',$4)")
            .bind(f.tenant).bind(f.warehouse).bind(sku).bind(opening.to_string()).execute(pool).await?;
        sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,1000000,'QA_OPENING')")
            .bind(f.tenant).bind(f.warehouse).bind(sku).bind(opening).execute(pool).await?;
    }
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-MAT-WO','QA-P','RELEASED',1000000)")
        .bind(f.tenant).bind(f.order).execute(pool).await?;
    let actor = Uuid::new_v4();
    let login = create_user(
        pool,
        TenantId(f.tenant),
        actor,
        format!("{actor}@material.qa.invalid"),
        STOCK_PERMISSIONS,
    )
    .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant)
    .bind(actor)
    .bind(f.warehouse)
    .execute(pool)
    .await?;
    f.login = Some(login);
    eprintln!("QA material identity tenant={} order={}", f.tenant, f.order);
    Ok(f)
}

fn app_and_token(f: &Fixture) -> Result<(axum::Router, String)> {
    let login = f
        .login
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing actual material login"))?;
    Ok((
        build_app(AppState::new(
            Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
            login.jwt.clone(),
        )),
        login.token.clone(),
    ))
}

async fn post(
    app: &axum::Router,
    f: &Fixture,
    token: &str,
    body: Value,
) -> Result<(StatusCode, Vec<u8>)> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{}/manufacturing/work-orders/{}/materials",
                    f.tenant, f.order
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    let status = response.status();
    Ok((
        status,
        response.into_body().collect().await?.to_bytes().to_vec(),
    ))
}

#[tokio::test]
async fn material_http_cannot_consume_another_sku_issue() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let (app, token) = app_and_token(&f)?;
    let (status, bytes) = post(
        &app,
        &f,
        &token,
        json!({"request_id":"issue-a","kind":"ISSUE","qty":"1",
        "warehouse_id":f.warehouse,"sku_id":f.sku_a}),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let issue: Uuid = sqlx::query_scalar(
        "select move_id from mfg_material_moves where tenant_id=$1 and kind='ISSUE'",
    )
    .bind(f.tenant)
    .fetch_one(&mut *reader)
    .await?;
    let (status, bytes) = post(
        &app,
        &f,
        &token,
        json!({"request_id":"consume-wrong","kind":"CONSUME","qty":"0.25",
        "warehouse_id":f.warehouse,"sku_id":f.sku_b,"original_issue_id":issue}),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "another material was accepted: {}",
        String::from_utf8_lossy(&bytes)
    );
    let (moves,receipts):(i64,i64)=sqlx::query_as("select (select count(*) from mfg_material_moves where tenant_id=$1),(select count(*) from command_receipts where tenant_id=$1)")
        .bind(f.tenant).fetch_one(&mut *reader).await?;
    assert_eq!((moves, receipts), (1, 1));
    let rows: Vec<(Uuid, i64)> =
        sqlx::query_as("select sku_id,on_hand from inventory_balances where tenant_id=$1")
            .bind(f.tenant)
            .fetch_all(&mut *reader)
            .await?;
    assert!(rows.contains(&(f.sku_a, 0)) && rows.contains(&(f.sku_b, 1)));
    let identity: (Uuid, Uuid) = sqlx::query_as(
        "select warehouse_id,sku_id from mfg_material_moves where tenant_id=$1 and move_id=$2",
    )
    .bind(f.tenant)
    .bind(issue)
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(identity, (f.warehouse, f.sku_a));
    Ok(())
}

pub(super) async fn seed_identity(
    pool: &PgPool,
    tenant: Uuid,
    warehouse: Uuid,
    sku: Uuid,
) -> Result<()> {
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Material QA',$3) on conflict(warehouse_id) do nothing")
        .bind(tenant).bind(warehouse).bind(warehouse.to_string()).execute(pool).await?;
    super::masters::sku(pool, tenant, sku, &sku.to_string()).await?;
    sqlx::query(
        "insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand) values($1,$2,$3,1)",
    )
    .bind(tenant)
    .bind(warehouse)
    .bind(sku)
    .execute(pool)
    .await?;
    sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_in_transit,delta_frozen,reason_type,ref_type,ref_id) values($1,$2,$3,1,0,0,0,'QA_OPENING','mfg_material_move',$4)")
        .bind(tenant).bind(warehouse).bind(sku).bind(Uuid::new_v4().to_string()).execute(pool).await?;
    Ok(())
}

fn movement(
    f: &Fixture,
    kind: MaterialMoveKind,
    quantity: &str,
    source: Option<Uuid>,
) -> MaterialMoveInput<'static> {
    MaterialMoveInput {
        work_order_id: f.order,
        warehouse_id: f.warehouse,
        sku_id: f.sku_a,
        batch_id: None,
        original_issue_id: source,
        movement: MaterialMove {
            kind,
            qty: super::qty(quantity),
            supplement_reason_required: false,
        },
        reason: None,
    }
}

async fn effect_count(reader: &mut sqlx::PgConnection, tenant: Uuid) -> Result<(i64, i64)> {
    Ok(sqlx::query_as("select (select count(*) from mfg_material_moves where tenant_id=$1),(select count(*) from command_receipts where tenant_id=$1)")
        .bind(tenant).fetch_one(reader).await?)
}

#[tokio::test]
async fn material_http_identity_conflicts_and_replays_original_snapshot() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let (app, token) = app_and_token(&f)?;
    let body = json!({"request_id":"same","kind":"ISSUE","qty":"1","warehouse_id":f.warehouse,"sku_id":f.sku_a});
    let (status, bytes) = post(&app, &f, &token, body.clone()).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let original: Value = serde_json::from_slice(&bytes)?;
    let issue = Uuid::parse_str(
        original["move_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("move_id missing"))?,
    )?;
    let mut changed = body.clone();
    changed["sku_id"] = json!(f.sku_b);
    assert_eq!(
        post(&app, &f, &token, changed).await?.0,
        StatusCode::CONFLICT
    );
    let (status,bytes)=post(&app,&f,&token,json!({"request_id":"consume","kind":"CONSUME","qty":"0.25","warehouse_id":f.warehouse,"sku_id":f.sku_a,"original_issue_id":issue})).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let mut retry = body;
    retry["qty"] = json!("1.000000");
    let (status, bytes) = post(&app, &f, &token, retry).await?;
    assert_eq!(status, StatusCode::OK);
    let replay: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(replay["move_id"], original["move_id"]);
    assert_eq!(replay["shop_qty_micros"], json!("1000000"));
    assert_eq!(replay["consumed_micros"], json!("0"));
    assert_eq!(replay["replay"], true);
    assert_eq!(effect_count(&mut reader, f.tenant).await?, (2, 2));
    Ok(())
}

#[tokio::test]
async fn material_http_missing_foreign_and_changed_source_have_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let foreign = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let (app, token) = app_and_token(&f)?;
    let repo = SharedContractRepo::new(&pool);
    let first = repo
        .post_material_move(
            TenantId(f.tenant),
            "seed",
            "seed",
            movement(&f, MaterialMoveKind::Issue, "1", None),
        )
        .await?
        .map_err(|e| anyhow::anyhow!("seed refused: {e:?}"))?;
    for body in [
        json!({"request_id":"missing","kind":"ISSUE","qty":"1"}),
        json!({"request_id":"foreign","kind":"ISSUE","qty":"1","warehouse_id":foreign.warehouse,"sku_id":f.sku_a}),
        json!({"request_id":"no-source","kind":"RETURN","qty":"0.1","warehouse_id":f.warehouse,"sku_id":f.sku_a}),
        json!({"request_id":"bad-source","kind":"RETURN","qty":"0.1","warehouse_id":f.warehouse,"sku_id":f.sku_a,"original_issue_id":Uuid::new_v4()}),
        json!({"request_id":"wrong-sku","kind":"RETURN","qty":"0.1","warehouse_id":f.warehouse,"sku_id":f.sku_b,"original_issue_id":first.move_id}),
    ] {
        let expected = if body["request_id"] == "foreign" {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        };
        let (status, bytes) = post(&app, &f, &token, body).await?;
        assert_eq!(status, expected, "{}", String::from_utf8_lossy(&bytes));
    }
    assert_eq!(effect_count(&mut reader, f.tenant).await?, (1, 1));
    Ok(())
}

#[tokio::test]
async fn material_source_cannot_borrow_another_issue_of_the_same_sku() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    sqlx::query("update inventory_balances set on_hand=2 where tenant_id=$1 and sku_id=$2")
        .bind(f.tenant)
        .bind(f.sku_a)
        .execute(&pool)
        .await?;
    sqlx::query(
        "update mfg_qty_balances set on_hand_micros=2000000 where tenant_id=$1 and sku_id=$2",
    )
    .bind(f.tenant)
    .bind(f.sku_a)
    .execute(&pool)
    .await?;
    let extra_opening = Uuid::new_v4();
    sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_in_transit,delta_frozen,reason_type,ref_type,ref_id) values($1,$2,$3,1,0,0,0,'QA_EXTRA_OPENING','mfg_material_move',$4)")
        .bind(f.tenant).bind(f.warehouse).bind(f.sku_a).bind(extra_opening.to_string()).execute(&pool).await?;
    sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,1000000,'QA_EXTRA_OPENING')")
        .bind(f.tenant).bind(f.warehouse).bind(f.sku_a).bind(extra_opening).execute(&pool).await?;
    let repo = SharedContractRepo::new(&pool);
    let first = repo
        .post_material_move(
            TenantId(f.tenant),
            "first",
            "first",
            movement(&f, MaterialMoveKind::Issue, "1", None),
        )
        .await?
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    repo.post_material_move(
        TenantId(f.tenant),
        "second",
        "second",
        movement(&f, MaterialMoveKind::Issue, "1", None),
    )
    .await?
    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    repo.post_material_move(
        TenantId(f.tenant),
        "consume-first",
        "consume-first",
        movement(&f, MaterialMoveKind::Consume, "1", Some(first.move_id)),
    )
    .await?
    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    assert!(
        repo.post_material_move(
            TenantId(f.tenant),
            "over",
            "over",
            movement(&f, MaterialMoveKind::Return, "0.125", Some(first.move_id))
        )
        .await
        .is_err()
    );
    let (spent,held):(i64,i64)=sqlx::query_as("select (sum(qty_micros) filter(where kind='CONSUME'))::bigint,max(material_shop_micros) filter(where kind='CONSUME') from mfg_material_moves where tenant_id=$1")
        .bind(f.tenant).fetch_one(&mut *reader).await?;
    assert_eq!((spent, held), (1000000, 1000000));
    assert_eq!(effect_count(&mut reader, f.tenant).await?, (3, 3));
    Ok(())
}

#[tokio::test]
async fn material_batch_keeps_original_identity_and_real_whole_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    for (batch, number, quantity) in [(a, "QA-BA", 1_i64), (b, "QA-BB", 0_i64)] {
        sqlx::query(
            "insert into sku_batches(tenant_id,sku_id,batch_id,batch_no) values($1,$2,$3,$4)",
        )
        .bind(f.tenant)
        .bind(f.sku_a)
        .bind(batch)
        .bind(number)
        .execute(&pool)
        .await?;
        // Controlled whole-unit fixture facts have an explicit exact opening.
        sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) values($1,$2,$3,$4,$5,0)")
            .bind(f.tenant).bind(f.warehouse).bind(f.sku_a).bind(batch).bind(quantity*1_000_000).execute(&pool).await?;
    }
    let repo = SharedContractRepo::new(&pool);
    let mut input = movement(&f, MaterialMoveKind::Issue, "1", None);
    input.batch_id = Some(a);
    let first = repo
        .post_material_move(TenantId(f.tenant), "batch-issue", "batch-issue", input)
        .await?
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let mut input = movement(&f, MaterialMoveKind::Return, "1", Some(first.move_id));
    input.batch_id = Some(b);
    assert!(
        repo.post_material_move(TenantId(f.tenant), "wrong-batch", "wrong-batch", input)
            .await
            .is_err()
    );
    input.batch_id = Some(a);
    repo.post_material_move(TenantId(f.tenant), "batch-return", "batch-return", input)
        .await?
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let rows: Vec<(Uuid, i64)> =
        sqlx::query_as("select batch_id,on_hand from inventory_batch_balances where tenant_id=$1")
            .bind(f.tenant)
            .fetch_all(&mut *reader)
            .await?;
    assert!(rows.contains(&(a, 1)) && rows.contains(&(b, 0)));
    let ledger:(i64,i64)=sqlx::query_as("select sum(delta_on_hand)::bigint,(select on_hand from inventory_balances where tenant_id=$1 and sku_id=$2) from inventory_ledgers where tenant_id=$1 and sku_id=$2")
        .bind(f.tenant).bind(f.sku_a).fetch_one(&mut *reader).await?;
    assert_eq!(ledger, (1, 1));
    assert_eq!(effect_count(&mut reader, f.tenant).await?, (2, 2));
    Ok(())
}

#[tokio::test]
async fn material_twenty_same_issues_keep_one_effect_under_real_fence() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let mut blocker = pool.begin().await?;
    sqlx::query(
        "select pg_advisory_xact_lock(hashtext($1),hashtext('material-command:qa-same-issue'))",
    )
    .bind(f.tenant.to_string())
    .execute(&mut *blocker)
    .await?;
    let blocker_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    let input = movement(&f, MaterialMoveKind::Issue, "1", None);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let pool = pool.clone();
        let tenant = TenantId(f.tenant);
        tasks.spawn(async move {
            SharedContractRepo::new(&pool)
                .post_material_move(tenant, "qa-same-issue", "same", input)
                .await
        });
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut waiting = 0_i64;
    while waiting < 2 && tokio::time::Instant::now() < deadline {
        waiting = waiting_descendants(&mut reader, blocker_pid).await?;
        if waiting < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    blocker.commit().await?;
    let mut writes = 0;
    let mut identities = std::collections::BTreeSet::new();
    let mut outcomes = Vec::new();
    while let Some(result) = tasks.join_next().await {
        outcomes.push(result);
    }
    assert_eq!(outcomes.len(), 20, "all actual results must be collected");
    for result in outcomes {
        let result = result??.map_err(|e| anyhow::anyhow!("{e:?}"))?;
        writes += usize::from(matches!(result.effect, SharedWrite::Stored(_)));
        identities.insert(result.move_id);
    }
    assert!(
        waiting >= 2,
        "observe the actual command/order blocking chain"
    );
    eprintln!(
        "QA material_same_issue tenant={} blocker_pid={blocker_pid} waiting_descendants={waiting} results=20 stored={writes}",
        f.tenant
    );
    assert_eq!((writes, identities.len()), (1, 1));
    assert_eq!(effect_count(&mut reader, f.tenant).await?, (1, 1));
    Ok(())
}

pub(super) async fn waiting_descendants(
    connection: &mut sqlx::PgConnection,
    blocker_pid: i32,
) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "with recursive blocked(pid) as (
        select a.pid from pg_stat_activity a where a.datname=current_database()
         and a.usename=current_user and $1=any(pg_blocking_pids(a.pid))
        union
        select a.pid from pg_stat_activity a join blocked b on b.pid=any(pg_blocking_pids(a.pid))
         where a.datname=current_database() and a.usename=current_user)
        select count(distinct pid)::bigint from blocked",
    )
    .bind(blocker_pid)
    .fetch_one(connection)
    .await?)
}

#[tokio::test]
async fn material_late_stock_failure_rolls_back_material_and_reservation() -> Result<()> {
    use erp_domain::quantity::{ScaledQty, parse_qty};
    use erp_repo::{
        inventory::InventoryService,
        inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
    };
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: TenantId(f.tenant),
                warehouse_id: f.warehouse,
                sku_id: f.sku_a,
            },
            on_hand: ScaledQty::ZERO,
            reserved: parse_qty("0.500")?,
            frozen: ScaledQty::ZERO,
            in_transit: ScaledQty::ZERO,
            reason_type: "QA_RESERVE".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    let source = Uuid::new_v4();
    sqlx::query("insert into inventory_reservations(tenant_id,reservation_id,warehouse_id,sku_id,qty,qty_micros,status,source_type,source_id) values($1,$2,$3,$4,0,500000,'RESERVED','MFG_WORK_ORDER',$5)")
        .bind(f.tenant).bind(source).bind(f.warehouse).bind(f.sku_a).bind(f.order.to_string()).execute(&pool).await?;
    // Only this new QA tenant fails at the later quantity-detail insert, after
    // the source reservation and canonical physical balance were updated.
    let injection = format!("qa_material_failure_{}", f.tenant.simple());
    sqlx::query(&format!("create function {injection}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid and new.reason_type='MFG_ISSUE' then raise exception 'QA late material detail failure'; end if; return new; end $$",f.tenant)).execute(&pool).await?;
    sqlx::query(&format!("create trigger {injection} before insert on mfg_qty_ledgers for each row execute function {injection}()")).execute(&pool).await?;
    let result = SharedContractRepo::new(&pool)
        .post_material_move(
            TenantId(f.tenant),
            "late",
            "late",
            movement(&f, MaterialMoveKind::Issue, "0.250", None),
        )
        .await;
    sqlx::query(&format!("drop trigger {injection} on mfg_qty_ledgers"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop function {injection}()"))
        .execute(&pool)
        .await?;
    let error = result
        .err()
        .ok_or_else(|| anyhow::anyhow!("late injected material write was accepted"))?;
    assert!(
        error
            .to_string()
            .contains("QA late material detail failure")
    );
    assert_eq!(effect_count(&mut reader, f.tenant).await?, (0, 0));
    let (quantity,reserved,ledger,source_qty):(i64,i64,i64,i64)=sqlx::query_as("select on_hand_micros,reserved_micros,(select count(*) from inventory_ledgers where tenant_id=$1 and reason_type='MFG_ISSUE'),(select qty_micros from inventory_reservations where tenant_id=$1 and reservation_id=$3) from inventory_balances where tenant_id=$1 and sku_id=$2")
        .bind(f.tenant).bind(f.sku_a).bind(source).fetch_one(&mut *reader).await?;
    assert_eq!(
        (quantity, reserved, ledger, source_qty),
        (1000000, 500000, 0, 500000)
    );
    Ok(())
}
