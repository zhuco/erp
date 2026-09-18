use anyhow::Result;
use chrono::{NaiveTime, Utc};
use erp_repo::wms::{
    OutboundDocCreateInput, OutboundDocItemInput, WaveCreateInput, WaveRuleCreateInput, WmsRepo,
};
use erp_types::TenantId;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

async fn setup_schema(pool: &PgPool) -> Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists tenants (
          tenant_id uuid primary key,
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists warehouses (
          warehouse_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          carrier_code text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists outbound_docs (
          outbound_doc_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          doc_type text not null,
          shipment_id uuid,
          doc_no text not null,
          status text not null default 'DRAFT',
          outbound_status text not null default 'DRAFT',
          pick_status text not null default 'NOT_PICKED',
          verify_status text not null default 'NOT_VERIFIED',
          pack_status text not null default 'NOT_PACKED',
          ship_status text not null default 'NOT_SHIPPED',
          picked_at timestamptz,
          packed_at timestamptz,
          shipped_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists outbound_doc_items (
          tenant_id uuid not null references tenants(tenant_id),
          outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
          sku_id uuid not null,
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
          sku_id uuid not null,
          qty bigint not null,
          picked_qty bigint not null default 0,
          status text not null default 'NOT_PICKED',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists waves (
          wave_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          status text not null default 'CREATED',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists wave_docs (
          tenant_id uuid not null references tenants(tenant_id),
          wave_id uuid not null references waves(wave_id) on delete cascade,
          outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade
        )
        "#,
        r#"
        create table if not exists wave_rules (
          rule_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          carrier_code text,
          max_orders int,
          max_qty bigint,
          cutoff_time time,
          strategy text,
          enabled boolean not null default true,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn try_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
        .ok()
}

fn select_wave_candidates(
    candidates: Vec<erp_repo::wms::WaveCandidate>,
    max_docs: i64,
    max_qty: Option<i64>,
) -> Vec<uuid::Uuid> {
    let max_docs = max_docs.max(1) as usize;
    let mut selected = Vec::new();
    let mut total_qty = 0_i64;
    for candidate in candidates {
        if selected.len() >= max_docs {
            break;
        }
        if let Some(limit) = max_qty {
            if !selected.is_empty() && total_qty + candidate.total_qty > limit {
                break;
            }
            if selected.is_empty() && candidate.total_qty > limit {
                selected.push(candidate.outbound_doc_id);
                break;
            }
            total_qty += candidate.total_qty;
        }
        selected.push(candidate.outbound_doc_id);
    }
    selected
}

#[tokio::test]
async fn wave_rule_respects_max_qty() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping wave_rule_respects_max_qty: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'tenant')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into warehouses (warehouse_id, tenant_id, name) values ($1, $2, 'wh')")
        .bind(warehouse_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

    let repo = WmsRepo::new(&pool);

    let first = repo
        .create_outbound_doc(OutboundDocCreateInput {
            tenant_id,
            warehouse_id,
            doc_type: "SALES".to_string(),
            shipment_id: None,
            doc_no: "OUT-WAVE-1".to_string(),
            items: vec![OutboundDocItemInput {
                sku_id,
                planned_qty: 2,
            }],
        })
        .await?;
    let second = repo
        .create_outbound_doc(OutboundDocCreateInput {
            tenant_id,
            warehouse_id,
            doc_type: "SALES".to_string(),
            shipment_id: None,
            doc_no: "OUT-WAVE-2".to_string(),
            items: vec![OutboundDocItemInput {
                sku_id,
                planned_qty: 2,
            }],
        })
        .await?;

    sqlx::query(
        "update outbound_docs set created_at = now() - interval '2 hours' where outbound_doc_id = $1",
    )
    .bind(first.doc.outbound_doc_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "update outbound_docs set created_at = now() - interval '1 hours' where outbound_doc_id = $1",
    )
    .bind(second.doc.outbound_doc_id)
    .execute(&pool)
    .await?;

    let rule = repo
        .create_wave_rule(WaveRuleCreateInput {
            tenant_id,
            warehouse_id,
            carrier_code: None,
            max_orders: Some(2),
            max_qty: Some(3),
            cutoff_time: Some(NaiveTime::from_hms_opt(23, 0, 0).unwrap()),
            strategy: None,
            enabled: true,
        })
        .await?;

    let cutoff_at = Utc::now();
    let candidates = repo
        .list_wave_candidates(
            tenant_id,
            warehouse_id,
            "DRAFT",
            cutoff_at,
            rule.carrier_code.as_deref(),
            rule.max_orders.unwrap_or(50) as i64,
        )
        .await?;
    let selected_ids = select_wave_candidates(candidates, 2, rule.max_qty);
    let detail = repo
        .create_wave(WaveCreateInput {
            tenant_id,
            warehouse_id,
            cutoff_at,
            max_docs: selected_ids.len() as i64,
            auto_pick: false,
            outbound_doc_ids: Some(selected_ids),
        })
        .await?;

    assert_eq!(detail.docs.len(), 1);
    assert_eq!(detail.docs[0].outbound_doc_id, first.doc.outbound_doc_id);
    assert_eq!(detail.tasks.len(), 1);

    Ok(())
}
