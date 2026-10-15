//! Admission records a measurement; the verdict controls a later release.
use super::ManufacturingError;
use crate::quantity::QuantityError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectionVerdict {
    Passed,
    Failed,
    Missing,
}

impl InspectionVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "PASSED",
            Self::Failed => "FAILED",
            Self::Missing => "MISSING",
        }
    }
}

pub struct InspectionOutcome {
    pub stored_micros: i64,
    pub present: bool,
    pub passed: bool,
    pub verdict: InspectionVerdict,
}

/// The frozen standard must be valid. An out-of-range value remains a failed fact.
pub fn record_measurement(
    item_code: &str,
    required: bool,
    minimum: i64,
    maximum: i64,
    reading: Option<i64>,
) -> Result<InspectionOutcome, ManufacturingError> {
    if item_code.trim().is_empty()
        || item_code.len() > 1024
        || minimum > maximum
        || (required && minimum == 0 && maximum == 0)
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let verdict = match reading {
        None => InspectionVerdict::Missing,
        Some(value) if (minimum..=maximum).contains(&value) => InspectionVerdict::Passed,
        Some(_) => InspectionVerdict::Failed,
    };
    Ok(InspectionOutcome {
        stored_micros: reading.unwrap_or(0),
        present: reading.is_some(),
        passed: match verdict {
            InspectionVerdict::Passed => true,
            InspectionVerdict::Failed => false,
            InspectionVerdict::Missing => !required,
        },
        verdict,
    })
}
