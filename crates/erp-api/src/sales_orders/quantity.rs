//! Text quantities use base units; old numeric revision fields retain micro units.
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum SalesQuantity {
    Text(String),
    Integer(i64),
}
impl SalesQuantity {
    pub(super) fn units(&self) -> String {
        match self {
            Self::Text(text) => text.trim().to_string(),
            Self::Integer(value) => value.to_string(),
        }
    }
    pub(super) fn revision_micros(&self) -> Result<i64, &'static str> {
        match self {
            Self::Integer(value) if *value > 0 => Ok(*value),
            Self::Integer(_) => Err("sales quantity must be positive"),
            Self::Text(_) => erp_domain::trade::document_qty_micros(&self.units())
                .map_err(|_| "invalid sales quantity"),
        }
    }
    pub(super) fn legacy_integer(&self) -> Result<i32, &'static str> {
        let micros = erp_domain::trade::document_qty_micros(&self.units())
            .map_err(|_| "invalid sales quantity")?;
        if micros % 1_000_000 != 0 {
            return Err("fractional quantity requires the precise trade contract");
        }
        i32::try_from(micros / 1_000_000).map_err(|_| "legacy sales quantity exceeds its range")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_units_and_legacy_revision_micro_units_remain_explicit()
    -> Result<(), Box<dyn std::error::Error>> {
        let text: SalesQuantity = serde_json::from_str("\"1.25\"")?;
        assert_eq!(text.revision_micros(), Ok(1_250_000));
        assert!(text.legacy_integer().is_err());
        let old: SalesQuantity = serde_json::from_str("1250000")?;
        assert_eq!(old.revision_micros(), Ok(1_250_000));
        let one: SalesQuantity = serde_json::from_str("1")?;
        assert_eq!(one.legacy_integer(), Ok(1));
        assert!(serde_json::from_str::<SalesQuantity>("1.25").is_err());
        Ok(())
    }
}
