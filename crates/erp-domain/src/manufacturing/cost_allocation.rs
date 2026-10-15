//! A versioned quantity allocation, with one conserved monetary pool.
use super::ManufacturingError;
use crate::quantity::{QTY_SCALE, QuantityError, ScaledQty};

pub const POLICY: &str = "QUANTITY_SHARE_V1";
#[derive(Clone, Copy, Debug)]
pub struct CostPool {
    pub material_cent: i64,
    pub labor_cent: i64,
    pub overhead_cent: Option<i64>,
    pub finished: ScaledQty,
    pub wip: ScaledQty,
    pub rejected: ScaledQty,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation {
    pub total_cent: i64,
    pub labor_cent: i64,
    pub finished_cent: i64,
    pub wip_cent: i64,
    pub rejected_cent: i64,
    pub unit_cent: Option<i64>,
    pub unit_cent_micros: Option<i128>,
    pub estimated: bool,
}
pub fn allocate(input: CostPool) -> Result<Allocation, ManufacturingError> {
    let invalid = || ManufacturingError::Quantity(QuantityError::InvalidText);
    let overflow = || ManufacturingError::Quantity(QuantityError::Overflow);
    let good = input.finished.as_micros();
    let wip = input.wip.as_micros();
    let bad = input.rejected.as_micros();
    if input.material_cent < 0 || input.labor_cent < 0 || good < 0 || wip < 0 || bad < 0 {
        return Err(invalid());
    }
    let qty = good
        .checked_add(wip)
        .and_then(|q| q.checked_add(bad))
        .filter(|q| *q > 0)
        .ok_or_else(invalid)?;
    let labor_cent = input.labor_cent;
    let overhead = input.overhead_cent.unwrap_or(0);
    if overhead < 0 {
        return Err(invalid());
    }
    let total_cent = input
        .material_cent
        .checked_add(labor_cent)
        .and_then(|v| v.checked_add(overhead))
        .ok_or_else(overflow)?;
    let pool = i128::from(total_cent);
    let weight = i128::from(qty);
    let finished_cent = i64::try_from(pool * i128::from(good) / weight).map_err(|_| overflow())?;
    let mature_and_wip =
        i64::try_from(pool * i128::from(good + wip) / weight).map_err(|_| overflow())?;
    let wip_cent = mature_and_wip - finished_cent;
    let rejected_cent = total_cent - mature_and_wip;
    let (unit_cent, unit_cent_micros) = if good > 0 {
        let numerator = i128::from(finished_cent) * i128::from(QTY_SCALE);
        (
            Some(i64::try_from(numerator / i128::from(good)).map_err(|_| overflow())?),
            Some(numerator * i128::from(QTY_SCALE) / i128::from(good)),
        )
    } else {
        (None, None)
    };
    Ok(Allocation {
        total_cent,
        labor_cent,
        finished_cent,
        wip_cent,
        rejected_cent,
        unit_cent,
        unit_cent_micros,
        estimated: input.overhead_cent.is_none(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pool(total: i64, good: i64, wip: i64, bad: i64) -> CostPool {
        CostPool {
            material_cent: total,
            labor_cent: 0,
            overhead_cent: Some(0),
            finished: ScaledQty::from_micros(good),
            wip: ScaledQty::from_micros(wip),
            rejected: ScaledQty::from_micros(bad),
        }
    }
    #[test]
    fn allocation_does_not_charge_the_full_pool_to_each_destination() {
        let result = allocate(pool(100, 1_000_000, 1_000_000, 0)).expect("valid allocation");
        assert_eq!(
            (
                result.finished_cent,
                result.wip_cent,
                result.rejected_cent,
                result.unit_cent
            ),
            (50, 50, 0, Some(50))
        );
    }
    #[test]
    fn allocation_preserves_cent_tails_and_fractional_units() {
        let result = allocate(pool(1, 250_000, 250_000, 500_000)).expect("valid fractions");
        assert_eq!(
            (result.finished_cent, result.wip_cent, result.rejected_cent),
            (0, 0, 1)
        );
        let half = allocate(pool(100, 500_000, 0, 0)).expect("half unit");
        assert_eq!(half.unit_cent, Some(200));
    }
    #[test]
    fn allocation_has_no_unit_price_without_finished_output() {
        let result = allocate(pool(100, 0, 1_000_000, 0)).expect("WIP only");
        assert_eq!(
            (result.wip_cent, result.unit_cent, result.unit_cent_micros),
            (100, None, None)
        );
    }
    #[test]
    fn allocation_conserves_every_small_quantity_and_cent_pool() {
        for total in 0..32 {
            for good in 0..7 {
                for wip in 0..7 {
                    for bad in 0..7 {
                        if good + wip + bad == 0 {
                            continue;
                        }
                        let result =
                            allocate(pool(total, good, wip, bad)).expect("valid small pool");
                        assert_eq!(
                            result.finished_cent + result.wip_cent + result.rejected_cent,
                            total
                        );
                    }
                }
            }
        }
    }
}
