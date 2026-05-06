use super::*;

pub(super) async fn handle_refund_requested(
    db: &Db,
    payload: RefundRequestedPayload,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            refund_doc_id = %payload.refund_doc_id,
            "read-only mode active; skipping refund request"
        );
        return Ok(());
    }

    let repo = AfterSalesRepo::new(db.pool());
    let refund_doc = repo
        .mark_refund_done(payload.tenant_id, payload.refund_doc_id)
        .await?;
    metrics().inc_after_sales_sync_success();

    if let Ok(Some(after_sales)) = repo
        .get_after_sales(payload.tenant_id, refund_doc.after_sales_id)
        .await
    {
        let cycle_ms = Utc::now()
            .signed_duration_since(after_sales.created_at)
            .num_milliseconds();
        if cycle_ms > 0 {
            metrics().add_after_sales_cycle_time_ms(cycle_ms as u64);
        }
        if let Err(err) =
            rollback_refunded_order(db, payload.tenant_id, after_sales.order_id, refund_doc.id)
                .await
        {
            warn!(
                tenant_id = %payload.tenant_id,
                order_id = %after_sales.order_id,
                error = %err,
                "refund rollback failed"
            );
        }
    }

    let audit_repo = AuditLogsRepo::new(db.pool());
    let after_json = serde_json::json!({
        "refund_doc_id": refund_doc.id,
        "after_sales_id": refund_doc.after_sales_id,
        "amount_cent": refund_doc.amount_cent,
        "status": refund_doc.status.to_string(),
        "channel": refund_doc.channel.to_string(),
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id: payload.tenant_id,
            user_id: None,
            action: "AFTERSALES.REFUND_DONE".to_string(),
            entity_type: "refund_doc".to_string(),
            entity_id: Some(refund_doc.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "audit log insert failed for refund");
    }

    Ok(())
}

pub(super) async fn rollback_refunded_order(
    db: &Db,
    tenant_id: TenantId,
    order_id: OrderId,
    refund_doc_id: uuid::Uuid,
) -> anyhow::Result<()> {
    let shipment_rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        r#"
        select shipment_id, status
        from shipments
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(db.pool())
    .await?;
    let shipment_ids: Vec<uuid::Uuid> = shipment_rows
        .iter()
        .map(|(shipment_id, _)| *shipment_id)
        .collect();
    let has_shipped_shipment = shipment_rows.iter().any(|(_, status)| {
        status == "SHIPPED" || status == "SUCCESS" || status == "PARTIALLY_SHIPPED"
    });
    if has_shipped_shipment {
        return Ok(());
    }

    #[derive(sqlx::FromRow)]
    struct OutboundRow {
        outbound_doc_id: uuid::Uuid,
        status: String,
    }

    let outbound_rows: Vec<OutboundRow> = sqlx::query_as(
        r#"
        select d.outbound_doc_id, d.status
        from outbound_docs d
        join shipments s
          on s.tenant_id = d.tenant_id
         and s.shipment_id = d.shipment_id
        where d.tenant_id = $1 and s.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(db.pool())
    .await?;

    if outbound_rows.iter().any(|row| row.status == "SHIPPED") {
        return Ok(());
    }
    let outbound_doc_ids: Vec<uuid::Uuid> = outbound_rows
        .iter()
        .map(|row| row.outbound_doc_id)
        .collect();

    if has_printed_jobs(
        db.pool(),
        tenant_id,
        order_id,
        &shipment_ids,
        &outbound_doc_ids,
    )
    .await?
    {
        return Ok(());
    }

    let fulfillment_repo = FulfillmentRepo::new(db.pool());
    let _ = fulfillment_repo
        .release_order_reservations(tenant_id, order_id)
        .await;

    let wms_repo = WmsRepo::new(db.pool());
    let cancelled_docs = wms_repo
        .cancel_outbounds_for_order(tenant_id, order_id)
        .await
        .unwrap_or_default();

    if !shipment_ids.is_empty() {
        let _ = sqlx::query(
            r#"
            delete from shipment_packages
            where tenant_id = $1 and shipment_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&shipment_ids)
        .execute(db.pool())
        .await;
    }

    cancel_print_jobs(
        db.pool(),
        tenant_id,
        order_id,
        &shipment_ids,
        &outbound_doc_ids,
    )
    .await?;

    let orders_repo = OrdersRepo::new(db.pool());
    if let Ok(Some(state)) = orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        let _ = orders_repo
            .update_order_review_state(
                tenant_id,
                order_id,
                erp_repo::orders::OrderReviewUpdate {
                    oms_status: erp_types::OmsOrderStatus::Cancelled,
                    order_status: erp_types::OrderMainStatus::Cancelled,
                    review_status: state.review_status,
                    fulfillment_status: erp_types::OrderFulfillmentStatus::NotAllocated,
                    exception_status: erp_types::OrderExceptionStatus::Closed,
                    exception_flags: state.exception_flags.clone(),
                    expected_version: Some(state.version),
                },
            )
            .await;
    }

    let reason = format!("refund rollback: {refund_doc_id}");
    let _ = orders_repo
        .mark_order_refunded(tenant_id, order_id, Some(reason))
        .await;

    let audit_repo = AuditLogsRepo::new(db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: None,
            action: "ORDER_REFUNDED_AUTO".to_string(),
            entity_type: "order".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "refund_doc_id": refund_doc_id,
                "shipment_ids": shipment_ids,
                "outbound_doc_ids": outbound_doc_ids,
                "cancelled_outbound_docs": cancelled_docs,
            })),
            ip: None,
            ua: None,
        })
        .await;
    metrics().inc_after_sales_compensation();

    Ok(())
}

pub(super) async fn has_printed_jobs(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    shipment_ids: &[uuid::Uuid],
    outbound_doc_ids: &[uuid::Uuid],
) -> anyhow::Result<bool> {
    let order_done: bool = sqlx::query_scalar(
        r#"
        select exists(
            select 1
            from print_jobs
            where tenant_id = $1
              and ref_type = 'ORDER'
              and ref_id = $2
              and status = 'DONE'
        )
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_one(pool)
    .await?;
    if order_done {
        return Ok(true);
    }

    if !shipment_ids.is_empty() {
        let shipped_done: bool = sqlx::query_scalar(
            r#"
            select exists(
                select 1
                from print_jobs
                where tenant_id = $1
                  and ref_type = 'SHIPMENT'
                  and ref_id = any($2)
                  and status = 'DONE'
            )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_ids)
        .fetch_one(pool)
        .await?;
        if shipped_done {
            return Ok(true);
        }
    }

    if !outbound_doc_ids.is_empty() {
        let outbound_done: bool = sqlx::query_scalar(
            r#"
            select exists(
                select 1
                from print_jobs
                where tenant_id = $1
                  and ref_type = 'OUTBOUND_DOC'
                  and ref_id = any($2)
                  and status = 'DONE'
            )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_ids)
        .fetch_one(pool)
        .await?;
        if outbound_done {
            return Ok(true);
        }
    }

    Ok(false)
}

pub(super) async fn cancel_print_jobs(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    shipment_ids: &[uuid::Uuid],
    outbound_doc_ids: &[uuid::Uuid],
) -> anyhow::Result<()> {
    let reason = format!("order refunded: {}", order_id);

    sqlx::query(
        r#"
        update print_jobs
        set status = 'CANCELLED',
            dispatch_status = 'FAILED',
            last_error = $4,
            updated_at = now()
        where tenant_id = $1
          and ref_type = $2
          and ref_id = $3
          and status in ('QUEUED','PENDING','RUNNING')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("ORDER")
    .bind(uuid::Uuid::from(order_id))
    .bind(&reason)
    .execute(pool)
    .await?;

    if !shipment_ids.is_empty() {
        sqlx::query(
            r#"
            update print_jobs
            set status = 'CANCELLED',
                dispatch_status = 'FAILED',
                last_error = $4,
                updated_at = now()
            where tenant_id = $1
              and ref_type = $2
              and ref_id = any($3)
              and status in ('QUEUED','PENDING','RUNNING')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("SHIPMENT")
        .bind(shipment_ids)
        .bind(&reason)
        .execute(pool)
        .await?;
    }

    if !outbound_doc_ids.is_empty() {
        sqlx::query(
            r#"
            update print_jobs
            set status = 'CANCELLED',
                dispatch_status = 'FAILED',
                last_error = $4,
                updated_at = now()
            where tenant_id = $1
              and ref_type = $2
              and ref_id = any($3)
              and status in ('QUEUED','PENDING','RUNNING')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("OUTBOUND_DOC")
        .bind(outbound_doc_ids)
        .bind(&reason)
        .execute(pool)
        .await?;
    }

    Ok(())
}
