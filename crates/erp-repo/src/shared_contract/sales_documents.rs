//! Current-authority document headers and queries use the original sales facts.
use super::{
    ReceiptValueActor, ReceiptValueAuthorizationError, SharedContractRepo, receipt_authority,
};
use erp_domain::trade::SalesLine;
use erp_types::TenantId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SalesDocumentRejected(pub String);

pub struct SalesDocumentLine {
    pub line: SalesLine,
    pub warehouse_id: Option<Uuid>,
}
pub(super) struct RevisionInput<'a> {
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub so_id: Option<Uuid>,
    pub customer_code: &'a str,
    pub expected_version: Option<i32>,
    pub action: &'a str,
    pub lines: &'a [SalesLine],
}
pub struct SalesDocumentInput<'a> {
    pub request_id: &'a str,
    pub order_id: Option<Uuid>,
    pub document_no: &'a str,
    pub customer_code: &'a str,
    pub currency: &'a str,
    pub action: &'a str,
    pub expected_version: Option<i32>,
    pub lines: &'a [SalesDocumentLine],
}
pub(super) struct Commit<'a> {
    pub input: &'a SalesDocumentInput<'a>,
    pub actor: ReceiptValueActor,
}
pub(super) struct Bound {
    pub guards: Vec<receipt_authority::Permit>,
    pub snapshot: Value,
    pub partner: Uuid,
    pub bindings: Vec<(i32, Uuid, Option<Uuid>)>,
}

impl SharedContractRepo<'_> {
    pub async fn list_sales_documents(
        &self,
        tenant: TenantId,
        after: Option<&str>,
        limit: i64,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        if ![100, 200, 300, 500, 800, 1000, 2000].contains(&limit)
            || after.is_some_and(|cursor| cursor.len() > 128)
        {
            anyhow::bail!("trade page budget invalid");
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read")
            .execute(&mut *tx)
            .await?;
        let identity = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["orders.read", "ORDERS.READ"],
        )
        .await?;
        // Hold the current grant rows used by the authorized list predicate.
        let _:Vec<Uuid>=sqlx::query_scalar("select a.warehouse_id from user_warehouse_access a join warehouses w on w.tenant_id=a.tenant_id and w.warehouse_id=a.warehouse_id where a.tenant_id=$1 and a.user_id=$2 order by a.warehouse_id for share of a,w").bind(tenant.0).bind(actor.user_id.0).fetch_all(&mut *tx).await?;
        let mut items:Vec<Value>=sqlx::query_scalar("select (h.snapshot-'authority')||jsonb_build_object('so_id',r.so_id,'revision_id',r.revision_id,'version_no',r.version_no,'status',r.status,'request_id',h.request_id) from trade_sales_documents d join lateral (select r.* from shared_sales_revisions r where r.tenant_id=d.tenant_id and r.so_id=d.so_id order by r.version_no desc limit 1) r on true join trade_sales_document_revisions h on h.tenant_id=r.tenant_id and h.revision_id=r.revision_id where d.tenant_id=$1 and ($3::text is null or d.document_no>$3) and not exists(select 1 from trade_sales_line_bindings b where b.tenant_id=r.tenant_id and b.revision_id=r.revision_id and b.warehouse_id is not null and not exists(select 1 from user_warehouse_access a join warehouses w on w.tenant_id=a.tenant_id and w.warehouse_id=a.warehouse_id where a.tenant_id=b.tenant_id and a.user_id=$2 and a.warehouse_id=b.warehouse_id)) order by d.document_no limit $4")
            .bind(tenant.0).bind(actor.user_id.0).bind(after).bind(limit+1).fetch_all(&mut *tx).await?;
        let has_more = items.len() > usize::try_from(limit)?;
        if has_more {
            items.pop();
        }
        let cursor = if has_more {
            items
                .last()
                .and_then(|item| item["document_no"].as_str())
                .map(str::to_string)
        } else {
            None
        };
        identity.recheck(&mut tx).await?;
        tx.commit().await?;
        Ok(
            json!({"items":items,"has_more":has_more,"next_cursor":cursor,"total":null,"count_kind":"UNKNOWN"}),
        )
    }
    pub async fn save_sales_document(
        &self,
        tenant: TenantId,
        input: SalesDocumentInput<'_>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        if input.document_no.trim().is_empty()
            || input.document_no.len() > 128
            || !["CNY", "USD", "EUR", "HKD"].contains(&input.currency)
            || input.lines.is_empty()
            || input.lines.len() > 1000
            || input.order_id.is_some_and(|id| id.is_nil())
        {
            anyhow::bail!("invalid trade document header or line budget");
        }
        if input.request_id.trim().is_empty()
            || input.request_id.len() > 128
            || input.customer_code.len() > 128
            || !["SUBMIT", "REVISE", "FULFILL", "CANCEL"].contains(&input.action)
            || (input.action != "SUBMIT"
                && (input.order_id.is_none()
                    || input.expected_version.is_none_or(|version| version <= 0)))
            || (input.action == "SUBMIT" && input.expected_version.is_some())
        {
            anyhow::bail!("trade document requires its original identity and expected version");
        }
        let lines = input
            .lines
            .iter()
            .map(|line| line.line.clone())
            .collect::<Vec<_>>();
        let fields = input
            .lines
            .iter()
            .map(|l| {
                json!([
                    l.line.line_no,
                    l.line.item_code,
                    l.line.qty_micros,
                    l.line.price_cent,
                    l.line.discount_cent,
                    l.line.tax_cent,
                    l.line.due_date,
                    l.line.address,
                    l.line.source,
                    l.warehouse_id
                ])
            })
            .collect::<Vec<_>>();
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&json!([
                "TRADE_DOC_V1",
                input.order_id,
                input.document_no.trim(),
                input.customer_code.trim(),
                input.currency,
                input.action,
                input.expected_version,
                fields
            ]))?)
        );
        let commit = Commit {
            input: &input,
            actor,
        };
        let (write, revision, version) = self
            .save_sales_revision_inner(
                tenant,
                RevisionInput {
                    request_id: input.request_id,
                    payload_hash: &fingerprint,
                    so_id: input.order_id,
                    customer_code: input.customer_code.trim(),
                    expected_version: input.expected_version,
                    action: input.action,
                    lines: &lines,
                },
                Some(&commit),
            )
            .await
            .map_err(|error| {
                // This call returns only after commit; commit/transport SQL errors stay unknown.
                // Logical errors raised inside it occur before the commit is sent.
                if error.downcast_ref::<sqlx::Error>().is_none()
                    && error
                        .downcast_ref::<ReceiptValueAuthorizationError>()
                        .is_none()
                {
                    anyhow::Error::new(SalesDocumentRejected(error.to_string()))
                } else {
                    error
                }
            })?
            .map_err(|_| anyhow::anyhow!("trade document request changed intent"))?;
        let mut tx = self.pool.begin().await?;
        let document = load(&mut tx, tenant, revision).await?;
        tx.commit().await?;
        Ok(
            json!({"request_id":input.request_id,"revision_id":revision,"version_no":version,"replay":matches!(write,super::SharedWrite::Replay(_)),"document":document}),
        )
    }

    pub async fn read_sales_document_number(
        &self,
        tenant: TenantId,
        number: &str,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        if number.trim().is_empty() || number.len() > 128 {
            anyhow::bail!("bad trade document number");
        }
        let mut tx = self.pool.begin().await?;
        let guard = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["orders.read", "ORDERS.READ"],
        )
        .await?;
        let order: Uuid = sqlx::query_scalar(
            "select so_id from trade_sales_documents where tenant_id=$1 and document_no=$2",
        )
        .bind(tenant.0)
        .bind(number.trim())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ReceiptValueAuthorizationError::Unavailable)?;
        guard.recheck(&mut tx).await?;
        tx.commit().await?;
        self.read_sales_document(tenant, order, None, actor).await
    }

    pub async fn read_sales_document_command(
        &self,
        tenant: TenantId,
        request: &str,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        if request.trim().is_empty() || request.len() > 128 {
            anyhow::bail!("bad original trade request");
        }
        let mut tx = self.pool.begin().await?;
        let identity = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["orders.read", "ORDERS.READ"],
        )
        .await?;
        let revision:Uuid=sqlx::query_scalar("select h.revision_id from trade_sales_document_revisions h join command_receipts c on c.tenant_id=h.tenant_id and c.request_id=h.request_id where h.tenant_id=$1 and h.request_id=$2 and c.status='SUCCEEDED' and c.effect_id=h.revision_id::text").bind(tenant.0).bind(request).fetch_optional(&mut *tx).await?.ok_or(ReceiptValueAuthorizationError::Unavailable)?;
        let guards = warehouse_guards(
            &mut tx,
            tenant,
            revision,
            actor,
            &["orders.read", "ORDERS.READ"],
        )
        .await?;
        let document = load(&mut tx, tenant, revision).await?;
        identity.recheck(&mut tx).await?;
        for guard in guards {
            guard.recheck(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(document)
    }

    pub async fn read_sales_document(
        &self,
        tenant: TenantId,
        order: Uuid,
        request: Option<&str>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        let identity = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["orders.read", "ORDERS.READ"],
        )
        .await?;
        let revision: Option<Uuid> = if let Some(request) = request {
            sqlx::query_scalar("select h.revision_id from trade_sales_document_revisions h join shared_sales_revisions r on r.tenant_id=h.tenant_id and r.revision_id=h.revision_id where h.tenant_id=$1 and h.so_id=$2 and h.request_id=$3").bind(tenant.0).bind(order).bind(request).fetch_optional(&mut *tx).await?
        } else {
            sqlx::query_scalar("select r.revision_id from shared_sales_revisions r join trade_sales_document_revisions h on h.tenant_id=r.tenant_id and h.revision_id=r.revision_id where r.tenant_id=$1 and r.so_id=$2 order by r.version_no desc limit 1").bind(tenant.0).bind(order).fetch_optional(&mut *tx).await?
        };
        let revision = revision.ok_or(ReceiptValueAuthorizationError::Unavailable)?;
        let guards = warehouse_guards(
            &mut tx,
            tenant,
            revision,
            actor,
            &["orders.read", "ORDERS.READ"],
        )
        .await?;
        let document = load(&mut tx, tenant, revision).await?;
        identity.recheck(&mut tx).await?;
        for guard in guards {
            guard.recheck(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(document)
    }
}

pub(super) async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    commit: &Commit<'_>,
) -> anyhow::Result<Bound> {
    let input = commit.input;
    let mut guards = vec![
        receipt_authority::lock_identity(
            tx,
            tenant,
            commit.actor,
            &["orders.write", "ORDERS.WRITE"],
        )
        .await?,
    ];
    let old: Option<(String, String)> = sqlx::query_as(
        "select document_no,currency from trade_sales_documents where tenant_id=$1 and so_id=$2",
    )
    .bind(tenant.0)
    .bind(order)
    .fetch_optional(&mut **tx)
    .await?;
    let managed = old.is_some();
    if old
        .is_some_and(|(no, currency)| no != input.document_no.trim() || currency != input.currency)
    {
        anyhow::bail!("trade document number and currency are immutable");
    }
    if !managed {
        let legacy:bool=sqlx::query_scalar("select exists(select 1 from shared_sales_revisions where tenant_id=$1 and so_id=$2) or exists(select 1 from sales_orders where tenant_id=$1 and sales_order_id=$2)").bind(tenant.0).bind(order).fetch_one(&mut **tx).await?;
        if legacy {
            anyhow::bail!("legacy sales identity requires explicit header/source reconciliation");
        }
    }
    let partners:Vec<Uuid>=sqlx::query_scalar("select partner_id from shared_partners where tenant_id=$1 and customer_no=$2 and customer=true and status='ACTIVE' order by partner_id limit 2 for share").bind(tenant.0).bind(input.customer_code.trim()).fetch_all(&mut **tx).await?;
    let [partner] = partners.as_slice() else {
        anyhow::bail!("trade document requires one active customer");
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut bindings = Vec::new();
    let mut total = 0i64;
    let warehouses = input
        .lines
        .iter()
        .filter_map(|l| l.warehouse_id)
        .collect::<std::collections::BTreeSet<_>>();
    for warehouse in warehouses {
        let mut actor = commit.actor;
        actor.warehouse_id = warehouse;
        guards.push(
            receipt_authority::lock_permissions(
                tx,
                tenant,
                actor,
                &["orders.write", "ORDERS.WRITE"],
            )
            .await?,
        );
    }
    for item in input.lines {
        let line = &item.line;
        if !seen.insert(line.line_no)
            || line.line_no <= 0
            || line.qty_micros <= 0
            || line.price_cent < 0
            || line.discount_cent < 0
            || line.tax_cent < 0
            || line.item_code.len() > 128
            || line.address.len() > 512
            || chrono::NaiveDate::parse_from_str(&line.due_date, "%Y-%m-%d").is_err()
        {
            anyhow::bail!("invalid trade document line");
        }
        let skus:Vec<Uuid>=sqlx::query_scalar("select sku_id from skus where tenant_id=$1 and sku_code=$2 order by sku_id limit 2 for share").bind(tenant.0).bind(&line.item_code).fetch_all(&mut **tx).await?;
        let [sku] = skus.as_slice() else {
            anyhow::bail!("trade document SKU is unavailable or ambiguous");
        };
        let amount = i128::from(line.qty_micros) * i128::from(line.price_cent) / 1_000_000
            - i128::from(line.discount_cent)
            + i128::from(line.tax_cent);
        let amount =
            i64::try_from(amount).map_err(|_| anyhow::anyhow!("trade line amount overflow"))?;
        if amount < 0 {
            anyhow::bail!("trade line discount exceeds value");
        }
        total = total
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("trade total overflow"))?;
        bindings.push((line.line_no, *sku, item.warehouse_id));
    }
    Ok(Bound {
        snapshot: json!({"document_no":input.document_no.trim(),"currency":input.currency,"partner_id":partner,"customer_code":input.customer_code.trim(),"total_cent":total,"actor":guards[0].snapshot(),"authority":guards.iter().map(receipt_authority::Permit::snapshot).collect::<Vec<_>>()}),
        guards,
        partner: *partner,
        bindings,
    })
}

pub(super) async fn store(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    revision: Uuid,
    commit: &Commit<'_>,
    bound: &Bound,
) -> anyhow::Result<()> {
    sqlx::query("insert into trade_sales_documents(tenant_id,so_id,document_no,currency) values($1,$2,$3,$4) on conflict(tenant_id,so_id) do nothing").bind(tenant.0).bind(order).bind(commit.input.document_no.trim()).bind(commit.input.currency).execute(&mut **tx).await?;
    sqlx::query("insert into trade_sales_document_revisions(tenant_id,revision_id,so_id,request_id,partner_id,snapshot) values($1,$2,$3,$4,$5,$6)").bind(tenant.0).bind(revision).bind(order).bind(commit.input.request_id).bind(bound.partner).bind(&bound.snapshot).execute(&mut **tx).await?;
    for (line, sku, warehouse) in &bound.bindings {
        sqlx::query("insert into trade_sales_line_bindings(tenant_id,revision_id,line_no,sku_id,warehouse_id) values($1,$2,$3,$4,$5)").bind(tenant.0).bind(revision).bind(line).bind(sku).bind(warehouse).execute(&mut **tx).await?;
    }
    crate::operation_logs::OperationLogsRepo::insert_with_tx(
        &mut **tx,
        crate::operation_logs::OperationLogCreate {
            tenant_id: tenant,
            actor_type: "USER".into(),
            actor_user_id: Some(commit.actor.user_id),
            action: "TRADE_SALES_SAVE".into(),
            ref_type: Some("sales_revision".into()),
            ref_id: Some(revision.to_string()),
            metadata: Some(bound.snapshot.clone()),
        },
    )
    .await?;
    crate::audit_logs::AuditLogsRepo::insert_with_tx(
        &mut **tx,
        crate::audit_logs::AuditLogCreate {
            tenant_id: tenant,
            user_id: Some(commit.actor.user_id),
            action: "trade.sales.save".into(),
            entity_type: "sales_revision".into(),
            entity_id: Some(revision.to_string()),
            before_json: None,
            after_json: Some(bound.snapshot.clone()),
            ip: None,
            ua: None,
        },
    )
    .await?;
    crate::outbox::OutboxRepo::enqueue_with_tx(
        &mut **tx,
        tenant,
        "trade.sales.saved",
        &json!({"so_id":order,"revision_id":revision,"request_id":commit.input.request_id}),
        chrono::Utc::now(),
    )
    .await?;
    for guard in &bound.guards {
        guard.recheck(tx).await?;
    }
    Ok(())
}

pub(super) async fn warehouse_guards(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    revision: Uuid,
    actor: ReceiptValueActor,
    permissions: &[&str],
) -> anyhow::Result<Vec<receipt_authority::Permit>> {
    let warehouses:Vec<Uuid>=sqlx::query_scalar("select distinct warehouse_id from trade_sales_line_bindings where tenant_id=$1 and revision_id=$2 and warehouse_id is not null order by warehouse_id limit 1001").bind(tenant.0).bind(revision).fetch_all(&mut **tx).await?;
    if warehouses.len() > 1000 {
        anyhow::bail!("trade warehouse budget exceeded");
    }
    let mut guards = Vec::new();
    for warehouse in warehouses {
        let mut actor = actor;
        actor.warehouse_id = warehouse;
        guards.push(receipt_authority::lock_permissions(tx, tenant, actor, permissions).await?);
    }
    Ok(guards)
}

async fn load(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    revision: Uuid,
) -> anyhow::Result<Value> {
    let header:Value=sqlx::query_scalar("select h.snapshot || jsonb_build_object('so_id',r.so_id,'revision_id',r.revision_id,'version_no',r.version_no,'status',r.status,'request_id',h.request_id) from shared_sales_revisions r join trade_sales_document_revisions h on h.tenant_id=r.tenant_id and h.revision_id=r.revision_id where r.tenant_id=$1 and r.revision_id=$2").bind(tenant.0).bind(revision).fetch_optional(&mut **tx).await?.ok_or(ReceiptValueAuthorizationError::Unavailable)?;
    let lines:Vec<Value>=sqlx::query_scalar("select to_jsonb(l)||jsonb_build_object('qty_micros',l.qty_micros::text,'sku_id',b.sku_id,'warehouse_id',b.warehouse_id) from shared_sales_lines l join trade_sales_line_bindings b on b.tenant_id=l.tenant_id and b.revision_id=l.revision_id and b.line_no=l.line_no where l.tenant_id=$1 and l.revision_id=$2 order by l.line_no limit 1001").bind(tenant.0).bind(revision).fetch_all(&mut **tx).await?;
    if lines.is_empty() || lines.len() > 1000 {
        anyhow::bail!("trade detail line budget invalid");
    }
    Ok(json!({"header":header,"items":lines}))
}
