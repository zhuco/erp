//! Legacy stock/source writes and required logs share one transaction.
//! A source replay returns current stock, without inventing an original command.
use anyhow::Result;
use erp_types::{TenantId, UserId};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    inventory::{
        InventoryBalance, InventoryError, InventoryRepo, InventoryReservationAction,
        InventoryReservationInput, InventoryReservationOutcome,
    },
    operation_logs::{
        OP_ACTION_INVENTORY_RELEASED, OP_ACTION_INVENTORY_RESERVED, OperationLogCreate,
        OperationLogsRepo,
    },
};

struct AuditContext {
    tenant: TenantId,
    actor: UserId,
    sku: Uuid,
    source: Uuid,
    operation_action: &'static str,
    audit_action: &'static str,
    metadata: Value,
}

impl InventoryRepo<'_> {
    pub async fn reserve_for_order_item_with_audit(
        &self,
        input: InventoryReservationInput,
        actor: UserId,
    ) -> Result<InventoryBalance> {
        let context = AuditContext {
            tenant: input.tenant_id,
            actor,
            sku: input.sku_id,
            source: input.order_item_id,
            operation_action: OP_ACTION_INVENTORY_RESERVED,
            audit_action: "INVENTORY.RESERVE",
            metadata: json!({"warehouse_id":input.warehouse_id,"sku_id":input.sku_id,
                "order_id":input.order_id,"order_item_id":input.order_item_id,"qty":input.qty}),
        };
        let mut tx = self
            .begin_audited_reservation(context.tenant, context.source, actor)
            .await?;
        let outcome = self.reserve_for_order_item_with_tx(&mut tx, input).await?;
        write_required_logs(&mut tx, context, &outcome).await?;
        tx.commit().await?;
        Ok(outcome.balance)
    }

    pub async fn release_reservation_with_audit(
        &self,
        input: InventoryReservationAction,
        actor: UserId,
    ) -> Result<InventoryBalance> {
        let context = AuditContext {
            tenant: input.tenant_id,
            actor,
            sku: input.sku_id,
            source: input.order_item_id,
            operation_action: OP_ACTION_INVENTORY_RELEASED,
            audit_action: "INVENTORY.RELEASE",
            metadata: json!({"warehouse_id":input.warehouse_id,"sku_id":input.sku_id,
                "order_item_id":input.order_item_id,"qty":input.qty}),
        };
        let mut tx = self
            .begin_audited_reservation(context.tenant, context.source, actor)
            .await?;
        let outcome = self
            .update_reservation_with_tx(&mut tx, input, "RELEASED", "RELEASE", false)
            .await?;
        write_required_logs(&mut tx, context, &outcome).await?;
        tx.commit().await?;
        Ok(outcome.balance)
    }

    async fn begin_audited_reservation(
        &self,
        tenant: TenantId,
        source: Uuid,
        actor: UserId,
    ) -> Result<Transaction<'_, Postgres>> {
        if actor.0.is_nil() {
            return Err(InventoryError::ReservationMismatch.into());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("set local statement_timeout='10s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local lock_timeout='5s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(tenant.to_string())
            .bind(format!("legacy-reservation-api:{source}"))
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }
}

async fn write_required_logs(
    tx: &mut Transaction<'_, Postgres>,
    mut context: AuditContext,
    outcome: &InventoryReservationOutcome,
) -> Result<()> {
    if !outcome.applied {
        return Ok(());
    }
    context.metadata["result"] = json!({"warehouse_id":outcome.balance.warehouse_id,
        "sku_id":outcome.balance.sku_id,"quantities":outcome.balance.quantities});
    OperationLogsRepo::insert_with_tx(
        &mut **tx,
        OperationLogCreate {
            tenant_id: context.tenant,
            actor_type: "USER".into(),
            actor_user_id: Some(context.actor),
            action: context.operation_action.into(),
            ref_type: Some("ORDER_ITEM".into()),
            ref_id: Some(context.source.to_string()),
            metadata: Some(context.metadata.clone()),
        },
    )
    .await?;
    AuditLogsRepo::insert_with_tx(
        &mut **tx,
        AuditLogCreate {
            tenant_id: context.tenant,
            user_id: Some(context.actor),
            action: context.audit_action.into(),
            entity_type: "INVENTORY_BALANCE".into(),
            entity_id: Some(context.sku.to_string()),
            before_json: None,
            after_json: Some(context.metadata),
            ip: None,
            ua: None,
        },
    )
    .await?;
    Ok(())
}
