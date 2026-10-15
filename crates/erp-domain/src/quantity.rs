//! Fixed-point quantity shared by inventory, documents, BOM, and MRP.
//!
//! The ledger scale is one millionth of the base unit. Existing integer columns
//! store whole base units; `1` remains one unit and must not be read as `0.001`.
//! Floating point is not a business ledger.

use thiserror::Error;

pub const QTY_SCALE: i64 = 1_000_000;
pub const QTY_FRACTION_DIGITS: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum QuantityError {
    #[error("quantity has more than six fractional digits")]
    ScaleExceeded,
    #[error("quantity is outside the supported range")]
    Overflow,
    #[error("quantity text is empty or invalid")]
    InvalidText,
}

/// Signed base-unit quantity stored as `units * 1_000_000 + fraction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScaledQty(i64);

impl ScaledQty {
    pub const ZERO: Self = Self(0);

    pub fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    pub fn as_micros(self) -> i64 {
        self.0
    }

    /// Whole units already stored in legacy integer quantity columns.
    pub fn from_legacy_whole_units(units: i64) -> Result<Self, QuantityError> {
        units
            .checked_mul(QTY_SCALE)
            .map(Self)
            .ok_or(QuantityError::Overflow)
    }

    pub fn try_add(self, other: Self) -> Result<Self, QuantityError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(QuantityError::Overflow)
    }

    pub fn try_sub(self, other: Self) -> Result<Self, QuantityError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(QuantityError::Overflow)
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }
}

/// Parse a decimal quantity. `0.125` is 125_000 micros; `1` is 1_000_000 micros.
pub fn parse_qty(text: &str) -> Result<ScaledQty, QuantityError> {
    let text = text.trim();
    if text.is_empty() || text.len() > 40 {
        return Err(QuantityError::InvalidText);
    }
    let (negative, rest) = if let Some(stripped) = text.strip_prefix('-') {
        (true, stripped)
    } else if let Some(stripped) = text.strip_prefix('+') {
        (false, stripped)
    } else {
        (false, text)
    };
    if rest.is_empty() || rest.starts_with('.') || rest.ends_with('.') {
        return Err(QuantityError::InvalidText);
    }
    let (whole, frac) = match rest.split_once('.') {
        Some((whole, frac)) => (whole, frac),
        None => (rest, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !frac.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(QuantityError::InvalidText);
    }
    if frac.len() > QTY_FRACTION_DIGITS as usize {
        return Err(QuantityError::ScaleExceeded);
    }
    let units: i64 = whole.parse().map_err(|_| QuantityError::Overflow)?;
    let mut micros = units
        .checked_mul(QTY_SCALE)
        .ok_or(QuantityError::Overflow)?;
    if !frac.is_empty() {
        let padded = format!("{frac:0<width$}", width = QTY_FRACTION_DIGITS as usize);
        let fraction: i64 = padded.parse().map_err(|_| QuantityError::Overflow)?;
        micros = micros
            .checked_add(fraction)
            .ok_or(QuantityError::Overflow)?;
    }
    if negative {
        micros = micros.checked_neg().ok_or(QuantityError::Overflow)?;
    }
    Ok(ScaledQty(micros))
}

/// How an auxiliary quantity becomes base-unit micros.
/// The ratio is base units per one auxiliary unit, not a float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitRounding {
    HalfUp,
    Floor,
    Ceil,
}

/// One request uses one scale. Display text is not a second quantity.
pub fn admit_unit_conversion(
    base_unit: &str,
    aux_unit: &str,
    ratio_text: &str,
    rounding: UnitRounding,
    display_text: &str,
    stored_micros: i64,
) -> Result<(i64, UnitRounding), QuantityError> {
    if base_unit.trim().is_empty() || aux_unit.trim().is_empty() || base_unit == aux_unit {
        return Err(QuantityError::InvalidText);
    }
    let ratio = parse_qty(ratio_text)?;
    if ratio.as_micros() <= 0 {
        return Err(QuantityError::InvalidText);
    }
    if !display_text.trim().is_empty() {
        let displayed = parse_qty(display_text)?;
        if displayed.as_micros() != stored_micros {
            return Err(QuantityError::InvalidText);
        }
    }
    let _ = rounding;
    Ok((ratio.as_micros(), rounding))
}

fn apply_rounding(quotient: i128, remainder: i128, divisor: i128, rounding: UnitRounding) -> i128 {
    match rounding {
        UnitRounding::Floor => quotient,
        UnitRounding::Ceil if remainder == 0 => quotient,
        UnitRounding::Ceil => quotient + remainder.signum(),
        UnitRounding::HalfUp => {
            let doubled = remainder.saturating_mul(2);
            if doubled.abs() >= divisor.abs() {
                quotient + remainder.signum()
            } else {
                quotient
            }
        }
    }
}

/// Converts auxiliary quantity into base-unit micros with an explicit rounding mode.
/// `ratio_micros` is base-unit micros per one auxiliary unit. Overflow is refused.
pub fn convert_aux_to_base(
    aux_micros: i64,
    ratio_micros: i64,
    rounding: UnitRounding,
) -> Result<i64, QuantityError> {
    if ratio_micros <= 0 {
        return Err(QuantityError::InvalidText);
    }
    let product = i128::from(aux_micros) * i128::from(ratio_micros);
    let scale = i128::from(QTY_SCALE);
    let quotient = product / scale;
    let remainder = product % scale;
    let adjusted = apply_rounding(quotient, remainder, scale, rounding);
    i64::try_from(adjusted).map_err(|_| QuantityError::Overflow)
}

/// Converts base-unit micros back to the auxiliary scale. The same ratio and rounding apply.
pub fn convert_base_to_aux(
    base_micros: i64,
    ratio_micros: i64,
    rounding: UnitRounding,
) -> Result<i64, QuantityError> {
    if ratio_micros <= 0 {
        return Err(QuantityError::InvalidText);
    }
    let product = i128::from(base_micros) * i128::from(QTY_SCALE);
    let ratio = i128::from(ratio_micros);
    let quotient = product / ratio;
    let remainder = product % ratio;
    let adjusted = apply_rounding(quotient, remainder, ratio, rounding);
    i64::try_from(adjusted).map_err(|_| QuantityError::Overflow)
}

/// A stored quantity reads back on the shared six-digit scale.
/// Legacy 1 is `1.000000`, not `0.001`. `0.125` is not `0`.
/// This is a stored quantity read, not a page and not a timing run.
pub fn read_back_qty(micros: i64, legacy_units: Option<i64>) -> Result<String, QuantityError> {
    if let Some(units) = legacy_units {
        let expected = ScaledQty::from_legacy_whole_units(units)?;
        if expected.as_micros() != micros {
            return Err(QuantityError::InvalidText);
        }
    }
    let text = format_qty(ScaledQty::from_micros(micros));
    if micros != 0 && (text == "0" || text == "-0" || text == "0.000000") {
        return Err(QuantityError::InvalidText);
    }
    let padded = format_qty_padded(micros);
    if legacy_units == Some(1) && padded != "1.000000" {
        return Err(QuantityError::InvalidText);
    }
    Ok(padded)
}

fn format_qty_padded(micros: i64) -> String {
    let negative = micros < 0;
    let abs = micros.unsigned_abs();
    let units = abs / QTY_SCALE as u64;
    let fraction = abs % QTY_SCALE as u64;
    let mut body = format!("{units}.{fraction:06}");
    if negative {
        body.insert(0, '-');
    }
    body
}

pub fn format_qty(qty: ScaledQty) -> String {
    let negative = qty.0 < 0;
    let abs = qty.0.unsigned_abs();
    let units = abs / QTY_SCALE as u64;
    let fraction = abs % QTY_SCALE as u64;
    let mut body = if fraction == 0 {
        units.to_string()
    } else {
        let digits = format!("{fraction:06}");
        let trimmed = digits.trim_end_matches('0');
        format!("{units}.{trimmed}")
    };
    if negative {
        body.insert(0, '-');
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_kilograms_keep_six_digit_scale() {
        let qty = parse_qty("0.125").unwrap();
        assert_eq!(qty.as_micros(), 125_000);
        assert_eq!(format_qty(qty), "0.125");
    }

    #[test]
    fn meters_with_one_fractional_digit_round_trip() {
        let qty = parse_qty("4.5").unwrap();
        assert_eq!(qty.as_micros(), 4_500_000);
        assert_eq!(format_qty(qty), "4.5");
    }

    #[test]
    fn legacy_integer_one_stays_one_base_unit() {
        let qty = ScaledQty::from_legacy_whole_units(1).unwrap();
        assert_eq!(qty.as_micros(), 1_000_000);
        assert_eq!(format_qty(qty), "1");
        assert_ne!(qty.as_micros(), 1_000);
    }

    #[test]
    fn extra_fractional_digits_are_rejected() {
        assert_eq!(parse_qty("1.0000001"), Err(QuantityError::ScaleExceeded));
    }

    #[test]
    fn box_ratio_converts_without_float() {
        let (ratio, rounding) =
            admit_unit_conversion("PCS", "BOX", "12", UnitRounding::HalfUp, "", 0).unwrap();
        assert_eq!(ratio, 12_000_000);
        let base = convert_aux_to_base(125_000, ratio, rounding).unwrap();
        assert_eq!(base, 1_500_000);
        assert_eq!(convert_base_to_aux(base, ratio, rounding).unwrap(), 125_000);
    }

    #[test]
    fn display_text_cannot_replace_stored_micros() {
        let refused = admit_unit_conversion("KG", "G", "0.001", UnitRounding::HalfUp, "0.13", 125_000);
        assert_eq!(refused, Err(QuantityError::InvalidText));
    }

    #[test]
    fn conversion_overflow_is_refused() {
        let refused = convert_aux_to_base(1_000_000_000_000_000, 10_000_000_000, UnitRounding::HalfUp);
        assert_eq!(refused, Err(QuantityError::Overflow));
    }

    #[test]
    fn explicit_rounding_is_not_a_float() {
        let floor = convert_aux_to_base(1, 500_000, UnitRounding::Floor).unwrap();
        let ceil = convert_aux_to_base(1, 500_000, UnitRounding::Ceil).unwrap();
        assert_eq!(floor, 0);
        assert_eq!(ceil, 1);
    }
}
