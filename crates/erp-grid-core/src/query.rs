use crate::{ColumnId, money_minor};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ValueKind {
    #[default]
    Text,
    Integer,
    Money,
    DateTime,
}

/// Spreadsheet-safe text without changing valid typed numbers (including negatives).
pub fn spreadsheet_value(value: &str, kind: ValueKind) -> std::borrow::Cow<'_, str> {
    let numeric = match kind {
        ValueKind::Integer => value.parse::<i64>().is_ok(),
        ValueKind::Money => money_minor(value).is_ok_and(|value| value.is_some()),
        _ => false,
    };
    if !numeric && value.trim_start().starts_with(['=', '+', '-', '@']) {
        std::borrow::Cow::Owned(format!("'{value}"))
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SortRule {
    pub column: ColumnId,
    pub descending: bool,
}

pub fn compare_values(left: &str, right: &str, kind: ValueKind, descending: bool) -> Ordering {
    // Empty values remain last in both directions.
    if left.is_empty() || right.is_empty() {
        return left.is_empty().cmp(&right.is_empty());
    }
    let order = match kind {
        ValueKind::Integer => left.parse::<i64>().ok().cmp(&right.parse::<i64>().ok()),
        ValueKind::Money => money_minor(left).ok().cmp(&money_minor(right).ok()),
        ValueKind::DateTime => left.cmp(right), // Canonical ISO civil time in a declared timezone.
        ValueKind::Text => left.to_lowercase().cmp(&right.to_lowercase()),
    };
    if descending { order.reverse() } else { order }
}

/// Stable bottom-up merge sort. Each advance emits at most `budget` indices.
pub struct SortJob {
    pub indices: Vec<usize>,
    scratch: Vec<usize>,
    width: usize,
    base: usize,
    left: usize,
    right: usize,
    end: usize,
    middle: usize,
    output: usize,
}
impl SortJob {
    pub fn new(indices: Vec<usize>) -> Self {
        let len = indices.len();
        Self {
            indices,
            scratch: vec![0; len],
            width: 1,
            base: 0,
            left: 0,
            right: 0,
            end: 0,
            middle: 0,
            output: 0,
        }
    }
    pub fn advance(
        &mut self,
        mut budget: usize,
        compare: impl Fn(usize, usize) -> Ordering,
    ) -> bool {
        let len = self.indices.len();
        while self.width < len && budget > 0 {
            if self.output == self.end {
                if self.base == len {
                    std::mem::swap(&mut self.indices, &mut self.scratch);
                    self.width = self.width.saturating_mul(2);
                    self.base = 0;
                    if self.width >= len {
                        break;
                    }
                }
                self.left = self.base;
                self.middle = (self.base + self.width).min(len);
                self.right = self.middle;
                self.end = (self.middle + self.width).min(len);
                self.output = self.base;
                self.base = self.end;
            }
            let take_left = self.left < self.middle
                && (self.right == self.end
                    || compare(self.indices[self.left], self.indices[self.right])
                        != Ordering::Greater);
            let source = if take_left {
                let i = self.left;
                self.left += 1;
                i
            } else {
                let i = self.right;
                self.right += 1;
                i
            };
            self.scratch[self.output] = self.indices[source];
            self.output += 1;
            budget -= 1;
        }
        self.width >= len
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooperative_sort_is_stable_and_numeric_and_keeps_empty_last() {
        let values = ["10", "2", "2", "", "-1"];
        let mut job = SortJob::new((0..values.len()).collect());
        let mut frames = 0;
        while !job.advance(1, |a, b| {
            compare_values(values[a], values[b], ValueKind::Integer, false)
        }) {
            frames += 1;
            assert!(frames < 100);
        }
        assert_eq!(job.indices, [4, 1, 2, 0, 3]);
        assert!(frames > 5);
        assert_eq!(
            compare_values("", "3", ValueKind::Money, true),
            Ordering::Greater
        );
        assert_eq!(
            compare_values("10.00", "2.00", ValueKind::Money, false),
            Ordering::Greater
        );
    }
}
