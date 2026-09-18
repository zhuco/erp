//! Versioned, idempotent SKU grid mutations with an atomic durable receipt.
use crate::product_actions::ActionError;
use chrono::{DateTime, Utc};
use erp_grid_core::{
    ColumnId, RowId,
    actions::{ActionCommand, ActionOutcome, ActionReceipt},
    money_minor,
};
use erp_types::{TenantId, UserId};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

pub const UPDATE: &str = "skus.update";
const SKU_COLUMNS: &str = "sku_id, tenant_id, product_id, sku_code, barcode, name, sku_kind, cost_cent, is_active, status, created_at, updated_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
struct SkuActionRow {
    sku_id: Uuid,
    tenant_id: Uuid,
    product_id: Uuid,
    sku_code: String,
    barcode: Option<String>,
    name: Option<String>,
    sku_kind: String,
    cost_cent: Option<i64>,
    is_active: bool,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub fn validate(command: &ActionCommand) -> Result<(), ActionError> {
    command.validate().map_err(ActionError::Invalid)?;
    if command.action != UPDATE || command.expected_version.is_none() || command.values.is_empty() {
        return Err(ActionError::Invalid(
            "SKU update requires a version and editable columns",
        ));
    }
    DateTime::parse_from_rfc3339(command.expected_version.as_deref().unwrap_or_default())
        .map_err(|_| ActionError::Invalid("Invalid SKU version"))?;
    for (column, raw) in &command.values {
        match column.0 {
            0 if !raw.trim().is_empty() && raw.trim().len() <= 255 => {}
            0 => {
                return Err(ActionError::Invalid(
                    "SKU code is required and must not exceed 255 bytes",
                ));
            }
            1 if raw.trim().len() <= 255 => {}
            1 => {
                return Err(ActionError::Invalid(
                    "SKU barcode must not exceed 255 bytes",
                ));
            }
            2 if raw.trim().len() <= 500 => {}
            2 => return Err(ActionError::Invalid("SKU name must not exceed 500 bytes")),
            4 => {
                let value = money_minor(raw).map_err(ActionError::Invalid)?;
                if value.is_some_and(|value| value < 0) {
                    return Err(ActionError::Invalid("SKU cost cannot be negative"));
                }
            }
            5 if raw == "true" || raw == "false" => {}
            5 => {
                return Err(ActionError::Invalid(
                    "SKU active value must be true or false",
                ));
            }
            _ => return Err(ActionError::Invalid("SKU column is read-only")),
        }
    }
    Ok(())
}

pub async fn receipt(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    operation: RowId,
) -> Result<Option<ActionReceipt>, ActionError> {
    let row: Option<(Value, Option<Value>)> = sqlx::query_as("select command, receipt from grid_sku_operations where tenant_id=$1 and actor_user_id=$2 and operation_id=$3")
        .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(Uuid::from_u128(operation.0)).fetch_optional(pool).await?;
    row.and_then(|(command, receipt)| receipt.map(|receipt| (command, receipt)))
        .map(|(command, receipt)| decode_receipt(command, receipt))
        .transpose()
}

fn decode_receipt(command: Value, value: Value) -> Result<ActionReceipt, ActionError> {
    let command: ActionCommand = serde_json::from_value(command)?;
    command
        .validate()
        .map_err(|_| ActionError::ReceiptMismatch)?;
    let receipt: ActionReceipt = serde_json::from_value(value)?;
    if receipt.operation_id != command.operation_id
        || receipt.source != command.source
        || receipt.command_fingerprint != command.fingerprint()
    {
        return Err(ActionError::ReceiptMismatch);
    }
    Ok(receipt)
}

fn rejected(code: &str, message: &str) -> ActionOutcome {
    ActionOutcome::Rejected {
        code: code.into(),
        message: message.into(),
    }
}

fn format_cost(cost: Option<i64>) -> String {
    cost.map(|cent| {
        let magnitude = cent.unsigned_abs();
        format!(
            "{}{}.{:02}",
            if cent < 0 { "-" } else { "" },
            magnitude / 100,
            magnitude % 100
        )
    })
    .unwrap_or_default()
}

fn applied(row: SkuActionRow, effective_cost: Option<i64>) -> ActionOutcome {
    ActionOutcome::Applied {
        row: RowId(row.sku_id.as_u128()),
        version: row.updated_at.to_rfc3339(),
        values: [
            (ColumnId(0), row.sku_code),
            (ColumnId(1), row.barcode.unwrap_or_default()),
            (ColumnId(2), row.name.unwrap_or_default()),
            (ColumnId(3), row.sku_kind),
            (ColumnId(4), format_cost(effective_cost)),
            (ColumnId(5), row.is_active.to_string()),
            (ColumnId(6), row.status),
            (ColumnId(7), row.created_at.to_rfc3339()),
            (ColumnId(8), row.updated_at.to_rfc3339()),
        ]
        .into(),
    }
}

pub async fn execute(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    command: &ActionCommand,
) -> Result<ActionReceipt, ActionError> {
    validate(command)?;
    let encoded = serde_json::to_value(command)?;
    let operation = Uuid::from_u128(command.operation_id.0);
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout='10s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local lock_timeout='3s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("insert into grid_sku_operations(tenant_id,actor_user_id,operation_id,command) values($1,$2,$3,$4) on conflict do nothing")
        .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(operation).bind(&encoded).execute(&mut *tx).await?;
    let (stored, previous): (Value, Option<Value>) = sqlx::query_as("select command,receipt from grid_sku_operations where tenant_id=$1 and actor_user_id=$2 and operation_id=$3 for update")
        .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(operation).fetch_one(&mut *tx).await?;
    if stored != encoded {
        return Err(ActionError::IdentityConflict);
    }
    if let Some(previous) = previous {
        let receipt = decode_receipt(stored, previous)?;
        tx.commit().await?;
        return Ok(receipt);
    }
    let source: Option<SkuActionRow> = sqlx::query_as(&format!("select {SKU_COLUMNS} from skus where tenant_id=$1 and sku_id=$2 and status <> 'DELETED' for update"))
        .bind(Uuid::from(tenant)).bind(Uuid::from_u128(command.source.0)).fetch_optional(&mut *tx).await?;
    let before = source.as_ref().map(serde_json::to_value).transpose()?;
    sqlx::query("savepoint grid_sku_action")
        .execute(&mut *tx)
        .await?;
    let outcome = match source {
        None => Ok(rejected(
            "NOT_FOUND",
            "SKU was removed or is outside this merchant",
        )),
        Some(ref row)
            if DateTime::parse_from_rfc3339(
                command.expected_version.as_deref().unwrap_or_default(),
            )
            .map(|v| v.with_timezone(&Utc) != row.updated_at)
            .unwrap_or(true) =>
        {
            Ok(rejected(
                "VERSION_CONFLICT",
                "SKU changed; refresh before saving",
            ))
        }
        Some(ref row) if row.sku_kind == "KIT" && command.values.contains_key(&ColumnId(4)) => {
            Ok(rejected(
                "KIT_COST_READ_ONLY",
                "KIT cost is derived from its BOM and cannot be edited",
            ))
        }
        Some(_) => update(&mut tx, tenant, command).await,
    };
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(ActionError::Database(sqlx::Error::Database(ref error)))
            if error.code().as_deref() == Some("23505") =>
        {
            rejected("DUPLICATE_CODE", "SKU code already exists")
        }
        Err(error) => return Err(error),
    };
    if matches!(outcome, ActionOutcome::Rejected { .. }) {
        sqlx::query("rollback to savepoint grid_sku_action")
            .execute(&mut *tx)
            .await?;
    }
    let result = ActionReceipt {
        operation_id: command.operation_id,
        source: command.source,
        command_fingerprint: command.fingerprint(),
        outcome,
    };
    if let ActionOutcome::Applied { row, .. } = &result.outcome {
        let metadata = serde_json::json!({"operation_id":command.operation_id,"source":command.source,"result":result});
        sqlx::query("insert into operation_logs(tenant_id,actor_type,actor_user_id,action,ref_type,ref_id,metadata) values($1,'USER',$2,$3,'SKU',$4,$5)")
            .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(&command.action).bind(row.to_uuid()).bind(&metadata).execute(&mut *tx).await?;
        sqlx::query("insert into audit_logs(tenant_id,user_id,action,entity_type,entity_id,before_json,after_json) values($1,$2,$3,'SKU',$4,$5,$6)")
            .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(&command.action).bind(row.to_uuid()).bind(before).bind(&metadata).execute(&mut *tx).await?;
    }
    sqlx::query("update grid_sku_operations set receipt=$4 where tenant_id=$1 and actor_user_id=$2 and operation_id=$3")
        .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(operation).bind(serde_json::to_value(&result)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(result)
}

async fn update(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: &ActionCommand,
) -> Result<ActionOutcome, ActionError> {
    let mut qb = QueryBuilder::<Postgres>::new("update skus set ");
    for (column, raw) in &command.values {
        match column.0 {
            0 => {
                qb.push("sku_code=").push_bind(raw.trim().to_owned());
            }
            1 => {
                qb.push("barcode=")
                    .push_bind((!raw.trim().is_empty()).then(|| raw.trim().to_owned()));
            }
            2 => {
                qb.push("name=")
                    .push_bind((!raw.trim().is_empty()).then(|| raw.trim().to_owned()));
            }
            4 => {
                qb.push("cost_cent=")
                    .push_bind(money_minor(raw).map_err(ActionError::Invalid)?);
            }
            5 => {
                let active = raw == "true";
                qb.push("is_active=")
                    .push_bind(active)
                    .push(", status=")
                    .push_bind(if active { "ACTIVE" } else { "INACTIVE" });
            }
            _ => return Err(ActionError::Invalid("Unsupported SKU column")),
        }
        qb.push(", ");
    }
    qb.push("updated_at=greatest(clock_timestamp(),updated_at+interval '1 microsecond') where tenant_id=").push_bind(Uuid::from(tenant));
    qb.push(" and sku_id=")
        .push_bind(Uuid::from_u128(command.source.0));
    qb.push(" returning ").push(SKU_COLUMNS);
    let row: SkuActionRow = qb.build_query_as().fetch_one(&mut **tx).await?;
    let effective_cost = if row.sku_kind == "KIT" {
        Some(sqlx::query_scalar("select coalesce(sum(b.qty * coalesce(c.cost_cent,0)),0)::bigint from sku_boms b join skus c on c.sku_id=b.component_sku_id and c.tenant_id=b.tenant_id where b.tenant_id=$1 and b.bundle_sku_id=$2")
            .bind(Uuid::from(tenant)).bind(row.sku_id).fetch_one(&mut **tx).await?)
    } else {
        row.cost_cent
    };
    Ok(applied(row, effective_cost))
}

#[cfg(test)]
mod tests;
