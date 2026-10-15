//! Manufacturing BOM structure storage and bounded graph expansion.

use erp_domain::manufacturing::admit_bom_page;
use erp_domain::receipt::{ReceiptError, admit_retry};
use erp_types::TenantId;
use sqlx::Postgres;
use uuid::Uuid;

use super::{SharedContractRepo, SharedWrite, insert_receipt, load_receipt, succeeded};

impl SharedContractRepo<'_> {
    /// This is not a trade kit. Publishing stays a later version step.
    pub async fn save_bom_structure(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        product_code: &str,
        lines: &[erp_domain::manufacturing::BomLine],
    ) -> anyhow::Result<Result<(SharedWrite, Uuid), ReceiptError>> {
        if product_code.trim().is_empty()
            || lines.first().is_none_or(|line| line.parent != product_code)
        {
            anyhow::bail!("bom structure needs a product parent");
        }
        erp_domain::manufacturing::admit_bom_structure(lines)
            .map_err(|error| anyhow::anyhow!("bom structure rejected: {error:?}"))?;
        let mut tx = self.pool.begin().await?;
        let lock_key = format!("bom-structure:{product_code}");
        sqlx::query("select pg_advisory_xact_lock(hashtext($1::text), hashtext($2))")
            .bind(Uuid::from(tenant_id).to_string())
            .bind(lock_key.trim())
            .execute(&mut *tx)
            .await?;
        match admit_retry(
            load_receipt(&mut tx, tenant_id, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(existing)) => {
                let bom_id = Uuid::parse_str(&existing.effect_id)?;
                tx.rollback().await?;
                Ok(Ok((SharedWrite::Replay(existing), bom_id)))
            }
            Err(error) => {
                tx.rollback().await?;
                Ok(Err(error))
            }
            Ok(None) => {
                let already: i64 = sqlx::query_scalar(
                    "select count(distinct bom_id)::bigint from shared_mfg_bom_lines where tenant_id = $1 and parent_code = $2 and line_no = 1",
                )
                .bind(Uuid::from(tenant_id))
                .bind(product_code)
                .fetch_one(&mut *tx)
                .await?;
                if already != 0 {
                    anyhow::bail!("this product already has a draft structure");
                }
                let bom_id = Uuid::new_v4();
                for chunk in lines.chunks(500) {
                    let mut insert = sqlx::QueryBuilder::<Postgres>::new(
                        "insert into shared_mfg_bom_lines(tenant_id, bom_id, line_no, parent_code, component_code, qty_per_micros, unit_code, scrap_ppm, supply_kind) ",
                    );
                    insert.push_values(chunk, |mut row, line| {
                        row.push_bind(Uuid::from(tenant_id))
                            .push_bind(bom_id)
                            .push_bind(line.line_no)
                            .push_bind(&line.parent)
                            .push_bind(&line.component)
                            .push_bind(line.qty_per.as_micros())
                            .push_bind(&line.unit_code)
                            .push_bind(line.scrap_ppm)
                            .push_bind(&line.supply);
                    });
                    insert.build().execute(&mut *tx).await?;
                }
                insert_receipt(
                    &mut tx,
                    tenant_id,
                    &succeeded(request_id, payload_hash, &bom_id.to_string()),
                )
                .await?;
                tx.commit().await?;
                Ok(Ok((
                    SharedWrite::Stored(succeeded(request_id, payload_hash, &bom_id.to_string())),
                    bom_id,
                )))
            }
        }
    }

    /// Explodes one saved structure. The stored rows are the independent recalculation source.
    pub async fn explode_bom_structure(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        bom_id: Uuid,
        order_qty_micros: i64,
    ) -> anyhow::Result<Result<(SharedWrite, Uuid, i64), ReceiptError>> {
        let mut tx = self.pool.begin().await?;
        let lock_key = format!("bom-explode:{bom_id}:{order_qty_micros}");
        sqlx::query("select pg_advisory_xact_lock(hashtext($1::text), hashtext($2))")
            .bind(Uuid::from(tenant_id).to_string())
            .bind(lock_key.trim())
            .execute(&mut *tx)
            .await?;
        match admit_retry(
            load_receipt(&mut tx, tenant_id, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(existing)) => {
                let explode_id = Uuid::parse_str(&existing.effect_id)?;
                let rows: i64 = sqlx::query_scalar(
                    "select count(*)::bigint from shared_mfg_bom_explodes where tenant_id = $1 and explode_id = $2",
                )
                .bind(Uuid::from(tenant_id))
                .bind(explode_id)
                .fetch_one(&mut *tx)
                .await?;
                tx.rollback().await?;
                Ok(Ok((SharedWrite::Replay(existing), explode_id, rows)))
            }
            Err(error) => {
                tx.rollback().await?;
                Ok(Err(error))
            }
            Ok(None) => {
                let stored: Vec<(i32, String, String, i64, String, i64, String)> = sqlx::query_as(
                    r#"
                    select line_no, parent_code, component_code, qty_per_micros, unit_code, scrap_ppm, supply_kind
                    from shared_mfg_bom_lines
                    where tenant_id = $1 and bom_id = $2
                    order by line_no limit 50001
                    "#,
                )
                .bind(Uuid::from(tenant_id))
                .bind(bom_id)
                .fetch_all(&mut *tx)
                .await?;
                if stored.len() > 50_000 {
                    anyhow::bail!("BOM edge budget exceeded");
                }
                if stored.is_empty() {
                    anyhow::bail!("bom structure is outside this tenant");
                }
                let already: i64 = sqlx::query_scalar(
                    "select count(*)::bigint from shared_mfg_bom_explodes where tenant_id = $1 and bom_id = $2",
                )
                .bind(Uuid::from(tenant_id))
                .bind(bom_id)
                .fetch_one(&mut *tx)
                .await?;
                if already != 0 {
                    anyhow::bail!("this bom structure is already exploded");
                }
                let components: std::collections::BTreeSet<&str> =
                    stored.iter().map(|row| row.2.as_str()).collect();
                let mut roots: Vec<&str> = stored
                    .iter()
                    .map(|row| row.1.as_str())
                    .filter(|parent| !components.contains(parent))
                    .collect();
                roots.sort_unstable();
                roots.dedup();
                if roots.len() != 1 {
                    anyhow::bail!("bom structure needs one root");
                }
                let root = roots[0].to_string();
                let lines: Vec<erp_domain::manufacturing::BomLine> = stored
                    .into_iter()
                    .map(|(line_no, parent, component, qty, unit, scrap, supply)| {
                        erp_domain::manufacturing::BomLine {
                            parent,
                            component,
                            qty_per: erp_domain::quantity::ScaledQty::from_micros(qty),
                            unit_code: unit,
                            scrap_ppm: scrap,
                            supply,
                            line_no,
                        }
                    })
                    .collect();
                let exploded = erp_domain::manufacturing::explode_bom(
                    &lines,
                    &root,
                    erp_domain::quantity::ScaledQty::from_micros(order_qty_micros),
                )
                .map_err(|error| anyhow::anyhow!("bom explode rejected: {error:?}"))?;
                let explode_id = Uuid::new_v4();
                let sequenced = exploded
                    .iter()
                    .enumerate()
                    .map(|(index, line)| i32::try_from(index + 1).map(|sequence| (sequence, line)))
                    .collect::<Result<Vec<_>, _>>()?;
                for chunk in sequenced.chunks(500) {
                    let mut insert = sqlx::QueryBuilder::<Postgres>::new(
                        "insert into shared_mfg_bom_explodes(tenant_id, explode_id, bom_id, parent_code, component_code, unit_code, qty_micros, line_no, depth, seq_no) ",
                    );
                    insert.push_values(chunk, |mut row, (sequence, line)| {
                        let (parent, component, unit, quantity, line_no, depth) = line;
                        row.push_bind(Uuid::from(tenant_id))
                            .push_bind(explode_id)
                            .push_bind(bom_id)
                            .push_bind(parent)
                            .push_bind(component)
                            .push_bind(unit)
                            .push_bind(quantity)
                            .push_bind(line_no)
                            .push_bind(depth)
                            .push_bind(sequence);
                    });
                    insert.build().execute(&mut *tx).await?;
                }
                insert_receipt(
                    &mut tx,
                    tenant_id,
                    &succeeded(request_id, payload_hash, &explode_id.to_string()),
                )
                .await?;
                tx.commit().await?;
                Ok(Ok((
                    SharedWrite::Stored(succeeded(
                        request_id,
                        payload_hash,
                        &explode_id.to_string(),
                    )),
                    explode_id,
                    exploded.len() as i64,
                )))
            }
        }
    }

    /// Reads one component quantity from a stored explode. Query order is component code, not line number.
    pub async fn read_bom_explode(
        &self,
        tenant_id: TenantId,
        explode_id: Uuid,
        component_code: &str,
    ) -> anyhow::Result<(i64, String, i32, i32)> {
        let row: (i64, String, i32, i32) = sqlx::query_as(
            r#"
            select qty_micros, unit_code, line_no, depth
            from shared_mfg_bom_explodes
            where tenant_id = $1 and explode_id = $2 and component_code = $3
            order by component_code, line_no
            limit 1
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(explode_id)
        .bind(component_code)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    /// Reads one parent page of a stored explode. Yield is recorded beside the scrap quantity and does not change it.
    pub async fn read_bom_explode_page(
        &self,
        tenant_id: TenantId,
        explode_id: Uuid,
        parent_code: &str,
        offset: i64,
        limit: i64,
        yield_ppm: i64,
    ) -> anyhow::Result<(i64, Vec<(String, i64, i32)>)> {
        if parent_code.trim().is_empty()
            || offset < 0
            || !(1..=50).contains(&limit)
            || !(0..=1_000_000).contains(&yield_ppm)
        {
            anyhow::bail!("BOM page parameters are invalid");
        }
        let total: i64 = sqlx::query_scalar(
            "select count(*)::bigint from shared_mfg_bom_explodes where tenant_id = $1 and explode_id = $2 and parent_code = $3",
        )
        .bind(Uuid::from(tenant_id))
        .bind(explode_id)
        .bind(parent_code)
        .fetch_one(self.pool)
        .await?;
        let rows: Vec<(String, i64, i32)> = sqlx::query_as(
            r#"
            select component_code, qty_micros, line_no
            from shared_mfg_bom_explodes
            where tenant_id = $1 and explode_id = $2 and parent_code = $3
            order by component_code
            offset $4 limit $5
            "#,
        )
        .bind(Uuid::from(tenant_id))
        .bind(explode_id)
        .bind(parent_code)
        .bind(offset)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        let qty = rows.first().map(|row| row.1).unwrap_or(0);
        admit_bom_page(erp_domain::manufacturing::BomPageInput {
            parent: parent_code,
            offset,
            limit,
            loaded: i64::try_from(rows.len()).unwrap_or(0),
            child_count: total,
            yield_ppm,
            qty_with_scrap: qty,
            qty_with_yield: qty,
        })
        .map_err(|_| anyhow::anyhow!("bom page is outside the stored explode"))?;
        Ok((total, rows))
    }
}
