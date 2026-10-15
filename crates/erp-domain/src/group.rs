//! Group companies stay separate tenants. Ownership, plant, and settlement are not the same field.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GroupError {
    #[error("group payload conflicts")]
    PayloadConflict,
}

/// A group links company spaces. It does not merge their tenant rows.
pub fn admit_group_link(group_code: &str, company_code: &str, legal_code: &str) -> Result<(), GroupError> {
    if group_code.trim().is_empty() || company_code.trim().is_empty() || legal_code.trim().is_empty() {
        return Err(GroupError::PayloadConflict);
    }
    if company_code == legal_code {
        return Err(GroupError::PayloadConflict);
    }
    Ok(())
}

/// Ownership, the plant that holds the goods, and the settlement party are stored apart.
pub fn admit_stock_title(owner_code: &str, plant_code: &str, settlement_code: &str, qty: i64) -> Result<(), GroupError> {
    if owner_code.trim().is_empty() || plant_code.trim().is_empty() || settlement_code.trim().is_empty() || qty == 0 {
        return Err(GroupError::PayloadConflict);
    }
    if owner_code == plant_code || owner_code == settlement_code || plant_code == settlement_code {
        return Err(GroupError::PayloadConflict);
    }
    Ok(())
}

/// An internal transfer needs both sides. A missing pair stays pending and does not post stock.
pub fn intercompany_pair_decision(seller_present: bool, buyer_present: bool) -> Result<&'static str, GroupError> {
    if seller_present && buyer_present {
        Ok("PAIRED")
    } else if seller_present || buyer_present {
        Ok("PENDING")
    } else {
        Err(GroupError::PayloadConflict)
    }
}

/// Group elimination removes the internal pair from the combined total. It does not change either company ledger.
pub fn eliminate_internal(company_a: i64, company_b: i64, internal_amount: i64) -> Result<i64, GroupError> {
    if internal_amount < 0 || company_a < internal_amount || company_b < internal_amount {
        return Err(GroupError::PayloadConflict);
    }
    Ok(company_a + company_b - 2 * internal_amount)
}
