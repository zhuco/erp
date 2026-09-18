//! Pure, bounded snapshots of unsubmitted remote-grid edit intent.
//! Drafts never contain executable commands or operation identities.
use erp_grid_core::{ColumnId, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_CELLS: usize = 10_000;
const MAX_CELL_BYTES: usize = 4096;
const MAX_VERSION_BYTES: usize = 128;
const TTL_SECONDS: u64 = 7 * 24 * 60 * 60;
// Conservative lower bounds for JSON field names, punctuation and UUID text.
// String escaping can only make the final encoding larger.
const SNAPSHOT_FIXED_BYTES: usize = 128;
const CELL_FIXED_BYTES: usize = 96;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftScope {
    pub server: String,
    pub tenant: RowId,
    pub actor: RowId,
    pub grid: String,
    pub schema: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftCell {
    pub row: RowId,
    pub column: ColumnId,
    pub before: String,
    pub after: String,
    pub expected_version: String,
    pub parent: Option<RowId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftSnapshot {
    pub scope: DraftScope,
    pub saved_at: u64,
    pub expires_at: u64,
    pub rows: Vec<DraftCell>,
}

pub(crate) fn encode(scope: &DraftScope, now: u64, rows: &[DraftCell]) -> Result<Vec<u8>, String> {
    validate_scope(scope)?;
    validate_cells(scope, rows)?;
    let snapshot = DraftSnapshot {
        scope: scope.clone(),
        saved_at: now,
        expires_at: now
            .checked_add(TTL_SECONDS)
            .ok_or("Draft expiry overflow")?,
        rows: rows.to_vec(),
    };
    let bytes = serde_json::to_vec(&snapshot).map_err(|_| "Cannot encode edit draft")?;
    if bytes.len() > MAX_BYTES {
        return Err("Edit draft exceeds 1 MiB".into());
    }
    Ok(bytes)
}

pub(crate) fn decode(scope: &DraftScope, now: u64, bytes: &[u8]) -> Result<DraftSnapshot, String> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("Edit draft must contain at most 1 MiB".into());
    }
    let snapshot: DraftSnapshot =
        serde_json::from_slice(bytes).map_err(|_| "Cannot decode edit draft")?;
    if &snapshot.scope != scope {
        return Err("Edit draft belongs to another authenticated grid".into());
    }
    validate(&snapshot, now, true)?;
    Ok(snapshot)
}

fn validate(snapshot: &DraftSnapshot, now: u64, require_unexpired: bool) -> Result<(), String> {
    validate_scope(&snapshot.scope)?;
    validate_cells(&snapshot.scope, &snapshot.rows)?;
    if snapshot
        .saved_at
        .checked_add(TTL_SECONDS)
        .is_none_or(|expires| expires != snapshot.expires_at)
        || snapshot.saved_at > now
    {
        return Err("Edit draft has invalid timestamps".into());
    }
    if require_unexpired && now >= snapshot.expires_at {
        return Err("Edit draft has expired".into());
    }
    Ok(())
}

fn validate_cells(scope: &DraftScope, rows: &[DraftCell]) -> Result<(), String> {
    if rows.is_empty() || rows.len() > MAX_CELLS {
        return Err("Edit draft must contain 1–10,000 cells".into());
    }
    let mut budget = SNAPSHOT_FIXED_BYTES
        .checked_add(scope.server.len())
        .and_then(|bytes| bytes.checked_add(scope.grid.len()))
        .ok_or("Edit draft byte count overflow")?;
    let mut identities = BTreeSet::new();
    let mut row_versions = BTreeMap::new();
    for cell in rows {
        budget = budget
            .checked_add(CELL_FIXED_BYTES)
            .and_then(|bytes| bytes.checked_add(cell.before.len()))
            .and_then(|bytes| bytes.checked_add(cell.after.len()))
            .and_then(|bytes| bytes.checked_add(cell.expected_version.len()))
            .ok_or("Edit draft byte count overflow")?;
        if budget > MAX_BYTES {
            return Err("Edit draft exceeds 1 MiB".into());
        }
        if cell.row.0 == 0
            || cell.parent.is_some_and(|parent| parent.0 == 0)
            || !identities.insert((cell.row, cell.column))
        {
            return Err("Edit draft contains an invalid or duplicate cell identity".into());
        }
        if cell.expected_version.trim().is_empty()
            || cell.expected_version.len() > MAX_VERSION_BYTES
            || cell.expected_version.contains('\0')
        {
            return Err("Edit draft cell has an invalid expected version".into());
        }
        if cell.before.len() > MAX_CELL_BYTES
            || cell.after.len() > MAX_CELL_BYTES
            || cell.before.contains('\0')
            || cell.after.contains('\0')
        {
            return Err("Edit draft cell value exceeds limits".into());
        }
        if cell.before == cell.after {
            return Err("Edit draft contains an unchanged cell".into());
        }
        let identity = (cell.expected_version.as_str(), cell.parent);
        if row_versions
            .insert(cell.row, identity)
            .is_some_and(|previous| previous != identity)
        {
            return Err("Edit draft has inconsistent row version or parent identity".into());
        }
    }
    Ok(())
}

fn validate_scope(scope: &DraftScope) -> Result<(), String> {
    let grid_valid = !scope.grid.is_empty()
        && scope.grid.len() <= 64
        && scope
            .grid
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
    if scope.server.trim().is_empty()
        || scope.server.len() > 4096
        || scope.server.contains('\0')
        || scope.tenant.0 == 0
        || scope.actor.0 == 0
        || !grid_valid
        || scope.schema == 0
    {
        return Err("Invalid authenticated edit draft scope".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(actor: u128) -> DraftScope {
        DraftScope {
            server: "https://erp.example.test/api".into(),
            tenant: RowId(1),
            actor: RowId(actor),
            grid: "products".into(),
            schema: 1,
        }
    }

    fn cell(row: u128, column: u64) -> DraftCell {
        DraftCell {
            row: RowId(row),
            column: ColumnId(column),
            before: "Old".into(),
            after: "Wanted".into(),
            expected_version: "2026-09-10T00:00:00Z".into(),
            parent: None,
        }
    }

    #[test]
    fn edit_draft_round_trip_contains_only_reviewable_cell_intent() {
        let scope = scope(2);
        let mut sku = cell(10, 4);
        sku.parent = Some(RowId(9));
        let bytes = encode(&scope, 100, &[sku.clone()]).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(!text.contains("operation_id"));
        assert!(!text.contains("action"));
        let restored = decode(&scope, 101, &bytes).unwrap();
        assert_eq!(restored.rows, vec![sku]);
        assert_eq!(restored.saved_at, 100);
        assert_eq!(restored.expires_at, 100 + TTL_SECONDS);
    }

    #[test]
    fn edit_draft_rejects_wrong_scope_expiry_and_unknown_fields() {
        let other_scope = scope(3);
        let scope = scope(2);
        let bytes = encode(&scope, 100, &[cell(10, 1)]).unwrap();
        assert!(decode(&other_scope, 101, &bytes).is_err());
        assert!(decode(&scope, 100 + TTL_SECONDS, &bytes).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["replay"] = serde_json::json!(true);
        assert!(decode(&scope, 101, &serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn edit_draft_rejects_duplicate_nil_missing_version_and_unchanged_cells() {
        let scope = scope(2);
        let duplicate = cell(10, 1);
        assert!(encode(&scope, 100, &[duplicate.clone(), duplicate]).is_err());
        let mut invalid = cell(0, 1);
        assert!(encode(&scope, 100, &[invalid.clone()]).is_err());
        invalid.row = RowId(10);
        invalid.expected_version.clear();
        assert!(encode(&scope, 100, &[invalid.clone()]).is_err());
        invalid.expected_version = "v1".into();
        invalid.after = invalid.before.clone();
        assert!(encode(&scope, 100, &[invalid]).is_err());
    }

    #[test]
    fn edit_draft_enforces_cell_count_value_and_json_byte_bounds() {
        let scope = scope(2);
        assert!(encode(&scope, 100, &[]).is_err());
        let too_many = (0..=MAX_CELLS)
            .map(|index| cell(index as u128 + 1, 1))
            .collect::<Vec<_>>();
        assert!(encode(&scope, 100, &too_many).is_err());
        let mut oversized_cell = cell(1, 1);
        oversized_cell.after = "x".repeat(MAX_CELL_BYTES + 1);
        assert!(encode(&scope, 100, &[oversized_cell]).is_err());
        let large = (0..6_000)
            .map(|index| {
                let mut value = cell(index + 1, 1);
                value.after = "x".repeat(64);
                value
            })
            .collect::<Vec<_>>();
        assert!(encode(&scope, 100, &large).is_err());
    }
}
