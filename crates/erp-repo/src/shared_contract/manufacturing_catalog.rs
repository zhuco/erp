//! Real business identifiers and bounded material context for the shared UI.
use super::{MaterialDocumentError, ReceiptValueActor, SharedContractRepo, receipt_authority};
use erp_types::TenantId;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
pub struct WorkOrderOption {
    pub work_order_id: Uuid,
    pub work_order_no: String,
    pub product_code: String,
    pub status: String,
    pub bom_id: Option<Uuid>,
    pub order_qty_micros: String,
    pub issued_micros: String,
    pub consumed_micros: String,
}

pub struct WorkOrderOptionQuery<'a> {
    pub text: &'a str,
    pub status: Option<&'a str>,
    pub open_only: bool,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct Component {
    line_no: i32,
    parent_code: String,
    component_code: String,
    qty_per_micros: String,
    unit_code: String,
    scrap_ppm: i64,
    supply_kind: String,
    sku_id: Option<Uuid>,
    sku_name: Option<String>,
    direct_component: bool,
}
#[derive(Serialize, sqlx::FromRow)]
struct WarehouseOption {
    warehouse_id: Uuid,
    code: Option<String>,
    name: String,
}

impl SharedContractRepo<'_> {
    pub async fn list_work_order_options(
        &self,
        tenant: TenantId,
        input: WorkOrderOptionQuery<'_>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<(i64, Vec<WorkOrderOption>)> {
        let text = input.text.trim();
        if text.len() > 128
            || text.chars().any(char::is_control)
            || !(0..=1000000).contains(&input.offset)
            || !(1..=2000).contains(&input.limit)
            || input.status.is_some_and(|s| {
                !matches!(
                    s,
                    "PLANNED" | "RELEASED" | "PAUSED" | "COMPLETED" | "CLOSED" | "CANCELLED"
                )
            })
        {
            anyhow::bail!("invalid work order option query");
        }
        let pattern = format!(
            "%{}%",
            text.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read")
            .execute(&mut *tx)
            .await?;
        let permit = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let total:i64=sqlx::query_scalar("select count(*) from mfg_work_orders where tenant_id=$1 and (work_order_no ilike $2 escape '\\' or product_code ilike $2 escape '\\') and ($3::text is null or status=$3) and (not $4 or status in('PLANNED','RELEASED','PAUSED'))")
            .bind(tenant.0).bind(&pattern).bind(input.status).bind(input.open_only).fetch_one(&mut *tx).await?;
        let data:Vec<WorkOrderOption>=sqlx::query_as("select work_order_id,work_order_no,product_code,status,bom_id,order_qty_micros::text,issued_micros::text,consumed_micros::text from mfg_work_orders where tenant_id=$1 and (work_order_no ilike $2 escape '\\' or product_code ilike $2 escape '\\') and ($3::text is null or status=$3) and (not $4 or status in('PLANNED','RELEASED','PAUSED')) order by work_order_id offset $5 limit $6")
            .bind(tenant.0).bind(pattern).bind(input.status).bind(input.open_only).bind(input.offset).bind(input.limit).fetch_all(&mut *tx).await?;
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok((total, data))
    }

    pub async fn read_work_order_material_context(
        &self,
        tenant: TenantId,
        order: Uuid,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read")
            .execute(&mut *tx)
            .await?;
        let permit = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let row:Option<WorkOrderOption>=sqlx::query_as("select work_order_id,work_order_no,product_code,status,bom_id,order_qty_micros::text,issued_micros::text,consumed_micros::text from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for share")
            .bind(tenant.0).bind(order).fetch_optional(&mut *tx).await?;
        let row = row.ok_or(MaterialDocumentError::NotFound)?;
        let version: Option<(i32, String)> = if let Some(id) = row.bom_id {
            sqlx::query_as("select version_no,status from shared_mfg_bom_versions where tenant_id=$1 and version_id=$2 for share")
                .bind(tenant.0).bind(id).fetch_optional(&mut *tx).await?
        } else {
            None
        };
        let materials: Vec<Component> = if let Some(id) = row.bom_id {
            sqlx::query_as("select l.line_no,l.parent_code,l.component_code,l.qty_per_micros::text,l.unit_code,l.scrap_ppm,l.supply_kind,s.sku_id,s.name as sku_name,l.parent_code=$3 as direct_component from shared_mfg_bom_version_lines l left join skus s on s.tenant_id=l.tenant_id and s.sku_code=l.component_code and s.is_active where l.tenant_id=$1 and l.version_id=$2 order by l.line_no limit 1001 for share of l")
                .bind(tenant.0).bind(id).bind(&row.product_code).fetch_all(&mut *tx).await?
        } else {
            Vec::new()
        };
        if materials.len() > 1000 {
            anyhow::bail!("work order material context exceeds its complete 1000-line budget");
        }
        let warehouses:Vec<WarehouseOption>=sqlx::query_as("select a.warehouse_id,w.code,w.name from user_warehouse_access a join warehouses w using(tenant_id,warehouse_id) where a.tenant_id=$1 and a.user_id=$2 order by a.warehouse_id limit 1001 for share of a,w")
            .bind(tenant.0).bind(actor.user_id.0).fetch_all(&mut *tx).await?;
        if warehouses.len() > 1000 {
            anyhow::bail!("warehouse option context exceeds its complete 1000-row budget");
        }
        let published = version.as_ref().is_some_and(|(v, status)| {
            *v > 0 && matches!(status.as_str(), "PUBLISHED" | "RETIRED")
        });
        let needs_reconciliation = !published
            || materials.is_empty()
            || materials
                .iter()
                .any(|m| m.sku_id.is_none() || m.unit_code.is_empty() || !m.direct_component);
        let result = json!({"work_order":row,"bom_version":version.map(|(version_no,status)|json!({"version_no":version_no,"status":status})),
            "materials":materials,"warehouses":warehouses,"needs_reconciliation":needs_reconciliation});
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok(result)
    }
}
