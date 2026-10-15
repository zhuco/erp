//! Read-only, complete exact batch context before a bounded manufacturing page.
use super::{MaterialDocumentError, ReceiptValueActor, SharedContractRepo, receipt_authority};
use crate::{inventory::InventoryService, inventory_precision::InventoryQuantityKey};
use erp_types::TenantId;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct MaterialBatchQuery<'a> {
    pub work_order_id: Uuid,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub text: &'a str,
    pub offset: i64,
    pub limit: i64,
}
#[derive(Serialize)]
pub struct MaterialBatchOption {
    pub batch_id: Uuid,
    pub batch_no: Option<String>,
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    pub on_hand_micros: Option<String>,
    pub reserved_micros: Option<String>,
    pub available_micros: Option<String>,
    pub state: String,
}
#[derive(sqlx::FromRow)]
struct Stored {
    batch_id: Uuid,
    batch_no: Option<String>,
    expire_at: Option<chrono::DateTime<chrono::Utc>>,
    hand: i64,
    reserved: i64,
    hand_micros: Option<i64>,
    reserved_micros: Option<i64>,
}
impl SharedContractRepo<'_> {
    pub async fn material_batch_options(
        &self,
        tenant: TenantId,
        input: MaterialBatchQuery<'_>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        if input.work_order_id.is_nil()
            || input.warehouse_id.is_nil()
            || input.sku_id.is_nil()
            || input.text.len() > 128
            || input.text.chars().any(char::is_control)
            || !(0..=1000000).contains(&input.offset)
            || !(1..=2000).contains(&input.limit)
        {
            anyhow::bail!("invalid material batch query");
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read")
            .execute(&mut *tx)
            .await?;
        let permit = receipt_authority::lock_permissions(
            &mut tx,
            tenant,
            ReceiptValueActor {
                warehouse_id: input.warehouse_id,
                ..actor
            },
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let owned: bool = sqlx::query_scalar(
            "select exists(select 1 from mfg_work_orders where tenant_id=$1 and work_order_id=$2)",
        )
        .bind(tenant.0)
        .bind(input.work_order_id)
        .fetch_one(&mut *tx)
        .await?;
        if !owned {
            return Err(MaterialDocumentError::NotFound.into());
        }
        let enabled:Option<bool>=sqlx::query_scalar("select batch_enabled from skus where tenant_id=$1 and sku_id=$2 and is_active for share")
            .bind(tenant.0).bind(input.sku_id).fetch_optional(&mut *tx).await?;
        let enabled = enabled.ok_or(MaterialDocumentError::NotFound)?;
        let rows:Vec<Stored>=sqlx::query_as("select q.batch_id,b.batch_no,b.expire_at,q.on_hand as hand,q.reserved,q.on_hand_micros as hand_micros,q.reserved_micros from inventory_batch_balances q left join sku_batches b on b.tenant_id=q.tenant_id and b.sku_id=q.sku_id and b.batch_id=q.batch_id where q.tenant_id=$1 and q.warehouse_id=$2 and q.sku_id=$3 order by q.batch_id limit 1001")
            .bind(tenant.0).bind(input.warehouse_id).bind(input.sku_id).fetch_all(&mut *tx).await?;
        if rows.len() > 1000 {
            return Err(crate::inventory_batch_effects::InventoryBatchLimit.into());
        }
        let physical = InventoryService::new(self.pool)
            .get_precise_balance_with_tx(
                &mut tx,
                InventoryQuantityKey {
                    tenant_id: tenant,
                    warehouse_id: input.warehouse_id,
                    sku_id: input.sku_id,
                },
            )
            .await;
        let (warehouse_available, warehouse_hand, warehouse_reserved, physical_unknown) =
            match physical {
                Ok(Some(balance)) => (
                    Some(
                        i128::from(balance.on_hand.as_micros())
                            - i128::from(balance.reserved.as_micros())
                            - i128::from(balance.frozen.as_micros()),
                    ),
                    Some(i128::from(balance.on_hand.as_micros())),
                    Some(i128::from(balance.reserved.as_micros())),
                    false,
                ),
                Ok(None) => (None, None, None, true),
                Err(error)
                    if error
                        .downcast_ref::<crate::inventory_precision::InventoryPrecisionError>()
                        .is_some() =>
                {
                    (None, None, None, true)
                }
                Err(error) => return Err(error),
            };
        let valid = |row: &Stored| match (row.hand_micros, row.reserved_micros) {
            (Some(hand), Some(reserved)) => {
                row.batch_no.is_some()
                    && hand >= 0
                    && reserved >= 0
                    && reserved <= hand
                    && hand / 1000000 == row.hand
                    && reserved / 1000000 == row.reserved
            }
            _ => false,
        };
        let hand: i128 = rows
            .iter()
            .filter_map(|r| r.hand_micros)
            .map(i128::from)
            .sum();
        let reserved: i128 = rows
            .iter()
            .filter_map(|r| r.reserved_micros)
            .map(i128::from)
            .sum();
        let reconciliation = physical_unknown
            || rows.iter().any(|r| !valid(r))
            || Some(hand) != warehouse_hand
            || warehouse_reserved.is_none_or(|r| reserved > r);
        let observed: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("select clock_timestamp()")
                .fetch_one(&mut *tx)
                .await?;
        let keyword = input.text.trim().to_lowercase();
        let mut matches = rows
            .into_iter()
            .filter(|r| {
                keyword.is_empty()
                    || r.batch_no
                        .as_ref()
                        .is_some_and(|s| s.to_lowercase().contains(&keyword))
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|r| (r.expire_at.is_none(), r.expire_at, r.batch_id));
        let total = matches.len();
        let data = matches
            .into_iter()
            .skip(usize::try_from(input.offset)?)
            .take(usize::try_from(input.limit)?)
            .map(|r| {
                let exact = valid(&r);
                let available = match (r.hand_micros, r.reserved_micros, warehouse_available) {
                    (Some(h), Some(held), Some(whole)) if exact && !reconciliation => {
                        Some((i128::from(h) - i128::from(held)).min(whole).to_string())
                    }
                    _ => None,
                };
                let state = if !exact || reconciliation {
                    "RECONCILIATION_REQUIRED"
                } else if r.expire_at.is_some_and(|e| e <= observed) {
                    "EXPIRED"
                } else if available.as_deref() == Some("0") {
                    "EMPTY"
                } else {
                    "READY"
                };
                MaterialBatchOption {
                    batch_id: r.batch_id,
                    batch_no: r.batch_no,
                    expire_at: r.expire_at,
                    on_hand_micros: r.hand_micros.map(|n| n.to_string()),
                    reserved_micros: r.reserved_micros.map(|n| n.to_string()),
                    available_micros: available,
                    state: state.into(),
                }
            })
            .collect::<Vec<_>>();
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok(
            json!({"work_order_id":input.work_order_id,"warehouse_id":input.warehouse_id,"sku_id":input.sku_id,"batch_enabled":enabled,
            "observed_at":observed.to_rfc3339(),"warehouse_available_micros":warehouse_available.map(|n|n.to_string()),"needs_reconciliation":reconciliation,
            "total":total,"offset":input.offset,"limit":input.limit,"data":data}),
        )
    }
}
