use chrono::{DateTime, Utc};
use erp_domain::quantity::{QTY_SCALE, QuantityError, ScaledQty, format_qty};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    inventory::InventoryService,
    inventory_precision::{InventoryPrecisionError, InventoryQuantityKey},
};

pub const MAX_ATP_BATCHES: i64 = 1000;

#[derive(Debug, thiserror::Error)]
pub enum AtpQuoteError {
    #[error("interactive ATP supports at most 1000 batches per warehouse/SKU")]
    BatchLimitExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtpAllocation {
    pub batch_id: uuid::Uuid,
    pub batch_no: String,
    pub expire_at: Option<DateTime<Utc>>,
    pub allocated_qty: i64,
    /// Authoritative base-unit quantity; the integer above is a legacy projection.
    pub quantity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtpQuantities {
    pub on_hand: String,
    pub reserved: String,
    pub frozen: String,
    pub available: String,
    pub request_qty: String,
    pub atp_qty: String,
    pub shortage_qty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtpQuote {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub on_hand: i64,
    pub reserved: i64,
    pub frozen: i64,
    pub available: i64,
    pub request_qty: i64,
    pub atp_qty: i64,
    pub shortage_qty: i64,
    pub quantities: AtpQuantities,
    pub allocations: Vec<AtpAllocation>,
}

pub struct AtpInventoryRepo<'a> {
    pool: &'a PgPool,
}

#[derive(Debug, Clone)]
struct BatchCandidate {
    batch_id: uuid::Uuid,
    batch_no: String,
    expire_at: Option<DateTime<Utc>>,
    available_qty: i64,
}

fn allocate_fefo_batches(
    candidates: Vec<BatchCandidate>,
    request_qty: i64,
) -> (i64, Vec<AtpAllocation>) {
    let mut remaining = request_qty.max(0);
    let mut allocations = Vec::new();
    for candidate in candidates {
        if remaining <= 0 {
            break;
        }
        if candidate.available_qty <= 0 {
            continue;
        }
        let allocated = candidate.available_qty.min(remaining);
        allocations.push(AtpAllocation {
            batch_id: candidate.batch_id,
            batch_no: candidate.batch_no,
            expire_at: candidate.expire_at,
            allocated_qty: allocated / QTY_SCALE,
            quantity: format_qty(ScaledQty::from_micros(allocated)),
        });
        remaining -= allocated;
    }
    let allocated_total = request_qty.max(0) - remaining;
    (allocated_total, allocations)
}

impl<'a> AtpInventoryRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn quote_fefo(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        request_qty: i64,
    ) -> anyhow::Result<AtpQuote> {
        self.quote_fefo_exact(
            tenant_id,
            warehouse_id,
            sku_id,
            ScaledQty::from_legacy_whole_units(request_qty)?,
        )
        .await
    }

    pub async fn quote_fefo_exact(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        request_qty: ScaledQty,
    ) -> anyhow::Result<AtpQuote> {
        if request_qty.is_negative() {
            return Err(QuantityError::InvalidText.into());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read read only")
            .execute(&mut *tx)
            .await?;
        let balance = InventoryService::new(self.pool)
            .get_precise_balance_with_tx(
                &mut tx,
                InventoryQuantityKey {
                    tenant_id,
                    warehouse_id,
                    sku_id,
                },
            )
            .await?;
        let (on_hand, reserved, frozen, mut available) = match balance {
            Some(balance) => (
                balance.on_hand.as_micros(),
                balance.reserved.as_micros(),
                balance.frozen.as_micros(),
                balance.available()?.as_micros(),
            ),
            None => (0, 0, 0, 0),
        };

        let batch_enabled: Option<bool> = sqlx::query_scalar(
            r#"
            select batch_enabled
            from skus
            where tenant_id = $1 and sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_optional(&mut *tx)
        .await?;

        let request_qty = request_qty.as_micros();
        let mut allocations: Vec<AtpAllocation> = Vec::new();
        let mut atp_qty = request_qty.min(available);

        if batch_enabled.unwrap_or(false) {
            #[derive(sqlx::FromRow)]
            struct BatchRow {
                batch_id: uuid::Uuid,
                batch_no: Option<String>,
                expire_at: Option<DateTime<Utc>>,
                on_hand_micros: Option<i64>,
                reserved_micros: Option<i64>,
                usable: bool,
            }

            let rows: Vec<BatchRow> = sqlx::query_as(
                r#"
                select
                    ibb.batch_id,
                    sb.batch_no,
                    sb.expire_at,
                    ibb.on_hand_micros,
                    ibb.reserved_micros,
                    coalesce(sb.expire_at > transaction_timestamp(), true) as usable
                from inventory_batch_balances ibb
                left join sku_batches sb on sb.tenant_id=ibb.tenant_id
                    and sb.sku_id=ibb.sku_id and sb.batch_id=ibb.batch_id
                where ibb.tenant_id = $1 and ibb.warehouse_id = $2 and ibb.sku_id = $3
                order by sb.expire_at asc nulls last, ibb.batch_id asc
                limit $4
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(sku_id)
            .bind(MAX_ATP_BATCHES + 1)
            .fetch_all(&mut *tx)
            .await?;

            if rows.len() > MAX_ATP_BATCHES as usize {
                return Err(AtpQuoteError::BatchLimitExceeded.into());
            }

            let mut candidates = Vec::new();
            let mut batch_on_hand = 0i128;
            let mut batch_reserved = 0i128;
            let mut batch_available = 0i128;
            for row in rows {
                let missing = || {
                    InventoryPrecisionError::ReconciliationRequired(
                        "batch exact quantity or owned metadata missing",
                    )
                };
                let hand = row.on_hand_micros.ok_or_else(missing)?;
                let held = row.reserved_micros.ok_or_else(missing)?;
                let batch_no = row.batch_no.ok_or_else(missing)?;
                if held < 0 || hand < held {
                    return Err(InventoryPrecisionError::ReconciliationRequired(
                        "batch physical or reserved quantity invalid",
                    )
                    .into());
                }
                batch_on_hand += i128::from(hand);
                batch_reserved += i128::from(held);
                if row.usable {
                    let free = hand - held;
                    batch_available += i128::from(free);
                    candidates.push(BatchCandidate {
                        batch_id: row.batch_id,
                        batch_no,
                        expire_at: row.expire_at,
                        available_qty: free,
                    });
                }
            }
            if batch_on_hand != i128::from(on_hand) || batch_reserved > i128::from(reserved) {
                return Err(InventoryPrecisionError::ReconciliationRequired(
                    "batch quantities disagree with the SKU physical or reserved balance",
                )
                .into());
            }
            // Sum in a wide type before capping by the validated SKU balance.
            available = i64::try_from(i128::from(available).min(batch_available))?;
            let (allocated_total, allocated_rows) =
                allocate_fefo_batches(candidates, request_qty.min(available));
            atp_qty = allocated_total;
            allocations = allocated_rows;
        }

        let shortage_qty = request_qty - atp_qty;
        tx.commit().await?;

        let quantities = AtpQuantities {
            on_hand: format_qty(ScaledQty::from_micros(on_hand)),
            reserved: format_qty(ScaledQty::from_micros(reserved)),
            frozen: format_qty(ScaledQty::from_micros(frozen)),
            available: format_qty(ScaledQty::from_micros(available)),
            request_qty: format_qty(ScaledQty::from_micros(request_qty)),
            atp_qty: format_qty(ScaledQty::from_micros(atp_qty)),
            shortage_qty: format_qty(ScaledQty::from_micros(shortage_qty)),
        };

        Ok(AtpQuote {
            tenant_id,
            warehouse_id,
            sku_id,
            on_hand: on_hand / QTY_SCALE,
            reserved: reserved / QTY_SCALE,
            frozen: frozen / QTY_SCALE,
            available: available / QTY_SCALE,
            request_qty: request_qty / QTY_SCALE,
            atp_qty: atp_qty / QTY_SCALE,
            // Keep old whole callers' promise/shortage equation conservative.
            shortage_qty: request_qty / QTY_SCALE - atp_qty / QTY_SCALE,
            quantities,
            allocations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchCandidate, allocate_fefo_batches};
    use erp_domain::quantity::QTY_SCALE;

    #[test]
    fn fefo_allocation_prioritizes_earlier_expiry() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();
        let candidates = vec![
            BatchCandidate {
                batch_id: a,
                batch_no: "A".to_string(),
                expire_at: Some(now),
                available_qty: 8 * QTY_SCALE,
            },
            BatchCandidate {
                batch_id: b,
                batch_no: "B".to_string(),
                expire_at: Some(now + chrono::Duration::days(10)),
                available_qty: 30 * QTY_SCALE,
            },
        ];

        let (allocated_total, allocations) = allocate_fefo_batches(candidates, 20 * QTY_SCALE);
        assert_eq!(allocated_total, 20 * QTY_SCALE);
        assert_eq!(allocations.len(), 2);
        assert_eq!(allocations[0].batch_id, a);
        assert_eq!(allocations[0].allocated_qty, 8);
        assert_eq!(allocations[1].batch_id, b);
        assert_eq!(allocations[1].allocated_qty, 12);
    }

    #[test]
    fn fefo_allocation_caps_when_short() {
        let candidates = vec![BatchCandidate {
            batch_id: uuid::Uuid::new_v4(),
            batch_no: "A".to_string(),
            expire_at: None,
            available_qty: 5 * QTY_SCALE,
        }];

        let (allocated_total, allocations) = allocate_fefo_batches(candidates, 12 * QTY_SCALE);
        assert_eq!(allocated_total, 5 * QTY_SCALE);
        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0].allocated_qty, 5);
    }
}
