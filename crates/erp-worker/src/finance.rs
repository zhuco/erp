use super::*;

pub(super) async fn handle_profit_compute_requested(
    db: &Db,
    payload: ProfitComputeRequestedPayload,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            order_id = %payload.order_id,
            "read-only mode active; skipping profit compute"
        );
        return Ok(());
    }

    let profit_repo = ProfitSnapshotsRepo::new(db.pool());
    let (revenue_cent, cost_cent, platform_fee_cent, shipping_fee_cent) = profit_repo
        .compute_profit_inputs(payload.tenant_id, payload.order_id)
        .await?;
    let gross_profit_cent = revenue_cent - cost_cent - platform_fee_cent - shipping_fee_cent;
    let computed_at = Utc::now();
    let snapshot = profit_repo
        .upsert_snapshot(ProfitSnapshotUpsert {
            tenant_id: payload.tenant_id,
            order_id: payload.order_id,
            revenue_cent,
            cost_cent,
            platform_fee_cent,
            shipping_fee_cent,
            gross_profit_cent,
            computed_at,
        })
        .await?;

    let audit_repo = AuditLogsRepo::new(db.pool());
    audit_repo
        .insert(AuditLogCreate {
            tenant_id: payload.tenant_id,
            user_id: None,
            action: "FINANCE.PROFIT_SNAPSHOT_UPSERT".to_string(),
            entity_type: "profit_snapshot".to_string(),
            entity_id: Some(payload.order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&snapshot)?),
            ip: None,
            ua: None,
        })
        .await?;

    Ok(())
}
