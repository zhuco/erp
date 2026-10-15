//! Owned inspection sources, frozen copies and the locks shared by their consumers.
use erp_domain::manufacturing::FrozenInspectionLine;
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

const MAX_LINES: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SourceKind {
    WorkOrder,
    ProductionReceipt,
    SalesOrder,
}

impl SourceKind {
    pub(super) fn parse(value: &str) -> anyhow::Result<Self> {
        if value.len() > 32 {
            anyhow::bail!("inspection source kind exceeds its byte budget");
        }
        match value.trim().to_ascii_uppercase().as_str() {
            "WORK_ORDER" => Ok(Self::WorkOrder),
            "PRODUCTION_RECEIPT" | "RECEIPT" => Ok(Self::ProductionReceipt),
            "SALES_ORDER" => Ok(Self::SalesOrder),
            _ => anyhow::bail!("inspection source kind is not supported"),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::WorkOrder => "WORK_ORDER",
            Self::ProductionReceipt => "PRODUCTION_RECEIPT",
            Self::SalesOrder => "SALES_ORDER",
        }
    }
}

pub(super) struct LockedSource {
    pub(super) kind: SourceKind,
    pub(super) source_id: Uuid,
    pub(super) work_order: Option<Uuid>,
    receipt: Option<Uuid>,
    sales_revision: Option<Uuid>,
    product: Option<String>,
    batch: Option<String>,
    facts: Value,
}

impl LockedSource {
    pub(super) async fn require_item(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        item: &str,
        batch: &str,
    ) -> anyhow::Result<()> {
        if item.is_empty() || item.len() > 1024 || batch.len() > 512 {
            anyhow::bail!("inspection source item or batch exceeds its byte budget");
        }
        if self.product.as_deref().is_some_and(|actual| actual != item) {
            anyhow::bail!("inspection item is not the source product");
        }
        if self
            .batch
            .as_deref()
            .is_some_and(|actual| !batch.is_empty() && actual != batch)
        {
            anyhow::bail!("inspection batch is not the source receipt batch");
        }
        if let Some(revision) = self.sales_revision {
            let present: bool = sqlx::query_scalar(
                "select exists(select 1 from shared_sales_lines where tenant_id=$1 and revision_id=$2 and item_code=$3 and qty_micros>0)")
                .bind(tenant.0).bind(revision).bind(item).fetch_one(&mut **tx).await?;
            if !present {
                anyhow::bail!("inspection item is outside the owned sales revision");
            }
        }
        Ok(())
    }
}

async fn work_order(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    id: Uuid,
) -> anyhow::Result<(String, Value)> {
    let row: Option<(Option<String>, String, i64)> = sqlx::query_as(
        "select case when octet_length(product_code)<=1024 then product_code end,status,order_qty_micros from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(tenant.0).bind(id).fetch_optional(&mut **tx).await?;
    let Some((Some(product), status, quantity)) = row else {
        anyhow::bail!(
            "inspection work order is outside this enterprise or exceeds its source budget"
        );
    };
    let facts = json!({"tenant_id":tenant,"work_order_id":id,"product_code":product,
        "status":status,"order_qty_micros":quantity.to_string()});
    Ok((product, facts))
}

pub(super) async fn lock_source(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    kind: &str,
    source: Uuid,
) -> anyhow::Result<LockedSource> {
    if tenant.0.is_nil() || source.is_nil() {
        anyhow::bail!("inspection source identity must be nonzero");
    }
    let kind = SourceKind::parse(kind)?;
    match kind {
        SourceKind::WorkOrder => {
            let (product, facts) = work_order(tx, tenant, source).await?;
            Ok(LockedSource {
                kind,
                source_id: source,
                work_order: Some(source),
                receipt: None,
                sales_revision: None,
                product: Some(product),
                batch: None,
                facts,
            })
        }
        SourceKind::ProductionReceipt => {
            let owner: Option<Uuid> = sqlx::query_scalar(
                "select work_order_id from mfg_receipts where tenant_id=$1 and receipt_id=$2",
            )
            .bind(tenant.0)
            .bind(source)
            .fetch_optional(&mut **tx)
            .await?;
            let owner = owner
                .ok_or_else(|| anyhow::anyhow!("inspection receipt is outside this enterprise"))?;
            // Match release order: owner first, then receipt. This also protects absence of a new freeze.
            let (product, order_facts) = work_order(tx, tenant, owner).await?;
            let row: Option<(Uuid, Option<String>, String, i64)> = sqlx::query_as(
                "select work_order_id,case when octet_length(batch_code)<=512 then batch_code end,status,qty_micros from mfg_receipts where tenant_id=$1 and receipt_id=$2 for share")
                .bind(tenant.0).bind(source).fetch_optional(&mut **tx).await?;
            let Some((actual_owner, Some(batch), status, quantity)) = row else {
                anyhow::bail!("inspection receipt has an unknown original batch");
            };
            if owner != actual_owner || quantity <= 0 {
                anyhow::bail!("inspection receipt owner or quantity changed");
            }
            Ok(LockedSource {
                kind,
                source_id: source,
                work_order: Some(owner),
                receipt: Some(source),
                sales_revision: None,
                product: Some(product),
                batch: Some(batch.clone()),
                facts: json!({"work_order":order_facts,"receipt":{"tenant_id":tenant,"receipt_id":source,
                    "work_order_id":owner,"batch_code":batch,"status":status,"qty_micros":quantity.to_string()}}),
            })
        }
        SourceKind::SalesOrder => {
            sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
                .bind(tenant.0.to_string())
                .bind(format!("sales-revision:{source}"))
                .execute(&mut **tx)
                .await?;
            let row: Option<(Uuid,i32,String)> = sqlx::query_as(
                "select revision_id,version_no,status from shared_sales_revisions where tenant_id=$1 and so_id=$2 order by version_no desc limit 1 for share")
                .bind(tenant.0).bind(source).fetch_optional(&mut **tx).await?;
            let Some((revision, version, status)) = row else {
                anyhow::bail!("inspection sales order is outside this enterprise");
            };
            Ok(LockedSource {
                kind,
                source_id: source,
                work_order: None,
                receipt: None,
                sales_revision: Some(revision),
                product: None,
                batch: None,
                facts: json!({"tenant_id":tenant,"so_id":source,"revision_id":revision,
                    "version_no":version,"status":status}),
            })
        }
    }
}

pub(super) async fn insert_frozen_lines_and_binding(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    freeze: Uuid,
    source: &LockedSource,
    lines: &[FrozenInspectionLine],
    request_id: &str,
    payload_hash: &str,
) -> anyhow::Result<()> {
    if lines.is_empty()
        || lines.len() > MAX_LINES
        || lines
            .iter()
            .any(|line| line.item_code.is_empty() || line.item_code.len() > 1024)
    {
        anyhow::bail!("inspection frozen copy must contain 1 to 1000 bounded items");
    }
    let mut query = QueryBuilder::<Postgres>::new(
        "insert into shared_inspection_freeze_lines(tenant_id,freeze_id,item_code,required,min_micros,max_micros,sample_count) ",
    );
    query.push_values(lines, |mut row, line| {
        row.push_bind(tenant.0)
            .push_bind(freeze)
            .push_bind(&line.item_code)
            .push_bind(line.required)
            .push_bind(line.min_micros)
            .push_bind(line.max_micros)
            .push_bind(line.sample_count);
    });
    if query.build().execute(&mut **tx).await?.rows_affected() != lines.len() as u64 {
        anyhow::bail!("inspection frozen copy was not fully stored");
    }
    let row: (Uuid,i32,String,String) = sqlx::query_as(
        "select template_id,version_no,item_code,batch_code from shared_inspection_freezes where tenant_id=$1 and freeze_id=$2")
        .bind(tenant.0).bind(freeze).fetch_one(&mut **tx).await?;
    let frozen: Vec<Value> = lines
        .iter()
        .map(|line| {
            json!({"item_code":line.item_code,"required":line.required,
        "min_micros":line.min_micros,"max_micros":line.max_micros,"sample_count":line.sample_count})
        })
        .collect();
    let snapshot = json!({"tenant_id":tenant,"source_kind":source.kind.as_str(),"source_id":source.source_id,
        "item_code":row.2,"batch_code":row.3,"facts":source.facts});
    let count = i32::try_from(lines.len())?;
    if sqlx::query("insert into shared_inspection_source_bindings(tenant_id,freeze_id,source_kind,source_id,work_order_id,receipt_id,sales_revision_id,template_id,version_no,item_code,batch_code,line_count,frozen_lines,source_snapshot,request_id,payload_hash) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
        .bind(tenant.0).bind(freeze).bind(source.kind.as_str()).bind(source.source_id)
        .bind(source.work_order).bind(source.receipt).bind(source.sales_revision).bind(row.0).bind(row.1)
        .bind(row.2).bind(row.3).bind(count).bind(json!(frozen)).bind(snapshot)
        .bind(request_id.trim()).bind(payload_hash)
        .execute(&mut **tx).await?.rows_affected()!=1 {
        anyhow::bail!("inspection source binding was not stored");
    }
    Ok(())
}
