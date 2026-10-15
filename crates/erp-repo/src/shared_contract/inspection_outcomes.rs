//! Append measured outcomes and authenticated reviews under the original source lock.
use super::inspection_sources::{LockedSource, SourceKind, lock_source};
use super::{SharedContractRepo, SharedWrite, insert_receipt, load_receipt};
use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    operation_logs::{OperationLogCreate, OperationLogsRepo},
};
use erp_domain::{
    manufacturing::inspection_outcomes::record_measurement,
    receipt::{ReceiptError, admit_retry, succeeded},
};
use erp_types::{TenantId, UserId};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const MAX_ATTEMPTS: i32 = 20;
const MAX_PROOF_ROWS: usize = 1000;

#[derive(Clone, Copy, Debug)]
pub enum InspectionActor {
    Internal,
    Authenticated {
        tenant_id: TenantId,
        user_id: UserId,
    },
}

impl InspectionActor {
    async fn require_active(
        self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
    ) -> anyhow::Result<Option<UserId>> {
        let Self::Authenticated { tenant_id, user_id } = self else {
            return Ok(None);
        };
        if tenant_id != tenant || user_id.0.is_nil() {
            return Err(InspectionOutcomeError::Forbidden.into());
        }
        let status: Option<String> = sqlx::query_scalar(
            "select status from users where tenant_id=$1 and user_id=$2 for share",
        )
        .bind(tenant.0)
        .bind(user_id.0)
        .fetch_optional(&mut **tx)
        .await?;
        if status.as_deref() != Some("ACTIVE") {
            return Err(InspectionOutcomeError::Forbidden.into());
        }
        Ok(Some(user_id))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InspectionOutcomeError {
    #[error("inspection actor is not active in this enterprise")]
    Forbidden,
    #[error("inspection source has already completed its physical release")]
    SourceClosed,
    #[error("inspection history requires reconciliation")]
    HistoryRequired,
    #[error("inspection exceeds its bounded reading budget")]
    Limit,
}

pub struct InspectionReadingInput<'a> {
    pub freeze_id: Uuid,
    pub item_code: &'a str,
    pub reading: Option<i64>,
    pub inspector: &'a str,
    pub declared_reviewer: &'a str,
    pub previous_item_id: Option<Uuid>,
    pub reason: &'a str,
}

pub struct InspectionReviewInput<'a> {
    pub freeze_id: Uuid,
    pub item_id: Uuid,
    pub decision: &'a str,
    pub reason: &'a str,
}

pub struct InspectionOutcomeWrite {
    pub write: SharedWrite,
    pub snapshot: Value,
}

#[derive(sqlx::FromRow)]
struct FrozenLine {
    source_kind: String,
    source_id: Uuid,
    product_code: String,
    batch_code: String,
    required: bool,
    min_micros: i64,
    max_micros: i64,
}

async fn lock_line(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    freeze: Uuid,
    item: &str,
) -> anyhow::Result<(FrozenLine, LockedSource)> {
    if item.trim().is_empty() || item.len() > 1024 || freeze.is_nil() {
        anyhow::bail!("bad frozen inspection line");
    }
    let source: Option<(String, Uuid)> = sqlx::query_as(
        "select case when octet_length(source_kind)<=32 then source_kind else '' end,source_id from shared_inspection_freezes where tenant_id=$1 and freeze_id=$2",
    ).bind(tenant.0).bind(freeze).fetch_optional(&mut **tx).await?;
    let Some((kind, source_id)) = source else {
        anyhow::bail!("frozen inspection source is outside this tenant");
    };
    let owner = lock_source(tx, tenant, &kind, source_id).await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
        .bind(tenant.0.to_string())
        .bind(format!("inspection-reading:{freeze}:{item}"))
        .execute(&mut **tx)
        .await?;
    let line: Option<FrozenLine> = sqlx::query_as(
        "select f.source_kind,f.source_id,f.item_code product_code,f.batch_code,l.required,l.min_micros,l.max_micros from shared_inspection_freezes f join shared_inspection_freeze_lines l on l.tenant_id=f.tenant_id and l.freeze_id=f.freeze_id where f.tenant_id=$1 and f.freeze_id=$2 and l.item_code=$3 for share of f,l",
    ).bind(tenant.0).bind(freeze).bind(item).fetch_optional(&mut **tx).await?;
    let line =
        line.ok_or_else(|| anyhow::anyhow!("frozen inspection line is outside this tenant"))?;
    if SourceKind::parse(&line.source_kind)? != owner.kind || line.source_id != owner.source_id {
        return Err(InspectionOutcomeError::HistoryRequired.into());
    }
    let bound: bool = sqlx::query_scalar(
        "select exists(select 1 from shared_inspection_source_bindings where tenant_id=$1 and freeze_id=$2)",
    ).bind(tenant.0).bind(freeze).fetch_one(&mut **tx).await?;
    if !bound {
        return Err(InspectionOutcomeError::HistoryRequired.into());
    }
    Ok((line, owner))
}

async fn require_open(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    line: &FrozenLine,
    source: &LockedSource,
) -> anyhow::Result<()> {
    let closed: bool = match source.kind {
        SourceKind::ProductionReceipt => sqlx::query_scalar(
            "select status<>'HOLD' or available_micros<>0 from mfg_receipts where tenant_id=$1 and receipt_id=$2",
        ).bind(tenant.0).bind(source.source_id).fetch_one(&mut **tx).await?,
        SourceKind::WorkOrder => sqlx::query_scalar(
            "select exists(select 1 from mfg_receipts where tenant_id=$1 and work_order_id=$2 and status='RELEASED' and($3='' or batch_code=$3)) or exists(select 1 from mfg_work_orders where tenant_id=$1 and work_order_id=$2 and status in('CANCELLED','CLOSED'))",
        ).bind(tenant.0).bind(source.source_id).bind(&line.batch_code).fetch_one(&mut **tx).await?,
        SourceKind::SalesOrder => sqlx::query_scalar(
            "select exists(select 1 from shared_sales_deliveries where tenant_id=$1 and so_id=$2 and item_code=$3 and stage='SHIPPED')",
        ).bind(tenant.0).bind(source.source_id).bind(&line.product_code).fetch_one(&mut **tx).await?,
    };
    if closed {
        return Err(InspectionOutcomeError::SourceClosed.into());
    }
    Ok(())
}

async fn lock_request(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    request: &str,
) -> anyhow::Result<()> {
    sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
        .bind(tenant.0.to_string())
        .bind(format!("inspection-command:{request}"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn log_effect(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: Option<UserId>,
    action: &str,
    item: Uuid,
    snapshot: &Value,
) -> anyhow::Result<()> {
    OperationLogsRepo::insert_with_tx(
        &mut **tx,
        OperationLogCreate {
            tenant_id: tenant,
            actor_type: if actor.is_some() { "USER" } else { "SYSTEM" }.into(),
            actor_user_id: actor,
            action: action.into(),
            ref_type: Some("inspection_reading".into()),
            ref_id: Some(item.to_string()),
            metadata: Some(snapshot.clone()),
        },
    )
    .await?;
    AuditLogsRepo::insert_with_tx(
        &mut **tx,
        AuditLogCreate {
            tenant_id: tenant,
            user_id: actor,
            action: action.to_ascii_lowercase(),
            entity_type: "inspection_reading".into(),
            entity_id: Some(item.to_string()),
            before_json: None,
            after_json: Some(snapshot.clone()),
            ip: None,
            ua: None,
        },
    )
    .await?;
    Ok(())
}

impl SharedContractRepo<'_> {
    /// First readings use the established v1 command. Reinspection is an explicit append.
    pub async fn save_inspection_outcome(
        &self,
        tenant: TenantId,
        request: &str,
        payload: &str,
        input: InspectionReadingInput<'_>,
        actor: InspectionActor,
    ) -> anyhow::Result<Result<InspectionOutcomeWrite, ReceiptError>> {
        if input.declared_reviewer.trim().is_empty()
            || input.declared_reviewer.len() > 512
            || input.inspector.len() > 512
            || (input.reading.is_some() && input.inspector.trim().is_empty())
            || input.reason.len() > 2048
            || (input.previous_item_id.is_some() && input.reason.trim().is_empty())
            || (input.previous_item_id.is_none() && !input.reason.is_empty())
        {
            anyhow::bail!("bad inspection reading responsibility or reason");
        }
        let mut tx = self.pool.begin().await?;
        let (line, source) = lock_line(&mut tx, tenant, input.freeze_id, input.item_code).await?;
        let user = actor.require_active(&mut tx, tenant).await?;
        lock_request(&mut tx, tenant, request).await?;
        match admit_retry(
            load_receipt(&mut tx, tenant, request).await?.as_ref(),
            request,
            payload,
        ) {
            Ok(Some(existing)) => {
                let original: Option<Value> = sqlx::query_scalar(
                    "select snapshot from shared_inspection_reading_events where tenant_id=$1 and request_id=$2 and item_id::text=$3 and freeze_id=$4 and item_code=$5 and payload_hash=$6",
                ).bind(tenant.0).bind(request).bind(&existing.effect_id).bind(input.freeze_id)
                    .bind(input.item_code).bind(payload).fetch_optional(&mut *tx).await?;
                let snapshot = if let Some(original) = original {
                    original
                } else {
                    // Preserve pre-migration results without inventing reviewer or actor history.
                    let old: Option<Value> = sqlx::query_scalar(
                        "select to_jsonb(q) from mfg_quality_items q where tenant_id=$1 and item_id::text=$2 and freeze_id=$3 and item_code=$4",
                    ).bind(tenant.0).bind(&existing.effect_id).bind(input.freeze_id)
                        .bind(input.item_code).fetch_optional(&mut *tx).await?;
                    json!({"provenance":"LEGACY","original_reading":old.ok_or(InspectionOutcomeError::HistoryRequired)?})
                };
                tx.rollback().await?;
                return Ok(Ok(InspectionOutcomeWrite {
                    write: SharedWrite::Replay(existing),
                    snapshot,
                }));
            }
            Err(error) => {
                tx.rollback().await?;
                return Ok(Err(error));
            }
            Ok(None) => {}
        }
        source
            .require_item(&mut tx, tenant, &line.product_code, &line.batch_code)
            .await?;
        require_open(&mut tx, tenant, &line, &source).await?;
        let count: i64 = sqlx::query_scalar(
            "select count(*) from mfg_quality_items where tenant_id=$1 and freeze_id=$2 and item_code=$3",
        ).bind(tenant.0).bind(input.freeze_id).bind(input.item_code).fetch_one(&mut *tx).await?;
        let attempt = if let Some(previous) = input.previous_item_id {
            let latest: Option<(Uuid, i32, String, Option<String>)> = sqlx::query_as(
                "select e.item_id,e.attempt_no,e.verdict,r.decision from shared_inspection_reading_events e left join shared_inspection_reading_reviews r on r.tenant_id=e.tenant_id and r.item_id=e.item_id where e.tenant_id=$1 and e.freeze_id=$2 and e.item_code=$3 order by e.attempt_no desc limit 1",
            ).bind(tenant.0).bind(input.freeze_id).bind(input.item_code).fetch_optional(&mut *tx).await?;
            let Some((latest, attempt, verdict, review)) = latest else {
                return Err(InspectionOutcomeError::HistoryRequired.into());
            };
            if latest != previous || count != i64::from(attempt) {
                anyhow::bail!("reinspection must cite the latest original reading");
            }
            if verdict == "PASSED" && review.as_deref() != Some("REJECT") {
                anyhow::bail!("passed reading cannot be replaced without an actual rejection");
            }
            if attempt >= MAX_ATTEMPTS {
                return Err(InspectionOutcomeError::Limit.into());
            }
            attempt + 1
        } else {
            if count != 0 {
                anyhow::bail!("inspection reading is already stored for this frozen line");
            }
            1
        };
        let outcome = record_measurement(
            input.item_code,
            line.required,
            line.min_micros,
            line.max_micros,
            input.reading,
        )
        .map_err(|error| anyhow::anyhow!("inspection reading rejected: {error:?}"))?;
        let item = Uuid::new_v4();
        let snapshot = json!({"schema":1,"tenant_id":tenant,"item_id":item,"freeze_id":input.freeze_id,
            "item_code":input.item_code,"source_kind":source.kind.as_str(),"source_id":source.source_id,
            "batch_code":line.batch_code,"required":line.required,"min_micros":line.min_micros.to_string(),
            "max_micros":line.max_micros.to_string(),"reading_micros":input.reading.map(|v|v.to_string()).unwrap_or_default(),
            "reading_present":outcome.present,"passed":outcome.passed,"verdict":outcome.verdict.as_str(),
            "inspector":input.inspector,"declared_reviewer":input.declared_reviewer,
            "actor_source":if user.is_some(){"USER"}else{"SYSTEM"},"actor_user_id":user,
            "attempt_no":attempt,"previous_item_id":input.previous_item_id,"reason":input.reason,
            "review_required":attempt>1});
        sqlx::query(
            "insert into mfg_quality_items(tenant_id,item_id,work_order_id,batch_code,item_code,required,min_micros,max_micros,reading_micros,reading_present,passed,inspector,freeze_id,source_kind,source_id) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
        ).bind(tenant.0).bind(item).bind(source.work_order).bind(&line.batch_code).bind(input.item_code)
            .bind(line.required).bind(line.min_micros).bind(line.max_micros).bind(outcome.stored_micros)
            .bind(outcome.present).bind(outcome.passed).bind(input.inspector).bind(input.freeze_id)
            .bind(source.kind.as_str()).bind(source.source_id).execute(&mut *tx).await?;
        sqlx::query(
            "insert into shared_inspection_reading_events(tenant_id,item_id,freeze_id,item_code,attempt_no,previous_item_id,review_required,verdict,declared_reviewer,actor_source,actor_user_id,reason,request_id,payload_hash,snapshot) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
        ).bind(tenant.0).bind(item).bind(input.freeze_id).bind(input.item_code).bind(attempt)
            .bind(input.previous_item_id).bind(attempt>1).bind(outcome.verdict.as_str()).bind(input.declared_reviewer)
            .bind(if user.is_some(){"USER"}else{"SYSTEM"}).bind(user.map(|id|id.0)).bind(input.reason)
            .bind(request).bind(payload).bind(&snapshot).execute(&mut *tx).await?;
        log_effect(&mut tx, tenant, user, "QUALITY_READING", item, &snapshot).await?;
        let command = succeeded(request, payload, &item.to_string());
        insert_receipt(&mut tx, tenant, &command).await?;
        tx.commit().await?;
        Ok(Ok(InspectionOutcomeWrite {
            write: SharedWrite::Stored(command),
            snapshot,
        }))
    }

    pub async fn review_inspection_outcome(
        &self,
        tenant: TenantId,
        request: &str,
        payload: &str,
        input: InspectionReviewInput<'_>,
        actor: InspectionActor,
    ) -> anyhow::Result<Result<InspectionOutcomeWrite, ReceiptError>> {
        if !matches!(input.decision, "APPROVE" | "REJECT")
            || input.reason.len() > 2048
            || (input.decision == "REJECT" && input.reason.trim().is_empty())
        {
            anyhow::bail!("bad inspection review decision or reason");
        }
        let mut tx = self.pool.begin().await?;
        let item: Option<String> = sqlx::query_scalar(
            "select item_code from shared_inspection_reading_events where tenant_id=$1 and freeze_id=$2 and item_id=$3",
        ).bind(tenant.0).bind(input.freeze_id).bind(input.item_id).fetch_optional(&mut *tx).await?;
        let item =
            item.ok_or_else(|| anyhow::anyhow!("reading is outside this frozen inspection"))?;
        let (line, source) = lock_line(&mut tx, tenant, input.freeze_id, &item).await?;
        let user = actor
            .require_active(&mut tx, tenant)
            .await?
            .ok_or(InspectionOutcomeError::Forbidden)?;
        lock_request(&mut tx, tenant, request).await?;
        match admit_retry(
            load_receipt(&mut tx, tenant, request).await?.as_ref(),
            request,
            payload,
        ) {
            Ok(Some(existing)) => {
                let snapshot: Option<Value> = sqlx::query_scalar(
                    "select snapshot from shared_inspection_reading_reviews where tenant_id=$1 and request_id=$2 and review_id::text=$3 and item_id=$4 and payload_hash=$5",
                ).bind(tenant.0).bind(request).bind(&existing.effect_id).bind(input.item_id)
                    .bind(payload).fetch_optional(&mut *tx).await?;
                let snapshot = snapshot.ok_or(InspectionOutcomeError::HistoryRequired)?;
                tx.rollback().await?;
                return Ok(Ok(InspectionOutcomeWrite {
                    write: SharedWrite::Replay(existing),
                    snapshot,
                }));
            }
            Err(error) => {
                tx.rollback().await?;
                return Ok(Err(error));
            }
            Ok(None) => {}
        }
        source
            .require_item(&mut tx, tenant, &line.product_code, &line.batch_code)
            .await?;
        require_open(&mut tx, tenant, &line, &source).await?;
        let latest: Option<(Uuid, String, Value)> = sqlx::query_as(
            "select item_id,verdict,snapshot from shared_inspection_reading_events where tenant_id=$1 and freeze_id=$2 and item_code=$3 order by attempt_no desc limit 1",
        ).bind(tenant.0).bind(input.freeze_id).bind(&item).fetch_optional(&mut *tx).await?;
        let Some((latest, verdict, reading)) = latest else {
            return Err(InspectionOutcomeError::HistoryRequired.into());
        };
        if latest != input.item_id || (input.decision == "APPROVE" && verdict != "PASSED") {
            anyhow::bail!("review must cite the latest passed measurement for approval");
        }
        let reviewed: bool = sqlx::query_scalar(
            "select exists(select 1 from shared_inspection_reading_reviews where tenant_id=$1 and item_id=$2)",
        ).bind(tenant.0).bind(input.item_id).fetch_one(&mut *tx).await?;
        if reviewed {
            anyhow::bail!("original reading already has an immutable review");
        }
        let review = Uuid::new_v4();
        let snapshot = json!({"schema":1,"tenant_id":tenant,"review_id":review,"item_id":input.item_id,
            "freeze_id":input.freeze_id,"reviewer_user_id":user,"decision":input.decision,
            "reason":input.reason,"reading":reading});
        sqlx::query(
            "insert into shared_inspection_reading_reviews(tenant_id,review_id,item_id,reviewer_user_id,decision,reason,request_id,payload_hash,snapshot) values($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        ).bind(tenant.0).bind(review).bind(input.item_id).bind(user.0).bind(input.decision)
            .bind(input.reason).bind(request).bind(payload).bind(&snapshot).execute(&mut *tx).await?;
        log_effect(
            &mut tx,
            tenant,
            Some(user),
            "QUALITY_READING_REVIEW",
            input.item_id,
            &snapshot,
        )
        .await?;
        let command = succeeded(request, payload, &review.to_string());
        insert_receipt(&mut tx, tenant, &command).await?;
        tx.commit().await?;
        Ok(Ok(InspectionOutcomeWrite {
            write: SharedWrite::Stored(command),
            snapshot,
        }))
    }

    /// One line has at most 20 attempts. Query its authorized bounded history, including reviews.
    pub async fn read_inspection_history(
        &self,
        tenant: TenantId,
        freeze: Uuid,
        item: &str,
        actor: InspectionActor,
    ) -> anyhow::Result<Value> {
        if item.trim().is_empty() || item.len() > 1024 || freeze.is_nil() {
            anyhow::bail!("bad frozen inspection line");
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read read only")
            .execute(&mut *tx)
            .await?;
        if let InspectionActor::Authenticated { tenant_id, user_id } = actor {
            if tenant_id != tenant || user_id.0.is_nil() {
                return Err(InspectionOutcomeError::Forbidden.into());
            }
            let status: Option<String> =
                sqlx::query_scalar("select status from users where tenant_id=$1 and user_id=$2")
                    .bind(tenant.0)
                    .bind(user_id.0)
                    .fetch_optional(&mut *tx)
                    .await?;
            if status.as_deref() != Some("ACTIVE") {
                return Err(InspectionOutcomeError::Forbidden.into());
            }
        }
        let owned: bool = sqlx::query_scalar(
            "select exists(select 1 from shared_inspection_freezes f join shared_inspection_freeze_lines l on l.tenant_id=f.tenant_id and l.freeze_id=f.freeze_id join shared_inspection_source_bindings b on b.tenant_id=f.tenant_id and b.freeze_id=f.freeze_id where f.tenant_id=$1 and f.freeze_id=$2 and l.item_code=$3)",
        ).bind(tenant.0).bind(freeze).bind(item).fetch_one(&mut *tx).await?;
        if !owned {
            anyhow::bail!("frozen inspection line is outside this tenant");
        }
        let rows: Vec<Value> = sqlx::query_scalar(
            "select jsonb_build_object('reading',e.snapshot,'request_id',e.request_id,'created_at',e.created_at,'review',r.snapshot) from shared_inspection_reading_events e left join shared_inspection_reading_reviews r on r.tenant_id=e.tenant_id and r.item_id=e.item_id where e.tenant_id=$1 and e.freeze_id=$2 and e.item_code=$3 order by e.attempt_no limit 21",
        ).bind(tenant.0).bind(freeze).bind(item).fetch_all(&mut *tx).await?;
        if rows.len() > MAX_ATTEMPTS as usize {
            return Err(InspectionOutcomeError::Limit.into());
        }
        let legacy: i64 = sqlx::query_scalar(
            "select count(*) from(select q.item_id from mfg_quality_items q where q.tenant_id=$1 and q.freeze_id=$2 and q.item_code=$3 and not exists(select 1 from shared_inspection_reading_events e where e.tenant_id=q.tenant_id and e.item_id=q.item_id) limit 1001) legacy",
        ).bind(tenant.0).bind(freeze).bind(item).fetch_one(&mut *tx).await?;
        if legacy > 1000 {
            return Err(InspectionOutcomeError::Limit.into());
        }
        tx.rollback().await?;
        Ok(
            json!({"freeze_id":freeze,"item_code":item,"attempt_limit":MAX_ATTEMPTS,
            "history":rows,"legacy_reading_count":legacy,"legacy_provenance_complete":legacy==0}),
        )
    }
}

pub(super) struct EffectiveReadings {
    pub(super) readings: Vec<Value>,
    pub(super) events: Vec<Value>,
    pub(super) reviews: Vec<Value>,
}

/// Original failed rows remain in the proof. Only the latest reviewed attempt is effective.
pub(super) async fn effective_readings(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    originals: &[Value],
) -> anyhow::Result<EffectiveReadings> {
    if originals.len() > MAX_PROOF_ROWS {
        return Err(InspectionOutcomeError::Limit.into());
    }
    let ids: Vec<Uuid> = originals
        .iter()
        .map(|q| {
            q["item_id"]
                .as_str()
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or(InspectionOutcomeError::HistoryRequired)
        })
        .collect::<Result<_, _>>()?;
    let events: Vec<Value> = sqlx::query_scalar(
        "select to_jsonb(e) from shared_inspection_reading_events e where tenant_id=$1 and item_id=any($2::uuid[]) order by freeze_id,item_code,attempt_no limit 1001 for share",
    ).bind(tenant.0).bind(&ids).fetch_all(&mut **tx).await?;
    let reviews: Vec<Value> = sqlx::query_scalar(
        "select to_jsonb(r) from shared_inspection_reading_reviews r where tenant_id=$1 and item_id=any($2::uuid[]) order by item_id limit 1001 for share",
    ).bind(tenant.0).bind(&ids).fetch_all(&mut **tx).await?;
    if events.len() > MAX_PROOF_ROWS || reviews.len() > MAX_PROOF_ROWS {
        return Err(InspectionOutcomeError::Limit.into());
    }
    let mut groups: BTreeMap<(String, String), Vec<&Value>> = BTreeMap::new();
    let mut current = Vec::new();
    let mut typed_ids = BTreeSet::new();
    for event in &events {
        let q = originals
            .iter()
            .find(|q| q["item_id"] == event["item_id"])
            .ok_or(InspectionOutcomeError::HistoryRequired)?;
        if q["freeze_id"] != event["freeze_id"] || q["item_code"] != event["item_code"] {
            return Err(InspectionOutcomeError::HistoryRequired.into());
        }
        let item = event["item_id"]
            .as_str()
            .ok_or(InspectionOutcomeError::HistoryRequired)?;
        typed_ids.insert(item.to_string());
        let key = (
            event["freeze_id"]
                .as_str()
                .ok_or(InspectionOutcomeError::HistoryRequired)?
                .to_string(),
            event["item_code"]
                .as_str()
                .ok_or(InspectionOutcomeError::HistoryRequired)?
                .to_string(),
        );
        groups.entry(key).or_default().push(event);
    }
    for ((freeze, item), chain) in &groups {
        if chain.len() > MAX_ATTEMPTS as usize {
            return Err(InspectionOutcomeError::Limit.into());
        }
        for (index, event) in chain.iter().enumerate() {
            if event["attempt_no"].as_u64() != Some(index as u64 + 1)
                || (index == 0 && !event["previous_item_id"].is_null())
                || (index > 0 && event["previous_item_id"] != chain[index - 1]["item_id"])
            {
                return Err(InspectionOutcomeError::HistoryRequired.into());
            }
        }
        let latest = chain
            .last()
            .ok_or(InspectionOutcomeError::HistoryRequired)?;
        let mut q = originals
            .iter()
            .find(|q| q["item_id"] == latest["item_id"])
            .ok_or(InspectionOutcomeError::HistoryRequired)?
            .clone();
        if originals.iter().any(|q| {
            q["freeze_id"].as_str() == Some(freeze)
                && q["item_code"].as_str() == Some(item)
                && q["item_id"]
                    .as_str()
                    .is_some_and(|id| !typed_ids.contains(id))
        }) {
            return Err(InspectionOutcomeError::HistoryRequired.into());
        }
        let review = reviews.iter().find(|r| r["item_id"] == latest["item_id"]);
        let decision = review.and_then(|r| r["decision"].as_str());
        q["review_required"] = latest["review_required"].clone();
        q["review_decision"] = json!(decision);
        q["review_allows_release"] = json!(
            decision != Some("REJECT")
                && (latest["review_required"] == false || decision == Some("APPROVE"))
        );
        current.push(q);
    }
    current.extend(
        originals
            .iter()
            .filter(|q| {
                q["item_id"]
                    .as_str()
                    .is_some_and(|id| !typed_ids.contains(id))
            })
            .cloned(),
    );
    Ok(EffectiveReadings {
        readings: current,
        events,
        reviews,
    })
}
