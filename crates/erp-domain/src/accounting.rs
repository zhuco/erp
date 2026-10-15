//! General-ledger rules. Partner balances are not a completed ledger.

/// A voucher line names one source and balances in base currency.
#[derive(Debug, Clone, Copy)]
pub struct VoucherLineInput<'a> {
    pub source_id: &'a str,
    pub account_code: &'a str,
    pub debit_cent: i64,
    pub credit_cent: i64,
    pub currency: &'a str,
    pub fx_cutoff: &'a str,
    pub base_currency: &'a str,
    pub rounding_cent: i64,
}

pub fn admit_voucher_line(input: VoucherLineInput<'_>) -> Result<(), &'static str> {
    let VoucherLineInput {
        source_id,
        account_code,
        debit_cent,
        credit_cent,
        currency,
        fx_cutoff,
        base_currency,
        rounding_cent,
    } = input;
    if source_id.trim().is_empty()
        || account_code.trim().is_empty()
        || currency.trim().is_empty()
        || fx_cutoff.trim().is_empty()
        || base_currency.trim().is_empty()
    {
        return Err("voucher line is incomplete");
    }
    if debit_cent < 0
        || credit_cent < 0
        || (debit_cent == 0 && credit_cent == 0)
        || (debit_cent > 0 && credit_cent > 0)
    {
        return Err("a voucher line is either debit or credit");
    }
    if rounding_cent < 0 {
        return Err("rounding difference cannot be negative");
    }
    Ok(())
}

/// Debits and credits must match before a voucher can post.
pub fn voucher_is_balanced(debit_cent: i64, credit_cent: i64) -> Result<(), &'static str> {
    if debit_cent != credit_cent || debit_cent <= 0 {
        return Err("voucher debits and credits are not balanced");
    }
    Ok(())
}

/// A closed period refuses another original line. A reversal is a new controlled line.
pub fn period_write_decision(closed: bool, reversal: bool) -> Result<&'static str, &'static str> {
    if closed && !reversal {
        return Err("a closed period cannot take another original line");
    }
    if reversal {
        Ok("REVERSAL")
    } else {
        Ok("POSTED")
    }
}
