//! Persisted batch counts retain fallible conversion instead of inventing zero.
use super::{BatchError, Status, count};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub(super) struct BatchStatusRow {
    batch_id: Uuid,
    total: i64,
    applied: i64,
    rejected: i64,
    cancelled: i64,
    pending: i64,
    unknown: i64,
    cancel_requested: bool,
}
impl TryFrom<BatchStatusRow> for Status {
    type Error = BatchError;
    fn try_from(row: BatchStatusRow) -> Result<Self, Self::Error> {
        Ok(Self {
            batch_id: row.batch_id,
            total: count(row.total)?,
            applied: count(row.applied)?,
            rejected: count(row.rejected)?,
            cancelled: count(row.cancelled)?,
            pending: count(row.pending)?,
            unknown: count(row.unknown)?,
            cancel_requested: row.cancel_requested,
        })
    }
}
