use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use erp_repo::sku_serials::{ShipmentSerialBindInput, SkuSerialsRepo};
use erp_types::TenantId;
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn setup_schema(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists tenants (
          tenant_id uuid primary key default gen_random_uuid(),
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shops (
          shop_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          platform_shop_id text not null,
          shop_name text not null default '',
          status text not null default 'ACTIVE',
          credentials jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, platform_shop_id)
        )
        "#,
        r#"
        create table if not exists warehouses (
          warehouse_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          code text not null,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, code)
        )
        "#,
        r#"
        create table if not exists products (
          product_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          brand text,
          category text,
          attrs jsonb not null default '{}'::jsonb,
          allow_negative_stock boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          product_id uuid not null references products(product_id),
          sku_code text not null,
          barcode text,
          name text,
          spec jsonb not null default '{}'::jsonb,
          weight_gram int,
          length_mm int,
          width_mm int,
          height_mm int,
          volume_cm3 int,
          sku_kind text not null default 'NORMAL',
          cost_cent bigint,
          is_active boolean not null default true,
          status text not null default 'ACTIVE',
          batch_enabled boolean not null default false,
          serial_enabled boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_code)
        )
        "#,
        r#"
        create table if not exists sku_batches (
          batch_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          sku_id uuid not null references skus(sku_id),
          batch_no text not null,
          production_date timestamptz,
          expire_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_id, batch_no)
        )
        "#,
        r#"
        create table if not exists shipments (
            shipment_id uuid primary key default gen_random_uuid(),
            tenant_id uuid not null references tenants(tenant_id),
            shop_id uuid not null references shops(shop_id),
            order_id uuid not null,
            platform text not null,
            platform_order_id text not null,
            warehouse_id uuid references warehouses(warehouse_id),
            carrier_code text,
            tracking_no text,
            status text not null default 'REQUESTED',
            shipment_status text not null default 'REQUESTED',
            status_version int not null default 0,
            requested_at timestamptz,
            shipped_at timestamptz,
            updated_at timestamptz not null default now(),
            unique (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists outbound_docs (
          outbound_doc_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          doc_type text not null,
          shipment_id uuid references shipments(shipment_id),
          doc_no text not null,
          status text not null default 'DRAFT',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists outbound_doc_items (
          tenant_id uuid not null references tenants(tenant_id),
          outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          planned_qty bigint not null,
          picked_qty bigint not null default 0,
          packed_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, outbound_doc_id, sku_id)
        )
        "#,
        r#"
        create table if not exists outbound_pick_tasks (
          pick_task_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          from_location_id uuid,
          qty bigint not null,
          picked_qty bigint not null default 0,
          status text not null default 'PENDING',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists pack_records (
          pack_record_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
          shipment_id uuid references shipments(shipment_id),
          package_no text not null,
          weight_gram int,
          operator_id uuid,
          packed_at timestamptz not null default now(),
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists sku_serials (
          serial_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          sku_id uuid not null references skus(sku_id),
          serial_no text not null,
          status text not null default 'IN_STOCK',
          ref_doc_type text,
          ref_doc_id text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, serial_no)
        )
        "#,
        r#"
        create table if not exists shipment_item_serials (
          tenant_id uuid not null references tenants(tenant_id),
          shipment_id uuid not null references shipments(shipment_id),
          order_id uuid not null,
          order_item_id uuid not null,
          sku_id uuid not null references skus(sku_id),
          serial_id uuid not null references sku_serials(serial_id),
          serial_no text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, shipment_id, serial_id)
        )
        "#,
        r#"
        create table if not exists shipment_sku_batches (
          tenant_id uuid not null references tenants(tenant_id),
          shipment_id uuid not null references shipments(shipment_id),
          sku_id uuid not null references skus(sku_id),
          batch_id uuid not null references sku_batches(batch_id),
          batch_no text not null,
          qty bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, shipment_id, sku_id, batch_id)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

fn build_token(jwt: &JwtService, tenant_id: &str) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: vec!["wms.write".to_string()],
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn wms_outbound_ship_requires_serials() -> anyhow::Result<()> {
    if let Err(err) = wms_outbound_ship_requires_serials_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping wms_outbound_ship_requires_serials: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn wms_outbound_ship_requires_serials_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let outbound_doc_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name, status)
        values ($1, $2, $3, $4, $5, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-OUT-1")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-OUT-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("Serial Product")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, serial_enabled)
        values ($1, $2, $3, $4, true)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("SERIAL-SKU-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
            warehouse_id, status, updated_at
        )
        values ($1, $2, $3, $4, $5, $6, $7, 'REQUESTED', now())
        "#,
    )
    .bind(shipment_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind(order_id)
    .bind("taobao")
    .bind("PLAT-ORDER-OUT-1")
    .bind(warehouse_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into outbound_docs (
            outbound_doc_id, tenant_id, warehouse_id, doc_type, shipment_id, doc_no, status
        ) values ($1, $2, $3, $4, $5, $6, 'PACKED')
        "#,
    )
    .bind(outbound_doc_id)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind("SHIP")
    .bind(shipment_id)
    .bind("OUT-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into outbound_doc_items (
            tenant_id, outbound_doc_id, sku_id, planned_qty, picked_qty, packed_qty
        ) values ($1, $2, $3, $4, $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(outbound_doc_id)
    .bind(sku_id)
    .bind(1_i64)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/ship"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&serde_json::json!({}))?))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("conflict"));
    assert!(json
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .contains("serial"));

    Ok(())
}

#[tokio::test]
async fn serial_binding_keeps_shipment_ref_and_marks_shipped() -> anyhow::Result<()> {
    if let Err(err) = serial_binding_keeps_shipment_ref_and_marks_shipped_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping serial_binding_keeps_shipment_ref_and_marks_shipped: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn serial_binding_keeps_shipment_ref_and_marks_shipped_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let second_shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let second_order_id = uuid::Uuid::new_v4();
    let order_item_id = uuid::Uuid::new_v4();
    let second_order_item_id = uuid::Uuid::new_v4();
    let serial_no = "SERIAL-WMS-001";

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name, status)
        values ($1, $2, $3, $4, $5, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-SERIAL-1")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-SERIAL-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("Serial Product")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, serial_enabled)
        values ($1, $2, $3, $4, true)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("SERIAL-SKU-TRACK-1")
    .execute(db.pool())
    .await?;

    for (shipment_id_value, order_id_value, platform_order_id) in [
        (shipment_id, order_id, "PLAT-ORDER-SERIAL-1"),
        (second_shipment_id, second_order_id, "PLAT-ORDER-SERIAL-2"),
    ] {
        sqlx::query(
            r#"
            insert into shipments (
                shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
                warehouse_id, status, updated_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, 'REQUESTED', now())
            "#,
        )
        .bind(shipment_id_value)
        .bind(tenant_id)
        .bind(shop_id)
        .bind(order_id_value)
        .bind("taobao")
        .bind(platform_order_id)
        .bind(warehouse_id)
        .execute(db.pool())
        .await?;
    }

    sqlx::query(
        r#"
        insert into sku_serials (tenant_id, sku_id, serial_no, status)
        values ($1, $2, $3, 'IN_STOCK')
        "#,
    )
    .bind(tenant_id)
    .bind(sku_id)
    .bind(serial_no)
    .execute(db.pool())
    .await?;

    let repo = SkuSerialsRepo::new(db.pool());
    let tenant = TenantId(tenant_id);

    let bound = repo
        .bind_serials_to_shipment(ShipmentSerialBindInput {
            tenant_id: tenant,
            shipment_id,
            order_id,
            order_item_id,
            sku_id,
            serial_nos: vec![serial_no.to_string()],
        })
        .await?;

    assert_eq!(bound.len(), 1);
    assert_eq!(bound[0].status, "ALLOCATED");
    assert_eq!(bound[0].ref_doc_type.as_deref(), Some("SHIPMENT"));
    assert_eq!(bound[0].ref_doc_id.as_deref(), Some(shipment_id.to_string().as_str()));

    let duplicate_err = repo
        .bind_serials_to_shipment(ShipmentSerialBindInput {
            tenant_id: tenant,
            shipment_id: second_shipment_id,
            order_id: second_order_id,
            order_item_id: second_order_item_id,
            sku_id,
            serial_nos: vec![serial_no.to_string()],
        })
        .await
        .expect_err("serial should not be available for another shipment");
    let duplicate_message = duplicate_err.to_string();
    assert!(duplicate_message.contains("serial not available"));
    assert!(duplicate_message.contains("status=ALLOCATED"));
    assert!(duplicate_message.contains("ref_doc_type=SHIPMENT"));
    assert!(duplicate_message.contains(&shipment_id.to_string()));

    repo.mark_shipment_serials_shipped(tenant, shipment_id).await?;

    #[derive(sqlx::FromRow)]
    struct SerialRow {
        status: String,
        ref_doc_type: Option<String>,
        ref_doc_id: Option<String>,
    }

    let serial_row: SerialRow = sqlx::query_as(
        r#"
        select status, ref_doc_type, ref_doc_id
        from sku_serials
        where tenant_id = $1 and sku_id = $2 and serial_no = $3
        "#,
    )
    .bind(tenant_id)
    .bind(sku_id)
    .bind(serial_no)
    .fetch_one(db.pool())
    .await?;

    assert_eq!(serial_row.status, "SHIPPED");
    assert_eq!(serial_row.ref_doc_type.as_deref(), Some("SHIPMENT"));
    assert_eq!(
        serial_row.ref_doc_id.as_deref(),
        Some(shipment_id.to_string().as_str())
    );

    Ok(())
}

#[tokio::test]
async fn wms_outbound_ship_blocks_expired_batch() -> anyhow::Result<()> {
    if let Err(err) = wms_outbound_ship_blocks_expired_batch_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping wms_outbound_ship_blocks_expired_batch: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn wms_outbound_ship_blocks_expired_batch_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let outbound_doc_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let batch_id = uuid::Uuid::new_v4();
    let batch_no = "BATCH-EXPIRED-001";

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name, status)
        values ($1, $2, $3, $4, $5, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-BATCH-1")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-BATCH-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("Food Product")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, batch_enabled)
        values ($1, $2, $3, $4, true)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("BATCH-SKU-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
            warehouse_id, status, updated_at
        )
        values ($1, $2, $3, $4, $5, $6, $7, 'REQUESTED', now())
        "#,
    )
    .bind(shipment_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind(order_id)
    .bind("taobao")
    .bind("PLAT-ORDER-BATCH-1")
    .bind(warehouse_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into outbound_docs (
            outbound_doc_id, tenant_id, warehouse_id, doc_type, shipment_id, doc_no, status
        ) values ($1, $2, $3, $4, $5, $6, 'PACKED')
        "#,
    )
    .bind(outbound_doc_id)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind("SHIP")
    .bind(shipment_id)
    .bind("OUT-BATCH-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into outbound_doc_items (
            tenant_id, outbound_doc_id, sku_id, planned_qty, picked_qty, packed_qty
        ) values ($1, $2, $3, $4, $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(outbound_doc_id)
    .bind(sku_id)
    .bind(1_i64)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into sku_batches (
            batch_id, tenant_id, sku_id, batch_no, production_date, expire_at
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(batch_id)
    .bind(tenant_id)
    .bind(sku_id)
    .bind(batch_no)
    .bind(Utc::now() - Duration::days(30))
    .bind(Utc::now() - Duration::days(1))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipment_sku_batches (
            tenant_id, shipment_id, sku_id, batch_id, batch_no, qty
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(tenant_id)
    .bind(shipment_id)
    .bind(sku_id)
    .bind(batch_id)
    .bind(batch_no)
    .bind(1_i64)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/ship"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&serde_json::json!({}))?))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("conflict"));
    let message = json
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    assert!(message.contains("batch expired before shipping"));
    assert!(message.contains(batch_no));
    assert!(message.contains(&sku_id.to_string()));

    Ok(())
}

#[tokio::test]
async fn wms_outbound_ship_blocks_future_production_batch() -> anyhow::Result<()> {
    if let Err(err) = wms_outbound_ship_blocks_future_production_batch_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping wms_outbound_ship_blocks_future_production_batch: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn wms_outbound_ship_blocks_future_production_batch_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let outbound_doc_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let batch_id = uuid::Uuid::new_v4();
    let batch_no = "BATCH-PROD-FUTURE-001";

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name, status)
        values ($1, $2, $3, $4, $5, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-BATCH-2")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-BATCH-2")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("Food Product Future")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, batch_enabled)
        values ($1, $2, $3, $4, true)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("BATCH-SKU-2")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
            warehouse_id, status, updated_at
        )
        values ($1, $2, $3, $4, $5, $6, $7, 'REQUESTED', now())
        "#,
    )
    .bind(shipment_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind(order_id)
    .bind("taobao")
    .bind("PLAT-ORDER-BATCH-2")
    .bind(warehouse_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into outbound_docs (
            outbound_doc_id, tenant_id, warehouse_id, doc_type, shipment_id, doc_no, status
        ) values ($1, $2, $3, $4, $5, $6, 'PACKED')
        "#,
    )
    .bind(outbound_doc_id)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind("SHIP")
    .bind(shipment_id)
    .bind("OUT-BATCH-2")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into outbound_doc_items (
            tenant_id, outbound_doc_id, sku_id, planned_qty, picked_qty, packed_qty
        ) values ($1, $2, $3, $4, $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(outbound_doc_id)
    .bind(sku_id)
    .bind(1_i64)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into sku_batches (
            batch_id, tenant_id, sku_id, batch_no, production_date, expire_at
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(batch_id)
    .bind(tenant_id)
    .bind(sku_id)
    .bind(batch_no)
    .bind(Utc::now() + Duration::days(3))
    .bind(Utc::now() + Duration::days(40))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipment_sku_batches (
            tenant_id, shipment_id, sku_id, batch_id, batch_no, qty
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(tenant_id)
    .bind(shipment_id)
    .bind(sku_id)
    .bind(batch_id)
    .bind(batch_no)
    .bind(1_i64)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/ship"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&serde_json::json!({}))?))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("conflict"));
    let message = json
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    assert!(message.contains("batch production date in future before shipping"));
    assert!(message.contains(batch_no));
    assert!(message.contains(&sku_id.to_string()));

    Ok(())
}
