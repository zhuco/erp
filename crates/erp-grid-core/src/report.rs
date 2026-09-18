//! Bounded, deterministic accumulators for report grouping and pivoting.
//!
//! This module deliberately owns no row identity or pagination semantics. A
//! caller decides which rows belong to the report and pushes each row once.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAX_GROUPS: usize = 4096;
pub const MAX_PIVOT_COLUMNS: usize = 64;
pub const MAX_KEY_DEPTH: usize = 8;
pub const MAX_TEXT_BYTES: usize = 4096;
pub const MAX_GROUP_KEY_BYTES: usize = 32 * 1024;
pub const MAX_TEXT_BUDGET_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportError {
    InvalidLimit,
    GroupLimit,
    PivotColumnLimit,
    KeyDepthLimit,
    TextTooLarge,
    KeyTooLarge,
    TextBudgetExceeded,
    CountOverflow,
    SumOverflow,
}

impl fmt::Display for ReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimit => "report limits must be non-zero and within hard limits",
            Self::GroupLimit => "report group limit reached",
            Self::PivotColumnLimit => "report pivot column limit reached",
            Self::KeyDepthLimit => "report group key exceeds 8 components",
            Self::TextTooLarge => "report text exceeds 4096 bytes or contains NUL",
            Self::KeyTooLarge => "report group key exceeds 32 KiB",
            Self::TextBudgetExceeded => "report text budget of 8 MiB reached",
            Self::CountOverflow => "report count overflow",
            Self::SumOverflow => "report sum overflow",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ReportError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub count: u64,
    pub sum: i128,
    pub min: Option<i128>,
    pub max: Option<i128>,
}

impl Summary {
    fn with_amount(&self, amount: i128) -> Result<Self, ReportError> {
        let count = self
            .count
            .checked_add(1)
            .ok_or(ReportError::CountOverflow)?;
        let sum = self
            .sum
            .checked_add(amount)
            .ok_or(ReportError::SumOverflow)?;
        Ok(Self {
            count,
            sum,
            min: Some(self.min.map_or(amount, |value| value.min(amount))),
            max: Some(self.max.map_or(amount, |value| value.max(amount))),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupSummary {
    pub key: Vec<String>,
    pub summary: Summary,
    pub pivot: BTreeMap<String, Summary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportResult {
    pub groups: Vec<GroupSummary>,
    pub pivot_keys: Vec<String>,
    pub total: Summary,
}

pub struct ReportAccumulator {
    max_groups: usize,
    max_pivot_columns: usize,
    groups: BTreeMap<Vec<String>, GroupSummary>,
    pivot_keys: BTreeSet<String>,
    total: Summary,
    text_bytes: usize,
}

impl ReportAccumulator {
    pub fn new(max_groups: usize, max_pivot_columns: usize) -> Result<Self, ReportError> {
        if max_groups == 0
            || max_groups > MAX_GROUPS
            || max_pivot_columns == 0
            || max_pivot_columns > MAX_PIVOT_COLUMNS
        {
            return Err(ReportError::InvalidLimit);
        }
        Ok(Self {
            max_groups,
            max_pivot_columns,
            groups: BTreeMap::new(),
            pivot_keys: BTreeSet::new(),
            total: Summary::default(),
            text_bytes: 0,
        })
    }

    /// Adds one caller-scoped input row. All validation and checked arithmetic
    /// happen before any member of the accumulator is changed.
    pub fn push(
        &mut self,
        group: Vec<String>,
        pivot: String,
        amount: i128,
    ) -> Result<(), ReportError> {
        validate_key(&group)?;
        validate_text(&pivot)?;

        let is_new_group = !self.groups.contains_key(&group);
        if is_new_group && self.groups.len() == self.max_groups {
            return Err(ReportError::GroupLimit);
        }
        let is_new_pivot = !self.pivot_keys.contains(&pivot);
        if is_new_pivot && self.pivot_keys.len() == self.max_pivot_columns {
            return Err(ReportError::PivotColumnLimit);
        }

        // A group key is retained in both the map key and GroupSummary.key.
        // A pivot key is retained globally and in each observed group bucket.
        let existing = self.groups.get(&group);
        let group_bytes = group.iter().map(String::len).sum::<usize>();
        let pivot_in_group = existing.and_then(|entry| entry.pivot.get(&pivot)).is_none();
        let added_text_bytes = group_bytes
            .checked_mul(if is_new_group { 2 } else { 0 })
            .and_then(|bytes| bytes.checked_add(if pivot_in_group { pivot.len() } else { 0 }))
            .and_then(|bytes| bytes.checked_add(if is_new_pivot { pivot.len() } else { 0 }))
            .ok_or(ReportError::TextBudgetExceeded)?;
        let next_text_bytes = self
            .text_bytes
            .checked_add(added_text_bytes)
            .ok_or(ReportError::TextBudgetExceeded)?;
        if next_text_bytes > MAX_TEXT_BUDGET_BYTES {
            return Err(ReportError::TextBudgetExceeded);
        }

        let next_total = self.total.with_amount(amount)?;
        let next_group_summary = existing
            .map_or_else(Summary::default, |entry| entry.summary.clone())
            .with_amount(amount)?;
        let next_pivot_summary = existing
            .and_then(|entry| entry.pivot.get(&pivot))
            .map_or_else(Summary::default, Clone::clone)
            .with_amount(amount)?;

        let entry = self
            .groups
            .entry(group.clone())
            .or_insert_with(|| GroupSummary {
                key: group,
                summary: Summary::default(),
                pivot: BTreeMap::new(),
            });
        entry.summary = next_group_summary;
        entry.pivot.insert(pivot.clone(), next_pivot_summary);
        self.total = next_total;
        self.pivot_keys.insert(pivot);
        self.text_bytes = next_text_bytes;
        Ok(())
    }

    pub fn finish(&self) -> ReportResult {
        ReportResult {
            groups: self.groups.values().cloned().collect(),
            pivot_keys: self.pivot_keys.iter().cloned().collect(),
            total: self.total.clone(),
        }
    }
}

fn validate_text(text: &str) -> Result<(), ReportError> {
    if text.len() > MAX_TEXT_BYTES || text.contains('\0') {
        return Err(ReportError::TextTooLarge);
    }
    Ok(())
}

fn validate_key(key: &[String]) -> Result<(), ReportError> {
    if key.len() > MAX_KEY_DEPTH {
        return Err(ReportError::KeyDepthLimit);
    }
    let mut bytes = 0usize;
    for part in key {
        validate_text(part)?;
        bytes = bytes
            .checked_add(part.len())
            .ok_or(ReportError::KeyTooLarge)?;
    }
    if bytes > MAX_GROUP_KEY_BYTES {
        return Err(ReportError::KeyTooLarge);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_total_and_missing_pivot_buckets_are_deterministic() {
        let mut report = ReportAccumulator::new(4, 4).unwrap();
        report.push(vec!["b".into()], "x".into(), 2).unwrap();
        report.push(vec!["a".into()], "y".into(), -3).unwrap();
        report.push(vec!["b".into()], "x".into(), 5).unwrap();
        let result = report.finish();
        assert_eq!(result.pivot_keys, ["x", "y"]);
        assert_eq!(result.groups[0].key, ["a"]);
        assert_eq!(
            result.groups[0].summary,
            Summary {
                count: 1,
                sum: -3,
                min: Some(-3),
                max: Some(-3)
            }
        );
        assert_eq!(
            result.groups[1].summary,
            Summary {
                count: 2,
                sum: 7,
                min: Some(2),
                max: Some(5)
            }
        );
        assert!(!result.groups[0].pivot.contains_key("x"));
        assert_eq!(
            result.total,
            Summary {
                count: 3,
                sum: 4,
                min: Some(-3),
                max: Some(5)
            }
        );
    }

    #[test]
    fn overflow_and_caps_are_atomic() {
        let mut report = ReportAccumulator::new(1, 1).unwrap();
        report
            .push(vec!["a".into()], "x".into(), i128::MAX)
            .unwrap();
        let before = report.finish();
        assert_eq!(
            report.push(vec!["a".into()], "x".into(), 1),
            Err(ReportError::SumOverflow)
        );
        assert_eq!(report.finish(), before);
        assert_eq!(
            report.push(vec!["b".into()], "x".into(), 1),
            Err(ReportError::GroupLimit)
        );
        assert_eq!(
            report.push(vec!["a".into()], "y".into(), 1),
            Err(ReportError::PivotColumnLimit)
        );
        assert_eq!(report.finish(), before);
    }

    #[test]
    fn input_limits_are_rejected_without_mutation() {
        let mut report = ReportAccumulator::new(2, 2).unwrap();
        let before = report.finish();
        assert_eq!(
            report.push(vec!["a".repeat(MAX_TEXT_BYTES + 1)], "x".into(), 1),
            Err(ReportError::TextTooLarge)
        );
        assert_eq!(
            report.push(vec!["a".into(); MAX_KEY_DEPTH + 1], "x".into(), 1),
            Err(ReportError::KeyDepthLimit)
        );
        assert_eq!(report.finish(), before);
    }

    #[test]
    fn text_budget_rejection_is_atomic() {
        let mut report = ReportAccumulator::new(2, 2).unwrap();
        report.text_bytes = MAX_TEXT_BUDGET_BYTES - 1;
        let before = report.finish();
        assert_eq!(
            report.push(vec!["group".into()], "pivot".into(), 1),
            Err(ReportError::TextBudgetExceeded)
        );
        assert_eq!(report.finish(), before);
        assert_eq!(report.text_bytes, MAX_TEXT_BUDGET_BYTES - 1);
    }
}
