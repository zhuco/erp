use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FilterKind {
    #[default]
    Text,
    /// The column remains visible but has no quick-search or query condition.
    None,
    Money,
    DateTime,
    Enum(Vec<String>),
}

impl FilterKind {
    /// Values must cover the whole current category, not merely the loaded page.
    pub fn categorical(values: impl IntoIterator<Item = String>, complete: bool) -> Self {
        if !complete {
            return Self::Text;
        }
        let mut distinct = BTreeSet::new();
        for value in values {
            distinct.insert(value);
            if distinct.len() > 100 {
                return Self::Text;
            }
        }
        Self::Enum(distinct.into_iter().collect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FilterDraft {
    Text(String),
    Money {
        min: String,
        max: String,
    },
    DateTime {
        start: Option<DateTimeParts>,
        end: Option<DateTimeParts>,
    },
    Enum(BTreeSet<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Filter {
    Text(String),
    Money {
        min: Option<i64>,
        max: Option<i64>,
    },
    DateTime {
        start: Option<i64>,
        end: Option<i64>,
    },
    Enum(BTreeSet<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DateTimeParts {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Default for DateTimeParts {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

impl DateTimeParts {
    /// Parse the canonical civil date/time used by grid fields, without timezone conversion.
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        if !value.is_ascii() || !matches!(value.len(), 10 | 19) {
            return Err("Use YYYY-MM-DD or YYYY-MM-DD HH:MM:SS");
        }
        let bytes = value.as_bytes();
        if bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7 | 10 | 13 | 16) && !byte.is_ascii_digit())
        {
            return Err("Invalid date part");
        }
        if bytes[4] != b'-'
            || bytes[7] != b'-'
            || (value.len() == 19 && (bytes[10] != b' ' || bytes[13] != b':' || bytes[16] != b':'))
        {
            return Err("Invalid date format");
        }
        let part = |start, end| {
            value[start..end]
                .parse::<u32>()
                .map_err(|_| "Invalid date part")
        };
        let date = Self {
            year: part(0, 4)? as i32,
            month: part(5, 7)?,
            day: part(8, 10)?,
            hour: if value.len() == 19 { part(11, 13)? } else { 0 },
            minute: if value.len() == 19 { part(14, 16)? } else { 0 },
            second: if value.len() == 19 { part(17, 19)? } else { 0 },
        };
        date.key()?;
        Ok(date)
    }
    pub fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        }
    }
    /// Civil-time ordering in the column's declared timezone (fixture uses UTC).
    pub fn key(self) -> Result<i64, &'static str> {
        if !(1900..=2100).contains(&self.year)
            || self.day == 0
            || self.day > Self::days_in_month(self.year, self.month)
            || self.hour > 23
            || self.minute > 59
            || self.second > 59
        {
            return Err("Invalid date or time");
        }
        Ok(i64::from(self.year) * 10_000_000_000
            + i64::from(self.month) * 100_000_000
            + i64::from(self.day) * 1_000_000
            + i64::from(self.hour) * 10_000
            + i64::from(self.minute) * 100
            + i64::from(self.second))
    }
    pub fn label(self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Exact two-decimal money, with checked arithmetic. Empty means an open bound.
pub fn money_minor(value: &str) -> Result<Option<i64>, &'static str> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let negative = value.starts_with('-');
    let unsigned = value.strip_prefix(['-', '+']).unwrap_or(value);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 2
    {
        return Err("Amount requires at most 2 decimal places");
    }
    let whole: i64 = whole.parse().map_err(|_| "Amount overflow")?;
    let fraction: i64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().map_err(|_| "Invalid amount")?
    };
    let value = whole
        .checked_mul(100)
        .and_then(|v| {
            v.checked_add(
                if unsigned.split_once('.').is_some_and(|(_, f)| f.len() == 1) {
                    fraction * 10
                } else {
                    fraction
                },
            )
        })
        .ok_or("Amount overflow")?;
    Ok(Some(if negative { -value } else { value }))
}

impl FilterDraft {
    pub fn for_kind(kind: &FilterKind) -> Self {
        match kind {
            FilterKind::Text => Self::Text(String::new()),
            FilterKind::None => Self::Text(String::new()),
            FilterKind::Money => Self::Money {
                min: String::new(),
                max: String::new(),
            },
            FilterKind::DateTime => Self::DateTime {
                start: None,
                end: None,
            },
            FilterKind::Enum(_) => Self::Enum(BTreeSet::new()),
        }
    }
    pub fn compile(&self, kind: &FilterKind) -> Result<Option<Filter>, &'static str> {
        match (self, kind) {
            (Self::Text(text), FilterKind::None) if text.trim().is_empty() => Ok(None),
            (Self::Enum(values), FilterKind::None) if values.is_empty() => Ok(None),
            (Self::Money { min, max }, FilterKind::None)
                if min.trim().is_empty() && max.trim().is_empty() =>
            {
                Ok(None)
            }
            (Self::DateTime { start, end }, FilterKind::None)
                if start.is_none() && end.is_none() =>
            {
                Ok(None)
            }
            (_, FilterKind::None) => Err("Column does not support filtering"),
            (Self::Text(text), FilterKind::Text) => {
                Ok((!text.trim().is_empty()).then(|| Filter::Text(text.trim().to_lowercase())))
            }
            (Self::Money { min, max }, FilterKind::Money) => {
                let min = money_minor(min)?;
                let max = money_minor(max)?;
                if min.zip(max).is_some_and(|(a, b)| a > b) {
                    return Err("Minimum exceeds maximum");
                }
                Ok((min.is_some() || max.is_some()).then_some(Filter::Money { min, max }))
            }
            (Self::DateTime { start, end }, FilterKind::DateTime) => {
                let start = start.map(DateTimeParts::key).transpose()?;
                let end = end.map(DateTimeParts::key).transpose()?;
                if start.zip(end).is_some_and(|(a, b)| a > b) {
                    return Err("Start exceeds end");
                }
                Ok((start.is_some() || end.is_some()).then_some(Filter::DateTime { start, end }))
            }
            (Self::Enum(selected), FilterKind::Enum(options)) => {
                if selected.iter().any(|v| !options.contains(v)) {
                    return Err("Selection is outside current category");
                }
                Ok((!selected.is_empty()).then(|| Filter::Enum(selected.clone())))
            }
            _ => Err("Filter schema changed; clear the old condition"),
        }
    }
}

impl Filter {
    pub fn matches_text(&self, value: &str) -> bool {
        match self {
            Self::Text(term) => value.to_lowercase().contains(term),
            Self::Enum(values) => values.contains(value),
            _ => false,
        }
    }
    pub fn matches_number(&self, value: i64) -> bool {
        match self {
            Self::Money { min, max } => {
                min.is_none_or(|min| value >= min) && max.is_none_or(|max| value <= max)
            }
            Self::DateTime { start, end } => {
                start.is_none_or(|min| value >= min) && end.is_none_or(|max| value <= max)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precise_amounts_and_invalid_bounds() {
        assert_eq!(money_minor("12.30"), Ok(Some(1230)));
        assert_eq!(money_minor("-0.05"), Ok(Some(-5)));
        assert!(money_minor("NaN").is_err());
        assert!(money_minor("1.001").is_err());
        assert!(money_minor("9999999999999999999").is_err());
        assert!(
            FilterDraft::Money {
                min: "2".into(),
                max: "1".into()
            }
            .compile(&FilterKind::Money)
            .is_err()
        );
    }
    #[test]
    fn enum_threshold_uses_complete_distinct_category() {
        assert!(matches!(
            FilterKind::categorical((0..100).map(|n| n.to_string()), true),
            FilterKind::Enum(_)
        ));
        assert_eq!(
            FilterKind::categorical((0..101).map(|n| n.to_string()), true),
            FilterKind::Text
        );
        assert_eq!(
            FilterKind::categorical(["A".into()], false),
            FilterKind::Text
        );
        assert!(
            FilterDraft::Enum(["Old".into()].into())
                .compile(&FilterKind::Enum(vec!["New".into()]))
                .is_err()
        );
    }
    #[test]
    fn none_never_compiles_an_effective_filter() {
        assert_eq!(
            FilterDraft::Text(String::new()).compile(&FilterKind::None),
            Ok(None)
        );
        assert!(
            FilterDraft::Text("ignored".into())
                .compile(&FilterKind::None)
                .is_err()
        );
        assert!(
            FilterDraft::Enum(["ignored".into()].into())
                .compile(&FilterKind::None)
                .is_err()
        );
    }
    #[test]
    fn dates_validate_leap_days_and_time() {
        let leap = DateTimeParts {
            year: 2024,
            month: 2,
            day: 29,
            ..Default::default()
        };
        assert!(leap.key().is_ok());
        assert!(DateTimeParts { year: 2025, ..leap }.key().is_err());
        assert!(DateTimeParts { hour: 24, ..leap }.key().is_err());
        assert!(
            FilterDraft::DateTime {
                start: Some(DateTimeParts { hour: 1, ..leap }),
                end: Some(leap)
            }
            .compile(&FilterKind::DateTime)
            .is_err()
        );
    }
}
