use super::*;
use anyhow::Result;
use chrono::{Datelike, Duration as ChronoDuration, Utc};
use erp_domain::OrderItemProjection;
use erp_types::{AfterSaleState, OrderId, OrderStatus, Platform, RefundState};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing_subscriber::fmt::MakeWriter;

#[test]
fn event_router_module_exports_route_event_symbol() {
    let _ = super::event_router::route_event;
}

#[derive(Clone, Default)]
struct LogCapture {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl LogCapture {
    fn contents(&self) -> String {
        let data = self.buffer.lock().expect("log buffer");
        String::from_utf8_lossy(&data).to_string()
    }
}

struct LogWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut data = self.buffer.lock().expect("log buffer");
        data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogCapture {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter {
            buffer: self.buffer.clone(),
        }
    }
}

fn test_order_with_refund_case(
    after_sale_id: Option<&str>,
    refund_state: RefundState,
    after_sale_state: AfterSaleState,
    refund_amount_cent: Option<i64>,
) -> OrderProjection {
    OrderProjection {
        tenant_id: "00000000-0000-0000-0000-000000000000".parse().unwrap(),
        shop_id: "00000000-0000-0000-0000-000000000000".parse().unwrap(),
        platform: Platform::Jd,
        platform_order_id: "P-ORDER-1".to_string(),
        status: OrderStatus::Shipped,
        total_amount_cent: 100,
        goods_amount_cent: None,
        postage_amount_cent: None,
        discount_amount_cent: None,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency: "CNY".to_string(),
        buyer_name: None,
        receiver_name: None,
        receiver_phone: None,
        receiver_address: None,
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: None,
        seller_remark: None,
        logistics_company: None,
        tracking_no: None,
        order_created_at: Utc::now(),
        platform_updated_at: Some(Utc::now()),
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "L1".to_string(),
            sku_id: Some("SKU-1".to_string()),
            title: Some("测试商品".to_string()),
            qty: 1,
            price_cent: 100,
            amount_cent: 100,
            pay_amount_cent: Some(100),
            discount_cent: None,
            refund_state,
            after_sale_state,
            refund_amount_cent,
            after_sale_type: Some("refund".to_string()),
            after_sale_id: after_sale_id.map(|v| v.to_string()),
            raw: serde_json::json!({}),
        }],
    }
}

#[test]
fn refund_sync_dedup_fingerprint_distinguishes_after_sale_case() {
    let order_a = test_order_with_refund_case(
        Some("CASE-A"),
        RefundState::Processing,
        AfterSaleState::Processing,
        Some(1200),
    );
    let order_b = test_order_with_refund_case(
        Some("CASE-B"),
        RefundState::Processing,
        AfterSaleState::Processing,
        Some(1200),
    );
    let fp_a = refund_sync_dedup_fingerprint(&order_a).expect("fingerprint A");
    let fp_b = refund_sync_dedup_fingerprint(&order_b).expect("fingerprint B");
    assert_ne!(fp_a, fp_b);
}

#[test]
fn compute_sync_dedup_key_includes_refund_fingerprint() {
    let k1 = compute_sync_dedup_key(&Platform::Jd, "P-ORDER-1", "REFUND_SYNC", "1", Some("A"));
    let k2 = compute_sync_dedup_key(&Platform::Jd, "P-ORDER-1", "REFUND_SYNC", "1", Some("B"));
    let k3 = compute_sync_dedup_key(&Platform::Jd, "P-ORDER-1", "REFUND_SYNC", "1", None);
    assert_ne!(k1, k2);
    assert_ne!(k1, k3);
}

async fn setup_schema(pool: &PgPool) -> Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists tenants (
          tenant_id uuid primary key,
          name text not null,
          status text not null default 'ACTIVE',
          pdd_only_mode boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shops (
          shop_id uuid primary key,
          tenant_id uuid not null,
          platform text not null,
          platform_shop_id text not null,
          name text not null default '',
          shop_name text not null default '',
          status text not null default 'AUTHORIZED',
          default_carrier_code text,
          credentials jsonb not null default '{}'::jsonb,
          auth_meta_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, platform_shop_id)
        )
        "#,
        r#"
        create table if not exists order_key_index (
          tenant_id uuid not null,
          platform text not null,
          platform_order_id text not null,
          order_id uuid not null,
          order_month date not null,
          shop_id uuid not null,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, platform, platform_order_id)
        )
        "#,
        r#"
        create table if not exists order_index (
          tenant_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          platform text not null,
          platform_order_id text not null,
          shop_id uuid not null,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists orders (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          platform text not null,
          shop_id uuid not null,
            platform_order_id text not null,
            status text not null,
            oms_status text not null default 'NEW',
            order_status text not null default 'PENDING_REVIEW',
            fulfillment_status text not null default 'NOT_ALLOCATED',
            exception_status text not null default 'NONE',
            review_status text not null default 'NOT_REVIEWED',
            exception_flags jsonb not null default '[]'::jsonb,
            version int not null default 0,
          total_amount_cent bigint not null,
          goods_amount_cent bigint,
          postage_amount_cent bigint,
          discount_amount_cent bigint,
          platform_discount_cent bigint,
          seller_discount_cent bigint,
          coupon_discount_cent bigint,
          currency text not null,
          buyer_name text,
          receiver_name text,
          receiver_phone text,
          receiver_address text,
          pay_at timestamptz,
          payment_status text,
          ship_at timestamptz,
          finish_at timestamptz,
          cancel_at timestamptz,
          cancel_reason text,
          buyer_message text,
          seller_remark text,
          logistics_company text,
          tracking_no text,
          order_created_at timestamptz not null,
          platform_updated_at timestamptz,
          raw jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id),
          unique (tenant_id, order_month, platform, platform_order_id)
        )
        "#,
        r#"
        create table if not exists order_items (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          line_id text not null,
          sku_id text,
          title text,
          qty int not null,
          price_cent bigint not null,
          amount_cent bigint not null,
          pay_amount_cent bigint,
          discount_cent bigint,
          refund_state text not null,
          after_sale_state text not null,
          refund_amount_cent bigint,
          after_sale_type text,
          after_sale_id text,
          raw jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id, line_id)
        )
        "#,
        r#"
        create table if not exists order_review_rules (
          rule_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          name text not null,
          enabled boolean not null default true,
          priority int not null default 0,
          conditions_json jsonb not null default '[]'::jsonb,
          action text not null,
          reason text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, name)
        )
        "#,
        r#"
        create table if not exists order_review_hits (
          tenant_id uuid not null,
          order_id uuid not null,
          rule_id uuid not null,
          hit boolean not null default false,
          message text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id, rule_id)
        )
        "#,
        r#"
        create table if not exists order_reviews (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          status text not null default 'PENDING',
          reviewed_by uuid,
          reviewed_at timestamptz,
          notes text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists order_tags (
          tenant_id uuid not null,
          order_id uuid not null,
          tag text not null,
          created_by uuid,
          created_at timestamptz not null default now(),
          primary key (tenant_id, order_id, tag)
        )
        "#,
        r#"
        create table if not exists order_notes (
          note_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          order_id uuid not null,
          note text not null,
          created_by uuid,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists customer_blacklist (
          blacklist_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          key_type text not null,
          key_value text not null,
          reason text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, key_type, key_value)
        )
        "#,
        r#"
        create table if not exists after_sale_cases (
          tenant_id uuid not null,
          case_id uuid not null default gen_random_uuid(),
          platform text not null,
          shop_id uuid not null,
          after_sale_id text not null,
          after_sale_type text,
          refund_state text not null default 'NONE',
          after_sale_state text not null default 'NONE',
          refund_amount_cent bigint not null default 0,
          item_count int not null default 0,
          total_qty int not null default 0,
          order_count int not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, case_id),
          unique (tenant_id, platform, shop_id, after_sale_id)
        )
        "#,
        r#"
        create table if not exists after_sale_case_items (
          tenant_id uuid not null,
          case_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          line_id text not null,
          sku_id text,
          title text,
          qty int not null,
          refund_state text not null default 'NONE',
          after_sale_state text not null default 'NONE',
          refund_amount_cent bigint,
          after_sale_type text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, case_id, order_id, line_id)
        )
        "#,
        r#"
        create table if not exists outbox_events (
          tenant_id uuid not null,
          event_month date not null,
          outbox_id uuid not null default gen_random_uuid(),
          event_type text not null,
          payload jsonb not null,
          trace_id text,
          dedup_key text,
          status text not null default 'PENDING',
          attempts int not null default 0,
          next_retry_at timestamptz not null default now(),
          last_error text not null default '',
          last_attempt_at timestamptz not null default now(),
          published_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, event_month, outbox_id)
        )
        "#,
        r#"
        create table if not exists worker_effects (
          tenant_id uuid not null,
          outbox_id uuid not null,
          event_type text not null,
          status text not null default 'PROCESSING',
          last_error text not null default '',
          processed_at timestamptz not null default now(),
          primary key (tenant_id, outbox_id)
        )
        "#,
        r#"
        create table if not exists feature_flags (
          tenant_id uuid not null,
          scope text not null,
          shop_id uuid,
          key text not null,
          enabled boolean not null default true,
          updated_at timestamptz not null default now(),
          unique (tenant_id, scope, shop_id, key)
        )
        "#,
        r#"
        create table if not exists dead_letters (
          dead_letter_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          outbox_id uuid not null,
          event_type text not null,
          payload jsonb not null,
          error text not null,
          attempts int not null,
          classification text not null,
          next_action text not null,
          status text not null default 'PENDING',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          replayed_at timestamptz
        )
        "#,
        r#"
        create unique index if not exists idx_dead_letters_outbox
          on dead_letters(tenant_id, outbox_id)
        "#,
        r#"
        create table if not exists channel_sku_mappings (
          mapping_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shop_id uuid not null,
          platform text not null,
          platform_sku_id text not null,
          platform_outer_sku_id text,
          sku_id uuid not null,
          status text not null default 'ACTIVE',
          confidence int not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, platform, platform_sku_id)
        )
        "#,
        r#"
        create table if not exists platform_tokens (
          tenant_id uuid not null,
          shop_id uuid not null,
          token_status text not null default 'ACTIVE',
          access_token text,
          refresh_token text,
          expires_at timestamptz,
          scopes text,
          last_refreshed_at timestamptz,
          refresh_fail_count int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, shop_id)
        )
        "#,
        r#"
        create table if not exists sync_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shop_id uuid not null,
          job_type text not null,
          cursor text,
          watermark timestamptz,
          status text not null default 'PENDING',
          last_error text not null default '',
          last_success_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, job_type)
        )
        "#,
        r#"
        create table if not exists inbox_raw_payloads (
          raw_id bigserial primary key,
          tenant_id uuid not null,
          platform text not null,
          shop_id uuid not null,
          headers jsonb not null default '{}'::jsonb,
          body bytea not null,
          received_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists inbox_dedup (
          tenant_id uuid not null,
          dedup_key text not null,
          first_seen_at timestamptz not null default now(),
          last_seen_at timestamptz not null default now(),
          seen_count int not null default 1,
          primary key (tenant_id, dedup_key)
        )
        "#,
        r#"
        create table if not exists inbox_events (
          tenant_id uuid not null,
          event_month date not null,
          inbox_event_id uuid not null default gen_random_uuid(),
          platform text not null,
          shop_id uuid not null,
          dedup_key text not null,
          raw_id bigint not null,
          status text not null default 'RECEIVED',
          error text not null default '',
          verify_status text not null default 'PASSED',
          failed_reason text not null default '',
          received_at timestamptz not null default now(),
          processed_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, event_month, inbox_event_id),
          unique (tenant_id, event_month, dedup_key)
        )
        "#,
        r#"
        create table if not exists mapping_rules (
          rule_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          platform text not null,
          entity_type text not null,
          version int not null,
          enabled boolean not null default true,
          rule_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, entity_type, version)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn setup_shipping_schema(pool: &PgPool) -> Result<()> {
    setup_schema(pool).await?;
    let statements = [
        r#"
        create table if not exists warehouses (
          warehouse_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          code text not null,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, code)
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key,
          tenant_id uuid not null,
          sku_kind text not null default 'NORMAL'
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null,
          shop_id uuid not null,
          order_id uuid not null,
          platform text not null,
          platform_order_id text not null,
            warehouse_id uuid,
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
        create table if not exists shipment_packages (
          package_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shipment_id uuid not null,
          package_no text not null,
          carrier_code text,
          tracking_no text,
          weight_gram int,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shipment_id, package_no)
        )
        "#,
        r#"
        create table if not exists outbound_docs (
          outbound_doc_id uuid primary key,
          tenant_id uuid not null,
          shipment_id uuid,
          status text not null default 'DRAFT',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists inventory_balances (
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          on_hand bigint not null default 0,
          reserved bigint not null default 0,
          in_transit bigint not null default 0,
          frozen bigint not null default 0,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, warehouse_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inventory_reservations (
          reservation_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          order_id uuid,
          order_item_id uuid,
          qty bigint not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_item_id)
        )
        "#,
        r#"
        create table if not exists inventory_ledgers (
          ledger_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          delta_on_hand bigint not null,
          delta_reserved bigint not null,
          delta_in_transit bigint not null,
          delta_frozen bigint not null,
          reason_type text not null,
          ref_type text,
          ref_id text,
          occurred_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipment_push_attempts (
          attempt_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          outbox_id uuid not null,
          shipment_id uuid not null,
          shop_id uuid not null,
          platform text not null,
          platform_order_id text not null,
          carrier_code text,
          tracking_no text,
          shipped_at timestamptz,
          status text not null,
          attempt int not null,
          duration_ms int not null,
          error_code text not null default '',
          error_message text not null default '',
          request_json jsonb not null default '{}'::jsonb,
          response_json jsonb not null default '{}'::jsonb,
          next_retry_at timestamptz,
          dead_letter_id uuid,
          trigger text not null default 'AUTO',
          created_at timestamptz not null default now()
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn setup_printing_schema(pool: &PgPool) -> Result<()> {
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
        create table if not exists print_templates (
          template_id uuid primary key,
          tenant_id uuid not null,
          shop_id uuid not null,
          platform text not null,
          template_type text not null,
          platform_template_id text,
          version int not null default 1,
          content_format text not null default 'HTML',
          content jsonb not null default '{}'::jsonb,
          is_active boolean not null default true,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists print_jobs (
          job_id uuid primary key,
          tenant_id uuid not null,
          type text,
          ref_type text,
          ref_id uuid,
          template_id uuid not null,
          template_version int not null,
          requested_by_user_id uuid not null,
          status text not null default 'PENDING',
          dispatch_status text not null default 'QUEUED',
          payload jsonb not null default '{}'::jsonb,
          output_url text,
          output_content text,
          attempts int not null default 0,
          last_error text not null default '',
          device_fingerprint text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists print_job_events (
          event_id uuid primary key,
          tenant_id uuid not null,
          job_id uuid not null,
          status text not null,
          actor_type text not null,
          actor_user_id uuid,
          error text,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists audit_logs (
          audit_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          user_id uuid,
          action text not null,
          entity_type text not null,
          entity_id text,
          before_json jsonb,
          after_json jsonb,
          ip text,
          ua text,
          created_at timestamptz not null default now()
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

#[tokio::test]
async fn print_job_renders_text_output() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping print_job_renders_text_output: db unavailable");
        return Ok(());
    };
    setup_printing_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'print-tenant', 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    let template_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into print_templates (
          template_id, tenant_id, shop_id, platform, template_type, version,
          content_format, content, is_active
        )
        values ($1, $2, $3, 'taobao', 'SHIPPING_LABEL', 1, 'TEXT', $4, true)
        "#,
    )
    .bind(template_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shop_id)
    .bind(serde_json::json!("LABEL {{payload}}"))
    .execute(&pool)
    .await?;

    let job_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into print_jobs (
          job_id, tenant_id, type, ref_type, ref_id, template_id, template_version,
          requested_by_user_id, status, payload, attempts, last_error
        )
        values ($1, $2, 'SHIPPING_LABEL', 'SHIPMENT', $3, $4, 1, $5, 'PENDING', $6, 0, '')
        "#,
    )
    .bind(job_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .bind(template_id)
    .bind(uuid::Uuid::new_v4())
    .bind(serde_json::json!({"tracking_no": "TEST-100"}))
    .execute(&pool)
    .await?;

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    let env = OutboxEnvelope {
        tenant_id,
        outbox_id: uuid::Uuid::new_v4(),
        event_type: "PRINT_JOB_QUEUED".to_string(),
        payload: serde_json::json!({"job_id": job_id}),
        produced_at: Utc::now(),
        trace_id: None,
    };

    handle_print_job_queued(&db, env).await?;

    #[derive(sqlx::FromRow)]
    struct JobRow {
        status: String,
        output_url: Option<String>,
        output_content: Option<String>,
        attempts: i32,
    }

    let row: JobRow = sqlx::query_as(
        r#"
        select status, output_url, output_content, attempts
        from print_jobs
        where tenant_id = $1 and job_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(job_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.status, "DONE");
    assert_eq!(row.attempts, 1);
    let content = row.output_content.unwrap_or_default();
    assert!(content.contains("tracking_no"));
    assert!(!content.to_lowercase().contains("<html"));
    assert!(row.output_url.is_some());

    Ok(())
}

#[tokio::test]
async fn mapping_missing_emits_outbox() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping mapping_missing_emits_outbox: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("TEST-SHOP")
    .bind("test-shop")
    .execute(&pool)
    .await?;

    let mut order = OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-1".to_string(),
        status: OrderStatus::PendingPayment,
        total_amount_cent: 100,
        goods_amount_cent: None,
        postage_amount_cent: None,
        discount_amount_cent: None,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency: "CNY".to_string(),
        buyer_name: None,
        receiver_name: None,
        receiver_phone: None,
        receiver_address: None,
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: None,
        seller_remark: None,
        logistics_company: None,
        tracking_no: None,
        order_created_at: Utc::now(),
        platform_updated_at: None,
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "LINE-1".to_string(),
            sku_id: Some("UNKNOWN-SKU".to_string()),
            title: Some("Test Item".to_string()),
            qty: 1,
            price_cent: 100,
            amount_cent: 100,
            pay_amount_cent: None,
            discount_cent: None,
            refund_state: RefundState::None,
            after_sale_state: AfterSaleState::None,
            refund_amount_cent: None,
            after_sale_type: None,
            after_sale_id: None,
            raw: serde_json::json!({}),
        }],
    };

    let outbox_repo = OutboxRepo::new(&pool);
    apply_sku_mappings(&pool, &outbox_repo, &mut order).await?;

    let orders_repo = OrdersRepo::new(&pool);
    let order_id = orders_repo.upsert_order_projection(&order).await?;

    let stored_sku: Option<String> = sqlx::query_scalar(
        r#"
        select sku_id
        from order_items
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_one(&pool)
    .await?;
    assert!(stored_sku.is_none());

    #[derive(sqlx::FromRow)]
    struct OutboxRow {
        event_type: String,
        payload: serde_json::Value,
    }

    let row: OutboxRow = sqlx::query_as(
        r#"
        select event_type, payload
        from outbox_events
        where event_type = 'MAPPING_MISSING'
        "#,
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.event_type, "MAPPING_MISSING");
    let tenant_id_str = tenant_id.to_string();
    let shop_id_str = shop_id.to_string();

    assert_eq!(
        row.payload.get("tenant_id").and_then(|v| v.as_str()),
        Some(tenant_id_str.as_str())
    );
    assert_eq!(
        row.payload.get("shop_id").and_then(|v| v.as_str()),
        Some(shop_id_str.as_str())
    );
    assert_eq!(
        row.payload.get("platform").and_then(|v| v.as_str()),
        Some("taobao")
    );
    assert_eq!(
        row.payload.get("platform_sku_id").and_then(|v| v.as_str()),
        Some("UNKNOWN-SKU")
    );
    assert_eq!(
        row.payload
            .get("platform_order_id")
            .and_then(|v| v.as_str()),
        Some("ORDER-1")
    );
    Ok(())
}

#[tokio::test]
async fn order_upsert_event_idempotent_for_cases() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping order_upsert_event_idempotent_for_cases: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("SHOP-CASE")
    .bind("shop")
    .execute(&pool)
    .await?;

    let order = OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-CASE-1".to_string(),
        status: OrderStatus::Completed,
        total_amount_cent: 100,
        goods_amount_cent: None,
        postage_amount_cent: None,
        discount_amount_cent: None,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency: "CNY".to_string(),
        buyer_name: None,
        receiver_name: None,
        receiver_phone: None,
        receiver_address: None,
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: None,
        seller_remark: None,
        logistics_company: None,
        tracking_no: None,
        order_created_at: Utc::now(),
        platform_updated_at: None,
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "LINE-1".to_string(),
            sku_id: Some("SKU-1".to_string()),
            title: Some("Return Item".to_string()),
            qty: 1,
            price_cent: 100,
            amount_cent: 100,
            pay_amount_cent: None,
            discount_cent: None,
            refund_state: RefundState::Processing,
            after_sale_state: AfterSaleState::Processing,
            refund_amount_cent: None,
            after_sale_type: Some("refund".to_string()),
            after_sale_id: Some("CASE-1".to_string()),
            raw: serde_json::json!({}),
        }],
    };

    let orders_repo = OrdersRepo::new(&pool);
    let order_id = orders_repo.upsert_order_projection(&order).await?;
    let payload = OrderUpsertedPayload {
        tenant_id,
        platform: Platform::Taobao,
        shop_id,
        platform_order_id: "ORDER-CASE-1".to_string(),
        order_id,
    };

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;

    handle_order_upserted(&db, payload.clone()).await?;
    handle_order_upserted(&db, payload).await?;

    let case_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from after_sale_cases
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(case_count, 1);

    let item_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from after_sale_case_items
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(item_count, 1);

    Ok(())
}

#[tokio::test]
async fn order_review_rule_hit_on_import() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping order_review_rule_hit_on_import: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    let now = Utc::now();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("review-tenant")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("SHOP-REVIEW")
    .bind("review-shop")
    .execute(&pool)
    .await?;

    let rule_id = uuid::Uuid::new_v4();
    let conditions_json = serde_json::json!([
        {
            "field_path": "total_cent",
            "op": ">",
            "value": 500
        }
    ]);
    sqlx::query(
        r#"
        insert into order_review_rules (
            rule_id, tenant_id, name, enabled, priority, conditions_json, action, reason
        ) values ($1, $2, $3, true, 10, $4, 'FLAG', 'HIGH_VALUE')
        "#,
    )
    .bind(rule_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind("high-value")
    .bind(conditions_json)
    .execute(&pool)
    .await?;

    let order = OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-REVIEW-1".to_string(),
        status: OrderStatus::PendingShipment,
        total_amount_cent: 1000,
        goods_amount_cent: None,
        postage_amount_cent: None,
        discount_amount_cent: None,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency: "CNY".to_string(),
        buyer_name: None,
        receiver_name: Some("Alice".to_string()),
        receiver_phone: None,
        receiver_address: Some("Beijing Road 1".to_string()),
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: None,
        seller_remark: None,
        logistics_company: None,
        tracking_no: None,
        order_created_at: now,
        platform_updated_at: Some(now),
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "LINE-1".to_string(),
            sku_id: Some("SKU-1".to_string()),
            title: Some("Rule Item".to_string()),
            qty: 1,
            price_cent: 1000,
            amount_cent: 1000,
            pay_amount_cent: None,
            discount_cent: None,
            refund_state: RefundState::None,
            after_sale_state: AfterSaleState::None,
            refund_amount_cent: None,
            after_sale_type: None,
            after_sale_id: None,
            raw: serde_json::json!({}),
        }],
    };

    let orders_repo = OrdersRepo::new(&pool);
    let order_id = orders_repo.upsert_order_projection(&order).await?;
    let payload = OrderUpsertedPayload {
        tenant_id,
        platform: Platform::Taobao,
        shop_id,
        platform_order_id: "ORDER-REVIEW-1".to_string(),
        order_id,
    };

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    handle_order_upserted(&db, payload).await?;

    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let row: (String, serde_json::Value) = sqlx::query_as(
        r#"
        select review_status, exception_flags
        from orders
        where tenant_id = $1 and order_id = $2 and order_month = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(order_month)
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.0, "PENDING");
    let flags = match row.1.as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>(),
        None => vec![],
    };
    assert!(flags.contains(&"HIGH_VALUE"));
    assert!(flags.contains(&"REVIEW_FLAGGED"));

    let review_status_row: (String,) = sqlx::query_as(
        r#"
        select status
        from order_reviews
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(review_status_row.0, "PENDING");

    let hit_row: (bool, String) = sqlx::query_as(
        r#"
        select hit, message
        from order_review_hits
        where tenant_id = $1 and order_id = $2 and rule_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(rule_id)
    .fetch_one(&pool)
    .await?;
    assert!(hit_row.0);
    assert_eq!(hit_row.1, "HIGH_VALUE");

    Ok(())
}

#[tokio::test]
async fn order_upsert_ignores_out_of_order_update() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping order_upsert_ignores_out_of_order_update: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    let now = Utc::now();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("order-tenant")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("SHOP-ORDER")
    .bind("order-shop")
    .execute(&pool)
    .await?;

    let mut order = OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-OUT-OF-ORDER".to_string(),
        status: OrderStatus::Shipped,
        total_amount_cent: 2000,
        goods_amount_cent: None,
        postage_amount_cent: None,
        discount_amount_cent: None,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency: "CNY".to_string(),
        buyer_name: None,
        receiver_name: None,
        receiver_phone: None,
        receiver_address: Some("Shanghai Road 9".to_string()),
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: None,
        seller_remark: None,
        logistics_company: None,
        tracking_no: None,
        order_created_at: now,
        platform_updated_at: Some(now),
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "LINE-1".to_string(),
            sku_id: Some("SKU-1".to_string()),
            title: Some("Out of order".to_string()),
            qty: 1,
            price_cent: 2000,
            amount_cent: 2000,
            pay_amount_cent: None,
            discount_cent: None,
            refund_state: RefundState::None,
            after_sale_state: AfterSaleState::None,
            refund_amount_cent: None,
            after_sale_type: None,
            after_sale_id: None,
            raw: serde_json::json!({}),
        }],
    };

    let orders_repo = OrdersRepo::new(&pool);
    let order_id = orders_repo.upsert_order_projection(&order).await?;

    order.status = OrderStatus::PendingShipment;
    order.platform_updated_at = Some(now - ChronoDuration::hours(1));
    orders_repo.upsert_order_projection(&order).await?;

    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let status: String = sqlx::query_scalar(
        r#"
        select status
        from orders
        where tenant_id = $1 and order_id = $2 and order_month = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(order_month)
    .fetch_one(&pool)
    .await?;

    assert_eq!(status, "SHIPPED");

    Ok(())
}

#[tokio::test]
async fn shipment_retry_consumes_once() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping shipment_retry_consumes_once: db unavailable");
        return Ok(());
    };
    setup_shipping_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let order_item_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("SHOP-1")
    .bind("shop")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind("Main")
    .bind("WH-1")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, sku_kind)
        values ($1, $2, 'NORMAL')
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,'PENDING_SHIPMENT',$7)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(order_id)
    .bind(Platform::Taobao.to_string())
    .bind(uuid::Uuid::from(shop_id))
    .bind("ORDER-1")
    .bind(now)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
          tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .bind(order_month)
    .bind(Platform::Taobao.to_string())
    .bind("ORDER-1")
    .bind(uuid::Uuid::from(shop_id))
    .bind(now)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty,
          price_cent, amount_cent, refund_state, after_sale_state, raw
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,'NONE','NONE','{}'::jsonb)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(order_id)
    .bind(order_item_id.to_string())
    .bind(sku_id.to_string())
    .bind(2_i32)
    .bind(100_i64)
    .bind(200_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand, reserved)
        values ($1,$2,$3,$4,$5)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(10_i64)
    .bind(2_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_reservations (
          tenant_id, warehouse_id, sku_id, order_id, order_item_id, qty, status
        ) values ($1,$2,$3,$4,$5,$6,'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(order_id)
    .bind(order_item_id)
    .bind(2_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
          shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
          warehouse_id, status, requested_at
        ) values ($1,$2,$3,$4,$5,$6,$7,'REQUESTED',$8)
        "#,
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(order_id)
    .bind(Platform::Taobao.to_string())
    .bind("ORDER-1")
    .bind(warehouse_id)
    .bind(now)
    .execute(&pool)
    .await?;

    let payload = ShipmentRequestedPayload {
        tenant_id,
        shipment_id,
        order_id: erp_types::OrderId(order_id),
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-1".to_string(),
        warehouse_id,
        carrier_code: Some("ZTO".to_string()),
        tracking_no: Some("TRACK-1".to_string()),
        weight_gram: None,
    };

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    let adapters = AdapterRegistry::new();

    handle_shipment_requested(&db, &adapters, payload.clone()).await?;
    handle_shipment_requested(&db, &adapters, payload).await?;

    #[derive(sqlx::FromRow)]
    struct BalanceRow {
        on_hand: i64,
        reserved: i64,
    }

    let balance: BalanceRow = sqlx::query_as(
        r#"
        select on_hand, reserved
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(balance.on_hand, 8);
    assert_eq!(balance.reserved, 0);

    let ship_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and sku_id = $2 and kind = 'SHIP'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(ship_count, 1);

    Ok(())
}

#[tokio::test]
async fn shipment_skipped_when_feature_disabled() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping shipment_skipped_when_feature_disabled: db unavailable");
        return Ok(());
    };
    setup_shipping_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let order_item_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("SHOP-FF")
    .bind("shop")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind("Main")
    .bind("WH-FF")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, sku_kind)
        values ($1, $2, 'NORMAL')
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,'PENDING_SHIPMENT',$7)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(order_id)
    .bind(Platform::Taobao.to_string())
    .bind(uuid::Uuid::from(shop_id))
    .bind("ORDER-FF")
    .bind(now)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
          tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .bind(order_month)
    .bind(Platform::Taobao.to_string())
    .bind("ORDER-FF")
    .bind(uuid::Uuid::from(shop_id))
    .bind(now)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty,
          price_cent, amount_cent, refund_state, after_sale_state, raw
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,'NONE','NONE','{}'::jsonb)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(order_id)
    .bind(order_item_id.to_string())
    .bind(sku_id.to_string())
    .bind(2_i32)
    .bind(100_i64)
    .bind(200_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand, reserved)
        values ($1,$2,$3,$4,$5)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(10_i64)
    .bind(2_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_reservations (
          tenant_id, warehouse_id, sku_id, order_id, order_item_id, qty, status
        ) values ($1,$2,$3,$4,$5,$6,'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(order_id)
    .bind(order_item_id)
    .bind(2_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
          shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
          warehouse_id, status, requested_at
        ) values ($1,$2,$3,$4,$5,$6,$7,'REQUESTED',$8)
        "#,
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(order_id)
    .bind(Platform::Taobao.to_string())
    .bind("ORDER-FF")
    .bind(warehouse_id)
    .bind(now)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into feature_flags (tenant_id, scope, shop_id, key, enabled)
        values ($1, 'SHOP', $2, 'SHIPPING_WRITE', false)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .execute(&pool)
    .await?;

    let payload = ShipmentRequestedPayload {
        tenant_id,
        shipment_id,
        order_id: erp_types::OrderId(order_id),
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-FF".to_string(),
        warehouse_id,
        carrier_code: Some("ZTO".to_string()),
        tracking_no: Some("TRACK-FF".to_string()),
        weight_gram: None,
    };

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    let adapters = AdapterRegistry::new();

    let logs = LogCapture::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    handle_shipment_requested(&db, &adapters, payload).await?;

    let status: String = sqlx::query_scalar(
        r#"
        select status
        from shipments
        where tenant_id = $1 and shipment_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "REQUESTED");

    #[derive(sqlx::FromRow)]
    struct BalanceRow {
        on_hand: i64,
        reserved: i64,
    }

    let balance: BalanceRow = sqlx::query_as(
        r#"
        select on_hand, reserved
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(balance.on_hand, 10);
    assert_eq!(balance.reserved, 2);

    let ship_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and sku_id = $2 and kind = 'SHIP'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(ship_count, 0);

    assert!(logs.contents().contains("feature disabled: shipping write"));

    Ok(())
}

#[tokio::test]
async fn order_upsert_skipped_when_feature_disabled() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping order_upsert_skipped_when_feature_disabled: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("SHOP-FF-CASE")
    .bind("shop")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into feature_flags (tenant_id, scope, shop_id, key, enabled)
        values ($1, 'SHOP', $2, 'AFTERSALES_WRITE', false)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .execute(&pool)
    .await?;

    let mut order = OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-FF-CASE".to_string(),
        status: OrderStatus::Completed,
        total_amount_cent: 100,
        goods_amount_cent: None,
        postage_amount_cent: None,
        discount_amount_cent: None,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency: "CNY".to_string(),
        buyer_name: None,
        receiver_name: None,
        receiver_phone: None,
        receiver_address: None,
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: None,
        seller_remark: None,
        logistics_company: None,
        tracking_no: None,
        order_created_at: Utc::now(),
        platform_updated_at: None,
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "LINE-FF".to_string(),
            sku_id: Some("SKU-FF".to_string()),
            title: Some("Test Item".to_string()),
            qty: 1,
            price_cent: 100,
            amount_cent: 100,
            pay_amount_cent: None,
            discount_cent: None,
            refund_state: RefundState::None,
            after_sale_state: AfterSaleState::None,
            refund_amount_cent: None,
            after_sale_type: None,
            after_sale_id: None,
            raw: serde_json::json!({}),
        }],
    };

    let outbox_repo = OutboxRepo::new(&pool);
    apply_sku_mappings(&pool, &outbox_repo, &mut order).await?;

    let orders_repo = OrdersRepo::new(&pool);
    let order_id = orders_repo.upsert_order_projection(&order).await?;

    let payload = OrderUpsertedPayload {
        tenant_id,
        platform: Platform::Taobao,
        shop_id,
        platform_order_id: "ORDER-FF-CASE".to_string(),
        order_id,
    };

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;

    let logs = LogCapture::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    handle_order_upserted(&db, payload).await?;

    let case_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from after_sale_cases
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(case_count, 0);

    assert!(logs
        .contents()
        .contains("feature disabled: aftersales write"));

    Ok(())
}

async fn apply_effect(pool: &PgPool, env: &OutboxEnvelope) -> Result<()> {
    sqlx::query(
        r#"
        insert into worker_effects (tenant_id, outbox_id, event_type)
        values ($1, $2, $3)
        on conflict (tenant_id, outbox_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(env.tenant_id))
    .bind(env.outbox_id)
    .bind(&env.event_type)
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn transient_failure_moves_to_dlq_after_max_attempts() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping transient_failure_moves_to_dlq_after_max_attempts: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    let env = OutboxEnvelope {
        tenant_id,
        outbox_id: uuid::Uuid::new_v4(),
        event_type: "DLQ_TRANSIENT".to_string(),
        payload: serde_json::json!({"value": 1}),
        produced_at: Utc::now(),
        trace_id: None,
    };

    let err = anyhow::anyhow!(erp_adapter::AdapterError::Temporary("timeout".to_string()));
    let policy = dlq::RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 1,
        max_delay_ms: 5,
    };

    for attempt in 1..=3 {
        let outcome = dlq::handle_processing_failure(&pool, &env, &err, attempt, &policy).await?;
        match outcome {
            dlq::FailureOutcome::Retry { .. } => {}
            _ => panic!("expected retry outcome"),
        }
    }

    let outcome = dlq::handle_processing_failure(&pool, &env, &err, 4, &policy).await?;
    let dead_letter_id = match outcome {
        dlq::FailureOutcome::DeadLettered { dead_letter_id, .. } => dead_letter_id,
        _ => panic!("expected dead letter"),
    };

    let row = dlq::fetch_dead_letter(&pool, uuid::Uuid::from(tenant_id), dead_letter_id)
        .await?
        .expect("dead letter row");
    assert_eq!(row.attempts, 3);
    assert_eq!(row.classification, dlq::FailureClass::Transient.as_str());
    assert_eq!(row.next_action, dlq::NEXT_ACTION_MANUAL_REPLAY);
    assert_eq!(row.payload.get("value").and_then(|v| v.as_i64()), Some(1));

    Ok(())
}

#[tokio::test]
async fn replay_dead_letter_requeues_idempotent() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping replay_dead_letter_requeues_idempotent: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    sqlx::query(
        r#"
        create table if not exists worker_effects (
          tenant_id uuid not null,
          outbox_id uuid not null,
          event_type text not null,
          status text not null default 'PROCESSING',
          last_error text not null default '',
          processed_at timestamptz not null default now(),
          primary key (tenant_id, outbox_id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    let env = OutboxEnvelope {
        tenant_id,
        outbox_id: uuid::Uuid::new_v4(),
        event_type: "DLQ_REPLAY".to_string(),
        payload: serde_json::json!({"value": "ok"}),
        produced_at: Utc::now(),
        trace_id: None,
    };

    let dead_letter_id = dlq::insert_dead_letter(
        &pool,
        &env,
        "boom",
        3,
        dlq::FailureClass::Transient,
        dlq::NEXT_ACTION_MANUAL_REPLAY,
    )
    .await?;

    let captured = Arc::new(Mutex::new(None));
    let captured_ref = captured.clone();
    let pool_ref = pool.clone();

    dlq::replay_dead_letter_with(
        &pool,
        uuid::Uuid::from(tenant_id),
        dead_letter_id,
        move |msg| {
            let captured_ref = captured_ref.clone();
            let pool_ref = pool_ref.clone();
            async move {
                *captured_ref.lock().unwrap() = Some(msg.clone());
                let env: OutboxEnvelope = serde_json::from_slice(&msg.payload)?;
                apply_effect(&pool_ref, &env).await?;
                Ok(())
            }
        },
    )
    .await?;

    let message = captured.lock().unwrap().clone().expect("replay message");
    let expected_subject =
        erp_outbox::publisher::subject_for(&tenant_id.to_string(), &env.event_type);
    assert_eq!(message.subject, expected_subject);

    let replay_env: OutboxEnvelope = serde_json::from_slice(&message.payload)?;
    apply_effect(&pool, &replay_env).await?;

    let count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from worker_effects
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 1);

    let row = dlq::fetch_dead_letter(&pool, uuid::Uuid::from(tenant_id), dead_letter_id)
        .await?
        .expect("dead letter row");
    assert_eq!(row.status, "REPLAYED");

    Ok(())
}

#[tokio::test]
async fn token_refresh_success_updates_token() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping token_refresh_success_updates_token: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'token-tenant', 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
        ) values ($1, $2, 'taobao', 'SHOP-TOKEN', 'shop', 'shop', 'AUTHORIZED', '{}'::jsonb, '{}'::jsonb)
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    let tokens_repo = PlatformTokensRepo::new(&pool);
    let token = tokens_repo
        .upsert_token(erp_repo::platform_tokens::PlatformTokenUpsert {
            tenant_id,
            shop_id,
            token_status: None,
            access_token: None,
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(Utc::now() - ChronoDuration::minutes(10)),
            scopes: Some("read".to_string()),
            last_refreshed_at: None,
            refresh_fail_count: Some(0),
            last_error: Some(String::new()),
        })
        .await?;

    let candidate = erp_repo::platform_tokens::TokenRefreshCandidate {
        token,
        platform: Platform::Taobao,
    };

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    let adapters = AdapterRegistry::new();

    token_refresh::refresh_token_for_candidate(&db, &adapters, &candidate).await?;

    let updated = tokens_repo
        .get_token(tenant_id, shop_id)
        .await?
        .expect("token");
    assert!(updated.access_token.is_some());
    assert!(updated.last_refreshed_at.is_some());
    assert!(updated.last_error.is_empty());

    let status: String =
        sqlx::query_scalar("select status from shops where tenant_id = $1 and shop_id = $2")
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(shop_id))
            .fetch_one(&pool)
            .await?;
    assert_eq!(status, "AUTHORIZED");

    Ok(())
}

#[tokio::test]
async fn token_refresh_failure_emits_outbox() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping token_refresh_failure_emits_outbox: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'token-tenant', 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
        ) values ($1, $2, 'taobao', 'SHOP-TOKEN-FAIL', 'shop', 'shop', 'AUTHORIZED', '{}'::jsonb, '{"refresh_fail": true}'::jsonb)
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    let tokens_repo = PlatformTokensRepo::new(&pool);
    let token = tokens_repo
        .upsert_token(erp_repo::platform_tokens::PlatformTokenUpsert {
            tenant_id,
            shop_id,
            token_status: None,
            access_token: None,
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(Utc::now() - ChronoDuration::minutes(10)),
            scopes: None,
            last_refreshed_at: None,
            refresh_fail_count: Some(0),
            last_error: Some(String::new()),
        })
        .await?;

    let candidate = erp_repo::platform_tokens::TokenRefreshCandidate {
        token,
        platform: Platform::Taobao,
    };

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    let adapters = AdapterRegistry::new();

    token_refresh::refresh_token_for_candidate(&db, &adapters, &candidate).await?;

    let updated = tokens_repo
        .get_token(tenant_id, shop_id)
        .await?
        .expect("token");
    assert!(!updated.last_error.is_empty());

    let status: String =
        sqlx::query_scalar("select status from shops where tenant_id = $1 and shop_id = $2")
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(shop_id))
            .fetch_one(&pool)
            .await?;
    assert_eq!(status, "EXPIRED");

    let outbox_count: i64 = sqlx::query_scalar(
        "select count(*) from outbox_events where event_type = 'TOKEN_REFRESH_FAILED'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(outbox_count, 1);

    Ok(())
}

#[tokio::test]
async fn refund_rollback_clears_packages_and_marks_refunded() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping refund_rollback_clears_packages_and_marks_refunded: db unavailable");
        return Ok(());
    };
    setup_shipping_schema(&pool).await?;
    setup_printing_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    let order_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let refund_doc_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'refund-tenant', 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, 'taobao', 'SHOP-REFUND', 'shop', 'shop', 'AUTHORIZED')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
          tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1, $2, $3, 'taobao', 'PLAT-REFUND', $4, $5)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .bind(order_month)
    .bind(uuid::Uuid::from(shop_id))
    .bind(now)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
          oms_status, order_status, fulfillment_status, exception_status, review_status,
          exception_flags, total_amount_cent, currency, order_created_at
        ) values (
          $1, $2, $3, 'taobao', $4, 'PLAT-REFUND', 'PENDING_SHIPMENT',
          'PENDING_SHIPMENT', 'PENDING_ALLOCATE', 'READY_TO_SHIP', 'NONE', 'APPROVED',
          '[]'::jsonb, 1000, 'CNY', $5
        )
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(order_id)
    .bind(uuid::Uuid::from(shop_id))
    .bind(now)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
          shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id, status
        ) values ($1, $2, $3, $4, 'taobao', 'PLAT-REFUND', 'REQUESTED')
        "#,
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(order_id)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into outbound_docs (outbound_doc_id, tenant_id, shipment_id, status)
        values ($1, $2, $3, 'REQUESTED')
        "#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shipment_packages (
          tenant_id, shipment_id, package_no, carrier_code
        ) values ($1, $2, 'PKG-REFUND', 'ZTO')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .execute(&pool)
    .await?;

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    rollback_refunded_order(&db, tenant_id, OrderId(order_id), refund_doc_id).await?;

    let row: (String, String, String, String, String, Option<String>) = sqlx::query_as(
        r#"
        select status, payment_status, oms_status, order_status, fulfillment_status, cancel_reason
        from orders
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.0, "REFUNDED");
    assert_eq!(row.1, "REFUNDED");
    assert_eq!(row.2, "CANCELLED");
    assert_eq!(row.3, "CANCELLED");
    assert_eq!(row.4, "NOT_ALLOCATED");
    let reason = row.5.unwrap_or_default();
    assert!(reason.contains(&refund_doc_id.to_string()));

    let package_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from shipment_packages
        where tenant_id = $1 and shipment_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(package_count, 0);

    Ok(())
}

#[tokio::test]
async fn sync_job_idempotent_outbox() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping sync_job_idempotent_outbox: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());

    let mock_order = serde_json::json!({
        "tid": "TB-1",
        "created": "2026-01-06 10:00:00",
        "payment": "10.00",
        "status": "WAIT_SELLER_SEND_GOODS",
        "orders": {
            "order": [
                { "oid": "OID-1", "num": 1, "price": "10.00", "payment": "10.00" }
            ]
        }
    });

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'sync-tenant', 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
        ) values ($1, $2, 'taobao', 'SHOP-SYNC', 'shop', 'shop', 'AUTHORIZED', '{}'::jsonb, $3)
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(serde_json::json!({ "mock_orders": [mock_order] }))
    .execute(&pool)
    .await?;

    let sync_repo = SyncJobsRepo::new(&pool);
    sync_repo
        .upsert_job(erp_repo::sync_jobs::SyncJobUpsert {
            tenant_id,
            shop_id,
            job_type: "order_sync".to_string(),
            cursor: None,
            watermark: None,
            status: Some("PENDING".to_string()),
            last_error: Some(String::new()),
        })
        .await?;

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    let db = Db::connect_lazy(&database_url)?;
    let adapters = AdapterRegistry::new();

    run_sync_jobs_once(&db, &adapters, ChronoDuration::minutes(60), 10).await?;
    run_sync_jobs_once(&db, &adapters, ChronoDuration::minutes(60), 10).await?;

    let outbox_count: i64 = sqlx::query_scalar(
        "select count(*) from outbox_events where event_type = 'INBOX_EVENT_RECEIVED'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(outbox_count, 1);

    Ok(())
}

#[test]
fn token_refresh_module_exports_expected_symbols() {
    let handle = token_refresh::handle_token_refresh_requested;
    let batch = token_refresh::refresh_tokens_once;
    let refresh = token_refresh::refresh_token_for_candidate;

    let _ = (handle, batch, refresh);
}
