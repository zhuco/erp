//! Bounded wire query shared by hosts and server adapters. Column semantics and
//! authorization belong to the adapter; this type grants no access to any data.
use crate::{ColumnId, DateTimeParts, Filter, SortRule};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageQuery {
    pub schema: u64,
    pub offset: u64,
    pub limit: u16,
    #[serde(default)]
    pub filters: BTreeMap<ColumnId, Filter>,
    #[serde(default)]
    pub sorts: Vec<SortRule>,
    #[serde(default)]
    pub search: Option<String>,
    /// Optional bounded nested expression. Legacy quick filters remain an
    /// implicit AND alongside this expression.
    #[serde(default)]
    pub expression: Option<FilterExpression>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FilterExpression {
    Condition { column: ColumnId, filter: Filter },
    And(Vec<FilterExpression>),
    Or(Vec<FilterExpression>),
}

const MAX_EXPRESSION_DEPTH: usize = 8;
const MAX_EXPRESSION_NODES: usize = 64;

impl FilterExpression {
    pub fn validate(&self) -> Result<(), &'static str> {
        fn walk(
            node: &FilterExpression,
            depth: usize,
            count: &mut usize,
        ) -> Result<(), &'static str> {
            if depth > MAX_EXPRESSION_DEPTH {
                return Err("Filter expression is too deep");
            }
            *count += 1;
            if *count > MAX_EXPRESSION_NODES {
                return Err("Filter expression has too many nodes");
            }
            match node {
                FilterExpression::Condition { filter, .. } => validate_filter(filter),
                FilterExpression::And(items) | FilterExpression::Or(items) => {
                    if items.is_empty() {
                        return Err("Filter expression group cannot be empty");
                    }
                    for child in items {
                        walk(child, depth + 1, count)?;
                    }
                    Ok(())
                }
            }
        }
        let mut count = 0;
        walk(self, 1, &mut count)
    }
}

fn validate_filter(filter: &Filter) -> Result<(), &'static str> {
    match filter {
        Filter::Text(text) if !valid_text(text) => Err("Invalid filter text"),
        Filter::Enum(values)
            if values.is_empty() || values.len() > 100 || values.iter().any(|v| !valid_text(v)) =>
        {
            Err("Invalid enumeration filter")
        }
        Filter::Money {
            min: Some(min),
            max: Some(max),
        } if min > max => Err("Reversed amount range"),
        Filter::DateTime { start, end } => {
            for value in [start, end].into_iter().flatten() {
                civil_parts(*value)?;
            }
            if start.zip(*end).is_some_and(|(a, b)| a > b) {
                return Err("Reversed date range");
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

impl PageQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(1..=500).contains(&self.limit) || self.offset > i64::MAX as u64 - u64::from(self.limit)
        {
            return Err("Invalid page range");
        }
        if self.filters.len() > 64 || self.sorts.len() > 8 {
            return Err("Too many filters or sort columns");
        }
        if self.search.as_ref().is_some_and(|s| !valid_text(s)) {
            return Err("Search exceeds 4096 bytes or contains NUL");
        }
        if let Some(expression) = &self.expression {
            expression.validate()?;
        }
        let mut seen = BTreeSet::new();
        if self.sorts.iter().any(|sort| !seen.insert(sort.column)) {
            return Err("Duplicate sort column");
        }
        for filter in self.filters.values() {
            validate_filter(filter)?;
        }
        Ok(())
    }
}

fn valid_text(text: &str) -> bool {
    text.len() <= 4096 && !text.contains('\0')
}

/// Decode the grid's YYYYMMDDHHMMSS civil key. Adapters declare the timezone;
/// it is not a Unix timestamp and must never be bound as one.
pub fn civil_parts(key: i64) -> Result<DateTimeParts, &'static str> {
    if !(19_000_101_000_000..=21_001_231_235_959).contains(&key) {
        return Err("Invalid civil date key");
    }
    let parts = DateTimeParts {
        year: (key / 10_000_000_000) as i32,
        month: (key / 100_000_000 % 100) as u32,
        day: (key / 1_000_000 % 100) as u32,
        hour: (key / 10_000 % 100) as u32,
        minute: (key / 100 % 100) as u32,
        second: (key % 100) as u32,
    };
    parts.key()?;
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_queries_reject_invalid_ranges_duplicates_and_dates() {
        let mut query = PageQuery {
            schema: 1,
            offset: 9_900,
            limit: 100,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
            expression: None,
        };
        assert!(query.validate().is_ok());
        query.offset = u64::MAX;
        assert!(query.validate().is_err());
        query.offset = 0;
        query.sorts = vec![
            SortRule {
                column: ColumnId(0),
                descending: false
            };
            2
        ];
        assert!(query.validate().is_err());
        query.sorts.clear();
        query.filters.insert(
            ColumnId(1),
            Filter::DateTime {
                start: Some(20_250_229_000_000),
                end: None,
            },
        );
        assert!(query.validate().is_err());
        assert_eq!(
            civil_parts(20_240_229_123_456).unwrap().label(),
            "2024-02-29 12:34:56"
        );
        assert!(civil_parts(i64::MIN).is_err());
        query.filters.clear();
        query.search = Some("a\0b".into());
        assert!(query.validate().is_err());
    }

    #[test]
    fn expression_groups_and_limits_are_bounded() {
        let legacy = PageQuery {
            schema: 1,
            offset: 0,
            limit: 10,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
            expression: None,
        };
        assert!(legacy.expression.is_none());
        assert!(FilterExpression::And(vec![]).validate().is_err());
        let leaf = FilterExpression::Condition {
            column: ColumnId(1),
            filter: Filter::Text("x".into()),
        };
        assert!(
            FilterExpression::Or(vec![leaf.clone(), leaf.clone()])
                .validate()
                .is_ok()
        );
        assert!(
            FilterExpression::Or(vec![leaf.clone(); 63])
                .validate()
                .is_ok()
        );
        assert!(
            FilterExpression::Or(vec![leaf.clone(); 64])
                .validate()
                .is_err()
        );
        let mut nested = leaf;
        for _ in 1..8 {
            nested = FilterExpression::And(vec![nested]);
        }
        assert!(nested.validate().is_ok());
        assert!(FilterExpression::And(vec![nested]).validate().is_err());
    }
}
