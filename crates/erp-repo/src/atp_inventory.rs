use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtpAllocation {
    pub batch_id: uuid::Uuid,
    pub batch_no: String,
    pub expire_at: Option<DateTime<Utc>>,
    pub allocated_qty: i64,
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
            allocated_qty: allocated,
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
        #[derive(sqlx::FromRow)]
        struct BalanceRow {
            on_hand: i64,
            reserved: i64,
            frozen: i64,
        }

        let balance: Option<BalanceRow> = sqlx::query_as(
            r#"
            select on_hand, reserved, frozen
            from inventory_balances
            where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .fetch_optional(self.pool)
        .await?;

        let (on_hand, reserved, frozen) = match balance {
            Some(v) => (v.on_hand, v.reserved, v.frozen),
            None => (0, 0, 0),
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
        .fetch_optional(self.pool)
        .await?;

        let mut available = (on_hand - reserved - frozen).max(0);
        let request_qty = request_qty.max(0);
        let mut allocations: Vec<AtpAllocation> = Vec::new();
        let mut atp_qty = request_qty.min(available);

        if batch_enabled.unwrap_or(false) {
            #[derive(sqlx::FromRow)]
            struct BatchRow {
                batch_id: uuid::Uuid,
                batch_no: String,
                expire_at: Option<DateTime<Utc>>,
                on_hand: i64,
                reserved: i64,
            }

            let rows: Vec<BatchRow> = sqlx::query_as(
                r#"
                select
                    ibb.batch_id,
                    sb.batch_no,
                    sb.expire_at,
                    ibb.on_hand,
                    ibb.reserved
                from inventory_batch_balances ibb
                join sku_batches sb on sb.batch_id = ibb.batch_id
                where ibb.tenant_id = $1 and ibb.warehouse_id = $2 and ibb.sku_id = $3
                order by sb.expire_at asc nulls last, ibb.batch_id asc
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(sku_id)
            .fetch_all(self.pool)
            .await?;

            let candidates: Vec<BatchCandidate> = rows
                .into_iter()
                .map(|row| BatchCandidate {
                    batch_id: row.batch_id,
                    batch_no: row.batch_no,
                    expire_at: row.expire_at,
                    available_qty: (row.on_hand - row.reserved).max(0),
                })
                .collect();

            let batch_available_total: i64 = candidates.iter().map(|row| row.available_qty).sum();
            available = available.min(batch_available_total.max(0));
            let (allocated_total, allocated_rows) =
                allocate_fefo_batches(candidates, request_qty.min(available));
            atp_qty = allocated_total;
            allocations = allocated_rows;
        }

        let shortage_qty = request_qty - atp_qty;

        Ok(AtpQuote {
            tenant_id,
            warehouse_id,
            sku_id,
            on_hand,
            reserved,
            frozen,
            available,
            request_qty,
            atp_qty,
            shortage_qty,
            allocations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchCandidate, allocate_fefo_batches};

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
                available_qty: 8,
            },
            BatchCandidate {
                batch_id: b,
                batch_no: "B".to_string(),
                expire_at: Some(now + chrono::Duration::days(10)),
                available_qty: 30,
            },
        ];

        let (allocated_total, allocations) = allocate_fefo_batches(candidates, 20);
        assert_eq!(allocated_total, 20);
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
            available_qty: 5,
        }];

        let (allocated_total, allocations) = allocate_fefo_batches(candidates, 12);
        assert_eq!(allocated_total, 5);
        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0].allocated_qty, 5);
    }
}
