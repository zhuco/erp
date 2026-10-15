use crate::{ColumnId, money_minor};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ValueKind {
    #[default]
    Text,
    Integer,
    Money,
    /// Exact decimal text; bounded to eight fractional digits and i128 intermediates.
    Decimal {
        scale: u8,
    },
    DateTime,
}

/// Spreadsheet-safe text without changing valid typed numbers (including negatives).
pub fn spreadsheet_value(value: &str, kind: ValueKind) -> std::borrow::Cow<'_, str> {
    let numeric = match kind {
        ValueKind::Integer => value.parse::<i64>().is_ok(),
        ValueKind::Money => money_minor(value).is_ok_and(|value| value.is_some()),
        ValueKind::Decimal { scale } => {
            decimal_scaled(value, scale).is_ok_and(|value| value.is_some())
        }
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
        ValueKind::Decimal { scale } => decimal_scaled(left, scale)
            .ok()
            .cmp(&decimal_scaled(right, scale).ok()),
        ValueKind::DateTime => left.cmp(right), // Canonical ISO civil time in a declared timezone.
        ValueKind::Text => left.to_lowercase().cmp(&right.to_lowercase()),
    };
    if descending { order.reverse() } else { order }
}

/// Parse plain decimal text without a floating-point or i64 narrowing step.
pub fn decimal_scaled(text: &str, scale: u8) -> Result<Option<i128>, &'static str> {
    if scale > 8 {
        return Err("Decimal scale exceeds eight digits");
    }
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if text.len() > 50 {
        return Err("Decimal is outside the supported range");
    }
    let negative = text.starts_with('-');
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || (unsigned.contains('.') && fraction.is_empty())
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || !fraction.bytes().all(|c| c.is_ascii_digit())
        || fraction.len() > usize::from(scale)
    {
        return Err("Invalid plain decimal");
    }
    let digits = format!(
        "{whole}{fraction}{}",
        "0".repeat(usize::from(scale) - fraction.len())
    );
    let absolute = digits.parse::<u128>().map_err(|_| "Decimal overflow")?;
    let value = if negative && absolute == i128::MAX as u128 + 1 {
        i128::MIN
    } else {
        let value = i128::try_from(absolute).map_err(|_| "Decimal overflow")?;
        if negative { -value } else { value }
    };
    Ok(Some(value))
}

/// Format a scaled integer without a power-of-ten overflow or float conversion.
pub fn decimal_text(value: i128, scale: u8) -> String {
    let mut digits = value.unsigned_abs().to_string();
    let scale = usize::from(scale);
    if scale != 0 {
        if digits.len() <= scale {
            digits.insert_str(0, &"0".repeat(scale + 1 - digits.len()));
        }
        digits.insert(digits.len() - scale, '.');
        while digits.ends_with('0') {
            digits.pop();
        }
        if digits.ends_with('.') {
            digits.pop();
        }
    }
    if value < 0 {
        digits.insert(0, '-');
    }
    digits
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
    fn decimal_values_sort_and_export_without_losing_precision() {
        let kind = ValueKind::Decimal { scale: 6 };
        assert_eq!(
            compare_values("10.01", "2.999999", kind, false),
            Ordering::Greater
        );
        assert_eq!(
            compare_values("-0.000002", "-0.000001", kind, false),
            Ordering::Less
        );
        assert_eq!(spreadsheet_value("-0.000001", kind), "-0.000001");
        assert_eq!(spreadsheet_value("=1+2", kind), "'=1+2");
        assert_eq!(
            decimal_scaled("18446744073709.551614", 6),
            Ok(Some(18_446_744_073_709_551_614))
        );
        assert_eq!(
            decimal_text(18_446_744_073_709_551_614, 6),
            "18446744073709.551614"
        );
        assert_eq!(
            decimal_scaled(&decimal_text(i128::MIN, 8), 8),
            Ok(Some(i128::MIN))
        );
        for invalid in ["1e2", "0.0000001", "NaN", "1.", ".1", "--1"] {
            assert!(decimal_scaled(invalid, 6).is_err(), "{invalid}");
        }
    }
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
