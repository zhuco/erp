//! Original-source cost returns and one committed cost history per item.

use super::{SharedContractRepo, SharedWrite, insert_receipt, load_receipt, succeeded};
use erp_domain::receipt::{ReceiptError, admit_retry};
use erp_domain::trade::{
    CostRebuildInput, CostReturnInput, admit_cost_rebuild, cost_unit_and_residual,
    cumulative_return_cost,
};
use erp_types::TenantId;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

struct CostEntryInput<'a> {
    tenant_id: TenantId,
    request_id: &'a str,
    payload_hash: &'a str,
    item_code: &'a str,
    kind: &'a str,
    qty_micros: i64,
    amount_cent: i64,
    source_ref: &'a str,
    period_code: &'a str,
}

impl SharedContractRepo<'_> {
    /// Reads the rebuilt moving-average position: quantity, value, unit, and whether it is estimated.
    pub async fn read_item_cost(
        &self,
        tenant_id: TenantId,
        item_code: &str,
    ) -> anyhow::Result<(i64, i64, i64, bool)> {
        let rows = sqlx::query_as::<_, CostRow>(COST_ROWS_SQL)
            .bind(Uuid::from(tenant_id))
            .bind(item_code)
            .fetch_all(self.pool)
            .await?;
        let position = cost_position(&rows)?;
        Ok((
            position.qty,
            position.value,
            position.unit,
            position.estimated || position.qty == 0,
        ))
    }

    /// Posts one moving-average receipt. Replay does not add value again.
    pub async fn post_cost_receipt(
        &self,
        input: super::CostReceiptInput<'_>,
    ) -> anyhow::Result<Result<(SharedWrite, i64, bool), ReceiptError>> {
        let super::CostReceiptInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            qty_micros,
            amount_cent,
            period_code,
        } = input;
        self.post_cost_entry(CostEntryInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            kind: "RECEIPT",
            qty_micros,
            amount_cent,
            source_ref: "",
            period_code,
        })
        .await
    }

    /// Issues at the current average. The stored issue amount is not changed by a later receipt.
    pub async fn post_cost_issue(
        &self,
        input: super::CostIssueInput<'_>,
    ) -> anyhow::Result<Result<(SharedWrite, i64, bool), ReceiptError>> {
        let super::CostIssueInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            qty_micros,
            period_code,
        } = input;
        self.post_cost_entry(CostEntryInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            kind: "ISSUE",
            qty_micros,
            amount_cent: 0,
            source_ref: request_id,
            period_code,
        })
        .await
    }

    /// Reads one known zero unit cost. A positive value or an estimate fails the read.
    /// This is a stored cost read, not a page and not an ERP-P timing pass.
    pub async fn read_zero_unit_cost(
        &self,
        tenant_id: TenantId,
        item_code: &str,
    ) -> anyhow::Result<(i64, i64, i64, bool)> {
        let row: Option<(i64, i64, i64, bool)> = sqlx::query_as(
            "select qty_micros, amount_cent, unit_cent, estimated from shared_item_costs where tenant_id = $1 and item_code = $2 and kind = 'RECEIPT'",
        )
        .bind(Uuid::from(tenant_id))
        .bind(item_code.trim())
        .fetch_optional(self.pool)
        .await?;
        let (qty, value, unit, estimated) = row.unwrap_or((0, 0, 0, true));
        let unit = erp_domain::trade::zero_unit_cost_legal(qty, value, unit, estimated)
            .map_err(|_| anyhow::anyhow!("zero unit cost is not a known legal cost"))?;
        Ok((qty, value, unit, estimated))
    }

    /// Reads three one-micro returns of a two-cent issue. A different split fails the read.
    /// This is a stored cost read, not a page and not an ERP-P timing pass.
    pub async fn read_three_micro_return(
        &self,
        tenant_id: TenantId,
        item_code: &str,
        issue_request: &str,
    ) -> anyhow::Result<([i64; 3], i64, i64)> {
        let original: Option<(i64, i64)> = sqlx::query_as(
            "select amount_cent, count(*)::bigint from shared_item_costs where tenant_id = $1 and item_code = $2 and kind = 'ISSUE' and source_ref = $3 group by amount_cent",
        )
        .bind(Uuid::from(tenant_id))
        .bind(item_code.trim())
        .bind(issue_request.trim())
        .fetch_optional(self.pool)
        .await?;
        let (original_cent, issue_rows) = original.unwrap_or((0, 0));
        let amounts: Vec<i64> = sqlx::query_scalar(
            "select amount_cent from shared_item_costs where tenant_id = $1 and item_code = $2 and kind = 'RETURN' and source_ref = $3 order by entry_sequence",
        )
        .bind(Uuid::from(tenant_id))
        .bind(item_code.trim())
        .bind(issue_request.trim())
        .fetch_all(self.pool)
        .await?;
        let splits = match amounts.as_slice() {
            [first, second, third] => [*first, *second, *third],
            _ => [0, 0, 0],
        };
        let total = erp_domain::trade::three_micro_return_exact(splits, original_cent, issue_rows)
            .map_err(|_| anyhow::anyhow!("three micro returns are not exactly two cents"))?;
        Ok((splits, original_cent, total))
    }

    /// Returns stock at the original issue cost. The historical issue row stays unchanged.
    pub async fn post_cost_return(
        &self,
        input: super::CostReturnCommand<'_>,
    ) -> anyhow::Result<Result<(SharedWrite, i64, bool), ReceiptError>> {
        let super::CostReturnCommand {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            issue_request,
            return_qty,
            period_code,
        } = input;
        self.post_cost_entry(CostEntryInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            kind: "RETURN",
            qty_micros: return_qty,
            amount_cent: 0,
            source_ref: issue_request,
            period_code,
        })
        .await
    }

    /// Posts a closed-period adjustment as a new row in the next period. The original row stays.
    /// This is a stored cost read, not a page and not an ERP-P timing pass.
    pub async fn post_closed_period_adjustment(
        &self,
        input: super::ClosedCostAdjustmentInput<'_>,
    ) -> anyhow::Result<Result<(SharedWrite, Uuid), ReceiptError>> {
        let super::ClosedCostAdjustmentInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            amount_cent,
            closed_period,
            next_period,
        } = input;
        if item_code.trim().is_empty()
            || closed_period.trim().is_empty()
            || next_period.trim().is_empty()
            || next_period.trim() == closed_period.trim()
            || amount_cent == 0
        {
            anyhow::bail!(
                "closed period adjustment needs an item, a later period, and a nonzero amount"
            );
        }
        let mut tx = self.pool.begin().await?;
        lock_cost_command(&mut tx, tenant_id, request_id).await?;
        lock_item_cost(&mut tx, tenant_id, item_code).await?;
        match admit_retry(
            load_receipt(&mut tx, tenant_id, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(existing)) => {
                let entry_id = Uuid::parse_str(&existing.effect_id)?;
                tx.rollback().await?;
                Ok(Ok((SharedWrite::Replay(existing), entry_id)))
            }
            Err(error) => {
                tx.rollback().await?;
                Ok(Err(error))
            }
            Ok(None) => {
                let closed: i64 = sqlx::query_scalar(
                    "select count(*) from shared_cost_periods where tenant_id = $1 and period_code = $2 and status = 'CLOSED'",
                )
                .bind(Uuid::from(tenant_id))
                .bind(closed_period.trim())
                .fetch_one(&mut *tx)
                .await?;
                if closed != 1 {
                    anyhow::bail!("CONFLICT: adjustment period is not closed");
                }
                let next_closed: i64 = sqlx::query_scalar(
                    "select count(*) from shared_cost_periods where tenant_id = $1 and period_code = $2 and status = 'CLOSED'",
                )
                .bind(Uuid::from(tenant_id))
                .bind(next_period.trim())
                .fetch_one(&mut *tx)
                .await?;
                if next_closed != 0 {
                    anyhow::bail!("CONFLICT: next period is already closed");
                }
                let original: Option<(Uuid, i64)> = sqlx::query_as(
                    "select entry_id, amount_cent from shared_item_costs where tenant_id = $1 and item_code = $2 and kind = 'RECEIPT' and period_code = $3 for update",
                )
                .bind(Uuid::from(tenant_id))
                .bind(item_code.trim())
                .bind(closed_period.trim())
                .fetch_optional(&mut *tx)
                .await?;
                let Some((original_id, original_cent)) = original else {
                    anyhow::bail!("closed period has no original cost");
                };
                let inside_refused = closed == 1 && next_period.trim() != closed_period.trim();
                if !inside_refused {
                    anyhow::bail!("closed period accepted an adjustment inside it");
                }
                let entry_id = Uuid::new_v4();
                sqlx::query(
                    r#"
                    insert into shared_item_costs(
                        tenant_id, entry_id, item_code, kind, qty_micros, amount_cent, unit_cent,
                        estimated, algorithm, source_ref, period_code
                    ) values ($1,$2,$3,'ADJUST',0,$4,0,false,'MOVING_AVERAGE_V1',$5,$6)
                    "#,
                )
                .bind(Uuid::from(tenant_id))
                .bind(entry_id)
                .bind(item_code.trim())
                .bind(amount_cent)
                .bind(original_id.to_string())
                .bind(next_period.trim())
                .execute(&mut *tx)
                .await?;
                let kept: i64 = sqlx::query_scalar(
                    "select amount_cent from shared_item_costs where tenant_id = $1 and entry_id = $2",
                )
                .bind(Uuid::from(tenant_id))
                .bind(original_id)
                .fetch_one(&mut *tx)
                .await?;
                if kept != original_cent {
                    anyhow::bail!("closed period adjustment rewrote the original cost");
                }
                insert_receipt(
                    &mut tx,
                    tenant_id,
                    &succeeded(request_id, payload_hash, &entry_id.to_string()),
                )
                .await?;
                tx.commit().await?;
                Ok(Ok((
                    SharedWrite::Stored(succeeded(request_id, payload_hash, &entry_id.to_string())),
                    entry_id,
                )))
            }
        }
    }

    /// Reads one closed-period adjustment. A rewritten original cost fails the read.
    pub async fn read_closed_period_adjustment(
        &self,
        tenant_id: TenantId,
        item_code: &str,
        closed_period: &str,
        next_period: &str,
    ) -> anyhow::Result<(i64, i64, String)> {
        let original: Option<(i64, i64)> = sqlx::query_as(
            "select amount_cent, count(*)::bigint from shared_item_costs where tenant_id = $1 and item_code = $2 and kind = 'RECEIPT' and period_code = $3 group by amount_cent",
        )
        .bind(Uuid::from(tenant_id))
        .bind(item_code.trim())
        .bind(closed_period.trim())
        .fetch_optional(self.pool)
        .await?;
        let (original_cent, original_rows) = original.unwrap_or((0, 0));
        let adjustment: Option<(i64, i64, String)> = sqlx::query_as(
            "select amount_cent, count(*)::bigint, period_code from shared_item_costs where tenant_id = $1 and item_code = $2 and kind = 'ADJUST' group by amount_cent, period_code",
        )
        .bind(Uuid::from(tenant_id))
        .bind(item_code.trim())
        .fetch_optional(self.pool)
        .await?;
        let (adjustment_cent, adjustment_rows, stored_period) =
            adjustment.unwrap_or((0, 0, String::new()));
        let original_cent = erp_domain::trade::closed_period_adjustment_keeps_original(
            original_cent,
            adjustment_cent,
            original_rows,
            adjustment_rows,
            stored_period.trim(),
            closed_period,
        )
        .map_err(|_| anyhow::anyhow!("closed period adjustment rewrote the original cost"))?;
        if stored_period.trim() != next_period.trim() {
            anyhow::bail!("closed period adjustment was not stored in the next period");
        }
        Ok((original_cent, adjustment_cent, stored_period))
    }

    /// Posts a late expense as a new adjustment. Closed periods refuse a document inside them.
    pub async fn post_cost_adjustment(
        &self,
        input: super::CostAdjustmentInput<'_>,
    ) -> anyhow::Result<Result<(SharedWrite, i64, bool), ReceiptError>> {
        let super::CostAdjustmentInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            amount_cent,
            period_code,
        } = input;
        self.post_cost_entry(CostEntryInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            kind: "ADJUST",
            qty_micros: 0,
            amount_cent,
            source_ref: "",
            period_code,
        })
        .await
    }

    /// Closes one inventory cost period. A later receipt inside that period is refused.
    pub async fn close_item_cost_period(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        period_code: &str,
    ) -> anyhow::Result<Result<SharedWrite, ReceiptError>> {
        if period_code.trim().is_empty() {
            anyhow::bail!("cost period is required");
        }
        let mut tx = self.pool.begin().await?;
        lock_cost_command(&mut tx, tenant_id, request_id).await?;
        lock_cost_period(&mut tx, tenant_id, period_code, true).await?;
        match admit_retry(
            load_receipt(&mut tx, tenant_id, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(existing)) => {
                tx.rollback().await?;
                Ok(Ok(SharedWrite::Replay(existing)))
            }
            Err(error) => {
                tx.rollback().await?;
                Ok(Err(error))
            }
            Ok(None) => {
                let closed: i64 = sqlx::query_scalar(
                    "select count(*) from shared_cost_periods where tenant_id = $1 and period_code = $2 and status = 'CLOSED'",
                )
                .bind(Uuid::from(tenant_id))
                .bind(period_code)
                .fetch_one(&mut *tx)
                .await?;
                if closed != 0 {
                    anyhow::bail!("cost period is already closed");
                }
                sqlx::query(
                    r#"
                    insert into shared_cost_periods(tenant_id, period_code, status)
                    values ($1, $2, 'CLOSED')
                    "#,
                )
                .bind(Uuid::from(tenant_id))
                .bind(period_code)
                .execute(&mut *tx)
                .await?;
                insert_receipt(
                    &mut tx,
                    tenant_id,
                    &succeeded(request_id, payload_hash, period_code),
                )
                .await?;
                tx.commit().await?;
                Ok(Ok(SharedWrite::Stored(succeeded(
                    request_id,
                    payload_hash,
                    period_code,
                ))))
            }
        }
    }

    async fn post_cost_entry(
        &self,
        input: CostEntryInput<'_>,
    ) -> anyhow::Result<Result<(SharedWrite, i64, bool), ReceiptError>> {
        let CostEntryInput {
            tenant_id,
            request_id,
            payload_hash,
            item_code,
            kind,
            qty_micros,
            amount_cent,
            source_ref,
            period_code,
        } = input;
        if item_code.trim().is_empty() || period_code.trim().is_empty() {
            anyhow::bail!("cost entry needs an item and a period");
        }
        let mut tx = self.pool.begin().await?;
        lock_cost_command(&mut tx, tenant_id, request_id).await?;
        lock_item_cost(&mut tx, tenant_id, item_code).await?;
        match admit_retry(
            load_receipt(&mut tx, tenant_id, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(existing)) => {
                let entry_id = Uuid::parse_str(&existing.effect_id)?;
                let stored: (String, String, i64, i64, bool, String, String) = sqlx::query_as(
                    "select item_code,kind,qty_micros,amount_cent,estimated,source_ref,period_code from shared_item_costs where tenant_id=$1 and entry_id=$2",
                ).bind(Uuid::from(tenant_id)).bind(entry_id).fetch_one(&mut *tx).await?;
                if stored.0 != item_code
                    || stored.1 != kind
                    || stored.2 != qty_micros
                    || stored.5 != source_ref
                    || stored.6 != period_code
                    || (matches!(kind, "RECEIPT" | "ADJUST") && stored.3 != amount_cent)
                {
                    anyhow::bail!("cost command payload conflict");
                }
                tx.rollback().await?;
                Ok(Ok((SharedWrite::Replay(existing), stored.3, stored.4)))
            }
            Err(error) => {
                tx.rollback().await?;
                Ok(Err(error))
            }
            Ok(None) => {
                lock_cost_period(&mut tx, tenant_id, period_code, false).await?;
                let closed: i64 = sqlx::query_scalar(
                    "select count(*) from shared_cost_periods where tenant_id = $1 and period_code = $2 and status = 'CLOSED'",
                )
                .bind(Uuid::from(tenant_id))
                .bind(period_code)
                .fetch_one(&mut *tx)
                .await?;
                if closed > 0 {
                    anyhow::bail!("closed cost period cannot accept a new document");
                }
                let rows = sqlx::query_as::<_, CostRow>(COST_ROWS_SQL)
                    .bind(Uuid::from(tenant_id))
                    .bind(item_code)
                    .fetch_all(&mut *tx)
                    .await?;
                let mut position = cost_position(&rows)?;
                let (on_hand, value, unit, estimated) = (
                    position.qty,
                    position.value,
                    position.unit,
                    position.estimated,
                );
                let (stored_qty, stored_amount, stored_unit, stored_estimated, stored_source) =
                    match kind {
                        "RECEIPT" => {
                            let (next_unit, _, next_estimated) =
                                erp_domain::trade::moving_average_unit(
                                    on_hand,
                                    value,
                                    qty_micros,
                                    amount_cent,
                                )
                                .map_err(|_| anyhow::anyhow!("moving average rejected"))?;
                            (
                                qty_micros,
                                amount_cent,
                                next_unit,
                                next_estimated,
                                source_ref.to_string(),
                            )
                        }
                        "ISSUE" => {
                            if qty_micros <= 0 || qty_micros > on_hand {
                                anyhow::bail!("issue exceeds the costed quantity");
                            }
                            // Allocate from the actual remaining value, including its cent tail.
                            let issue_amount =
                                erp_domain::trade::return_cost_cent(value, on_hand, qty_micros)
                                    .map_err(|_| anyhow::anyhow!("issue cost rejected"))?;
                            let issue_estimated = estimated;
                            (
                                qty_micros,
                                issue_amount,
                                unit,
                                issue_estimated,
                                source_ref.to_string(),
                            )
                        }
                        "RETURN" => {
                            let original = rows
                                .iter()
                                .find(|row| row.kind == "ISSUE" && row.source_ref == source_ref);
                            let Some(original) = original else {
                                anyhow::bail!("return needs the named original issue");
                            };
                            let (returned_qty, returned_cent) = rows
                                .iter()
                                .filter(|row| row.kind == "RETURN" && row.source_ref == source_ref)
                                .try_fold(
                                    (0_i64, 0_i64),
                                    |(qty, cents), row| -> anyhow::Result<_> {
                                        Ok((
                                            qty.checked_add(row.qty_micros).ok_or_else(|| {
                                                anyhow::anyhow!("return quantity overflow")
                                            })?,
                                            cents.checked_add(row.amount_cent).ok_or_else(
                                                || anyhow::anyhow!("return amount overflow"),
                                            )?,
                                        ))
                                    },
                                )?;
                            let amount = cumulative_return_cost(CostReturnInput {
                                original_qty: original.qty_micros,
                                original_cent: original.amount_cent,
                                returned_qty,
                                returned_cent,
                                new_qty: qty_micros,
                            })
                            .map_err(|_| {
                                anyhow::anyhow!(
                                    "return exceeds the original issue quantity or cost"
                                )
                            })?;
                            (
                                qty_micros,
                                amount,
                                original.unit_cent,
                                original.estimated,
                                source_ref.to_string(),
                            )
                        }
                        "ADJUST" => {
                            if amount_cent == 0 {
                                anyhow::bail!("cost adjustment cannot be zero");
                            }
                            let next_value = value
                                .checked_add(amount_cent)
                                .filter(|next| *next >= 0)
                                .ok_or_else(|| {
                                    anyhow::anyhow!("cost adjustment exceeds remaining value")
                                })?;
                            let next_unit = if on_hand > 0 && !estimated {
                                cost_unit_and_residual(next_value, on_hand)
                                    .map_err(|_| anyhow::anyhow!("cost unit overflow"))?
                                    .0
                            } else {
                                unit
                            };
                            (0, amount_cent, next_unit, estimated, source_ref.to_string())
                        }
                        _ => anyhow::bail!("unknown cost kind"),
                    };
                let entry_id = Uuid::new_v4();
                position.apply(&CostRow {
                    kind: kind.to_string(),
                    qty_micros: stored_qty,
                    amount_cent: stored_amount,
                    unit_cent: stored_unit,
                    estimated: stored_estimated,
                    source_ref: stored_source.clone(),
                })?;
                sqlx::query(
                    r#"
                    insert into shared_item_costs(
                        tenant_id, entry_id, item_code, kind, qty_micros, amount_cent, unit_cent,
                        estimated, algorithm, source_ref, period_code
                    ) values ($1,$2,$3,$4,$5,$6,$7,$8,'MOVING_AVERAGE_V1',$9,$10)
                    "#,
                )
                .bind(Uuid::from(tenant_id))
                .bind(entry_id)
                .bind(item_code)
                .bind(kind)
                .bind(stored_qty)
                .bind(stored_amount)
                .bind(stored_unit)
                .bind(stored_estimated)
                .bind(&stored_source)
                .bind(period_code)
                .execute(&mut *tx)
                .await?;
                insert_receipt(
                    &mut tx,
                    tenant_id,
                    &succeeded(request_id, payload_hash, &entry_id.to_string()),
                )
                .await?;
                tx.commit().await?;
                Ok(Ok((
                    SharedWrite::Stored(succeeded(request_id, payload_hash, &entry_id.to_string())),
                    stored_amount,
                    stored_estimated,
                )))
            }
        }
    }

    /// Rebuilds one item cost from its own rows and stores the comparison. It does not rewrite those rows.
    pub async fn rebuild_item_cost(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        item_code: &str,
    ) -> anyhow::Result<Result<SharedWrite, ReceiptError>> {
        if item_code.trim().is_empty() {
            anyhow::bail!("cost rebuild needs an item");
        }
        let mut tx = self.pool.begin().await?;
        lock_cost_command(&mut tx, tenant_id, request_id).await?;
        lock_item_cost(&mut tx, tenant_id, item_code).await?;
        match admit_retry(
            load_receipt(&mut tx, tenant_id, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(existing)) => {
                tx.rollback().await?;
                Ok(Ok(SharedWrite::Replay(existing)))
            }
            Err(error) => {
                tx.rollback().await?;
                Ok(Err(error))
            }
            Ok(None) => {
                let rows = sqlx::query_as::<_, CostRow>(COST_ROWS_SQL)
                    .bind(Uuid::from(tenant_id))
                    .bind(item_code)
                    .fetch_all(&mut *tx)
                    .await?;
                if rows.is_empty() {
                    anyhow::bail!("cost rebuild needs stored rows");
                }
                let rebuilt: i64 = sqlx::query_scalar(
                    "select count(*) from shared_cost_rebuilds where tenant_id = $1 and item_code = $2",
                )
                .bind(Uuid::from(tenant_id))
                .bind(item_code)
                .fetch_one(&mut *tx)
                .await?;
                if rebuilt != 0 {
                    anyhow::bail!("this item cost is already rebuilt");
                }
                let position = cost_position(&rows)?;
                let (qty, value, estimated) = (
                    position.qty,
                    position.value,
                    position.estimated || position.qty == 0,
                );
                let (unit, residual) = if qty > 0 && !estimated {
                    cost_unit_and_residual(value, qty)
                        .map_err(|_| anyhow::anyhow!("cost unit overflow"))?
                } else {
                    (0, 0)
                };
                let decision = admit_cost_rebuild(CostRebuildInput {
                    stored_qty: qty,
                    stored_value: value,
                    stored_unit: unit,
                    rebuilt_qty: qty,
                    rebuilt_value: value,
                    rebuilt_unit: unit,
                    residual_cent: residual,
                    estimated,
                })
                .map_err(|_| anyhow::anyhow!("cost rebuild does not match the stored position"))?;
                sqlx::query(
                    "insert into shared_cost_rebuilds(tenant_id, rebuild_id, item_code, qty_micros, value_cent, unit_cent, residual_cent, estimated, decision) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                )
                .bind(Uuid::from(tenant_id))
                .bind(Uuid::new_v4())
                .bind(item_code)
                .bind(qty)
                .bind(value)
                .bind(unit)
                .bind(residual)
                .bind(estimated)
                .bind(decision)
                .execute(&mut *tx)
                .await?;
                insert_receipt(
                    &mut tx,
                    tenant_id,
                    &succeeded(request_id, payload_hash, decision),
                )
                .await?;
                tx.commit().await?;
                Ok(Ok(SharedWrite::Stored(succeeded(
                    request_id,
                    payload_hash,
                    decision,
                ))))
            }
        }
    }

    /// Posts an unknown receipt as estimated. It does not invent a unit cost.
    pub async fn post_estimated_cost(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        item_code: &str,
        qty_micros: i64,
        period_code: &str,
    ) -> anyhow::Result<Result<(SharedWrite, Uuid), ReceiptError>> {
        if item_code.trim().is_empty() || period_code.trim().is_empty() || qty_micros <= 0 {
            anyhow::bail!("estimated cost needs an item, a period, and a positive quantity");
        }
        let mut tx = self.pool.begin().await?;
        lock_cost_command(&mut tx, tenant_id, request_id).await?;
        lock_item_cost(&mut tx, tenant_id, item_code).await?;
        match admit_retry(
            load_receipt(&mut tx, tenant_id, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(existing)) => {
                let entry_id = Uuid::parse_str(&existing.effect_id)?;
                tx.rollback().await?;
                Ok(Ok((SharedWrite::Replay(existing), entry_id)))
            }
            Err(error) => {
                tx.rollback().await?;
                Ok(Err(error))
            }
            Ok(None) => {
                lock_cost_period(&mut tx, tenant_id, period_code, false).await?;
                let rows = sqlx::query_as::<_, CostRow>(COST_ROWS_SQL)
                    .bind(Uuid::from(tenant_id))
                    .bind(item_code)
                    .fetch_all(&mut *tx)
                    .await?;
                let mut position = cost_position(&rows)?;
                position.apply(&CostRow {
                    kind: "RECEIPT".to_string(),
                    qty_micros,
                    amount_cent: 0,
                    unit_cent: 0,
                    estimated: true,
                    source_ref: "UNKNOWN".to_string(),
                })?;
                let closed: i64 = sqlx::query_scalar(
                    "select count(*) from shared_cost_periods where tenant_id = $1 and period_code = $2 and status = 'CLOSED'",
                )
                .bind(Uuid::from(tenant_id))
                .bind(period_code)
                .fetch_one(&mut *tx)
                .await?;
                if closed > 0 {
                    anyhow::bail!("closed cost period cannot accept an estimate");
                }
                let entry_id = Uuid::new_v4();
                sqlx::query(
                    "insert into shared_item_costs(tenant_id, entry_id, item_code, kind, qty_micros, amount_cent, unit_cent, estimated, algorithm, source_ref, period_code) values ($1,$2,$3,'RECEIPT',$4,0,0,true,'MOVING_AVERAGE_V1','UNKNOWN',$5)",
                )
                .bind(Uuid::from(tenant_id))
                .bind(entry_id)
                .bind(item_code)
                .bind(qty_micros)
                .bind(period_code)
                .execute(&mut *tx)
                .await?;
                insert_receipt(
                    &mut tx,
                    tenant_id,
                    &succeeded(request_id, payload_hash, &entry_id.to_string()),
                )
                .await?;
                tx.commit().await?;
                Ok(Ok((
                    SharedWrite::Stored(succeeded(request_id, payload_hash, &entry_id.to_string())),
                    entry_id,
                )))
            }
        }
    }
}

const COST_ROWS_SQL: &str = "select kind,qty_micros,amount_cent,unit_cent,estimated,source_ref from shared_item_costs where tenant_id=$1 and item_code=$2 order by entry_sequence";

/// Freeze the SKU's actual cost-book position under its existing writer lock.
pub(super) async fn material_price(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    sku: Uuid,
) -> anyhow::Result<serde_json::Value> {
    let period = chrono::Utc::now().format("%Y-%m").to_string();
    lock_cost_period(tx, tenant, &period, false).await?;
    let closed:bool=sqlx::query_scalar("select exists(select 1 from shared_cost_periods where tenant_id=$1 and period_code=$2 and status='CLOSED')")
        .bind(tenant.0).bind(&period).fetch_one(&mut **tx).await?;
    if closed {
        return Err(
            super::MaterialCostError::Reconcile("current material cost period is closed").into(),
        );
    }
    let code: Option<String> =
        sqlx::query_scalar("select sku_code from skus where tenant_id=$1 and sku_id=$2 for share")
            .bind(tenant.0)
            .bind(sku)
            .fetch_optional(&mut **tx)
            .await?;
    let Some(code) = code else {
        return Ok(serde_json::json!({"known":false,"reason":"SKU cost identity missing"}));
    };
    lock_item_cost(tx, tenant, &code).await?;
    let rows = sqlx::query_as::<_, CostRow>(&format!("{COST_ROWS_SQL} limit 10001"))
        .bind(tenant.0)
        .bind(&code)
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() > 10000 {
        return Err(super::MaterialCostError::Reconcile("SKU cost history budget exceeded").into());
    }
    let position = cost_position(&rows)?;
    let sequence: Option<i64> = sqlx::query_scalar(
        "select max(entry_sequence) from shared_item_costs where tenant_id=$1 and item_code=$2",
    )
    .bind(tenant.0)
    .bind(&code)
    .fetch_one(&mut **tx)
    .await?;
    let known =
        !position.estimated && position.qty > 0 && position.value >= 0 && position.unit >= 0;
    Ok(
        serde_json::json!({"known":known,"sku_id":sku,"sku_code":code,"entry_sequence":sequence,"period_code":period,
        "unit_cent":if known {Some(position.unit)}else{None},"position_qty_micros":position.qty,
        "position_value_cent":position.value,"estimated":position.estimated,"method":"MOVING_AVERAGE_V1"}),
    )
}

pub(super) async fn material_book_issue(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    issue: Uuid,
    qty: i64,
    price: &serde_json::Value,
) -> anyhow::Result<Option<i64>> {
    if price["known"] != true {
        return Ok(None);
    }
    let available = price["position_qty_micros"]
        .as_i64()
        .ok_or(super::MaterialCostError::Cost)?;
    if qty > available {
        return Ok(None);
    }
    let value = price["position_value_cent"]
        .as_i64()
        .ok_or(super::MaterialCostError::Cost)?;
    let cent = erp_domain::trade::return_cost_cent(value, available, qty)
        .map_err(|_| super::MaterialCostError::Cost)?;
    sqlx::query("insert into shared_item_costs(tenant_id,entry_id,item_code,kind,qty_micros,amount_cent,unit_cent,estimated,algorithm,source_ref,period_code) values($1,$2,$3,'ISSUE',$4,$5,$6,false,'MOVING_AVERAGE_V1',$7,$8)")
        .bind(tenant.0).bind(Uuid::new_v4()).bind(price["sku_code"].as_str()).bind(qty).bind(cent).bind(price["unit_cent"].as_i64())
        .bind(format!("MFG:{issue}")).bind(price["period_code"].as_str()).execute(&mut **tx).await?;
    Ok(Some(cent))
}

pub(super) async fn material_book_return(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    issue: Uuid,
    qty: i64,
    cent: i64,
    price: &serde_json::Value,
) -> anyhow::Result<()> {
    material_return_lock(tx, tenant, price).await?;
    let period = chrono::Utc::now().format("%Y-%m").to_string();
    let code = price["sku_code"]
        .as_str()
        .ok_or(super::MaterialCostError::Cost)?;
    sqlx::query("insert into shared_item_costs(tenant_id,entry_id,item_code,kind,qty_micros,amount_cent,unit_cent,estimated,algorithm,source_ref,period_code) values($1,$2,$3,'RETURN',$4,$5,$6,false,'MOVING_AVERAGE_V1',$7,$8)")
        .bind(tenant.0).bind(Uuid::new_v4()).bind(code).bind(qty).bind(cent).bind(price["unit_cent"].as_i64())
        .bind(format!("MFG:{issue}")).bind(period).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn material_return_lock(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    price: &serde_json::Value,
) -> anyhow::Result<()> {
    let period = chrono::Utc::now().format("%Y-%m").to_string();
    lock_cost_period(tx, tenant, &period, false).await?;
    let closed:bool=sqlx::query_scalar("select exists(select 1 from shared_cost_periods where tenant_id=$1 and period_code=$2 and status='CLOSED')")
        .bind(tenant.0).bind(&period).fetch_one(&mut **tx).await?;
    if closed {
        return Err(super::MaterialCostError::Reconcile(
            "current material return cost period is closed",
        )
        .into());
    }
    let code = price["sku_code"]
        .as_str()
        .ok_or(super::MaterialCostError::Cost)?;
    lock_item_cost(tx, tenant, code).await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct CostRow {
    kind: String,
    qty_micros: i64,
    amount_cent: i64,
    unit_cent: i64,
    estimated: bool,
    source_ref: String,
}

struct CostPosition {
    qty: i64,
    value: i64,
    unit: i64,
    estimated: bool,
}

fn cost_position(rows: &[CostRow]) -> anyhow::Result<CostPosition> {
    let mut position = CostPosition {
        qty: 0,
        value: 0,
        unit: 0,
        estimated: false,
    };
    for row in rows {
        position.apply(row)?;
    }
    if position.qty > 0 {
        position.unit = cost_unit_and_residual(position.value, position.qty)
            .map_err(|_| anyhow::anyhow!("cost unit overflow"))?
            .0;
    }
    Ok(position)
}

impl CostPosition {
    fn apply(&mut self, row: &CostRow) -> anyhow::Result<()> {
        match row.kind.as_str() {
            "RECEIPT" | "RETURN" => {
                if row.qty_micros <= 0 || row.amount_cent < 0 {
                    anyhow::bail!("invalid cost receipt quantity or value");
                }
                self.qty = self
                    .qty
                    .checked_add(row.qty_micros)
                    .ok_or_else(|| anyhow::anyhow!("cost quantity overflow"))?;
                self.value = self
                    .value
                    .checked_add(row.amount_cent)
                    .ok_or_else(|| anyhow::anyhow!("cost value overflow"))?;
                self.estimated |= row.estimated;
            }
            "ISSUE" => {
                if row.qty_micros <= 0
                    || row.qty_micros > self.qty
                    || row.amount_cent < 0
                    || row.amount_cent > self.value
                {
                    anyhow::bail!("cost history contains an invalid issue");
                }
                self.qty -= row.qty_micros;
                self.value -= row.amount_cent;
                self.estimated |= row.estimated;
            }
            "ADJUST" => {
                self.value = self
                    .value
                    .checked_add(row.amount_cent)
                    .filter(|next| *next >= 0)
                    .ok_or_else(|| {
                        anyhow::anyhow!("cost history value overflow or negative value")
                    })?;
            }
            _ => anyhow::bail!("unknown cost kind"),
        }
        if self.qty == 0 && self.value == 0 {
            self.estimated = false;
        }
        if self.qty == 0 && self.value != 0 {
            anyhow::bail!("cost history has value without stock; reconcile the original entries");
        }
        Ok(())
    }
}

async fn lock_cost_command(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    request: &str,
) -> anyhow::Result<()> {
    if request.trim().is_empty() {
        anyhow::bail!("cost command needs a request identity");
    }
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(Uuid::from(tenant).to_string())
        .bind(format!("cost-command:{request}"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn lock_item_cost(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    item: &str,
) -> anyhow::Result<()> {
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(Uuid::from(tenant).to_string())
        .bind(format!("item-cost:{item}").trim())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn lock_cost_period(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    period: &str,
    exclusive: bool,
) -> anyhow::Result<()> {
    let sql = if exclusive {
        "select pg_advisory_xact_lock(hashtext($1),hashtext($2))"
    } else {
        "select pg_advisory_xact_lock_shared(hashtext($1),hashtext($2))"
    };
    sqlx::query(sql)
        .bind(Uuid::from(tenant).to_string())
        .bind(format!("cost-period:{period}"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}
