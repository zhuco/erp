//! Billing events. A browser claim of success is not a payment.

/// A callback must name the order, event, currency, and a matching signed amount.
pub fn admit_payment_callback(
    order_no: &str,
    event_id: &str,
    currency: &str,
    signed_amount: i64,
    order_amount: i64,
    signature_matches: bool,
) -> Result<(), &'static str> {
    if order_no.trim().is_empty() || event_id.trim().is_empty() || currency.trim().is_empty() {
        return Err("payment callback identity is incomplete");
    }
    if !signature_matches || signed_amount != order_amount || signed_amount < 0 {
        return Err("payment callback signature or amount does not match");
    }
    Ok(())
}

/// Payment, invoice, and entitlement move on their own. A browser claim stays unpaid.
pub fn payment_status(channel_paid: bool, browser_claims_paid: bool) -> &'static str {
    if channel_paid { "PAID" } else if browser_claims_paid { "UNPAID" } else { "OPEN" }
}

/// One paid order can grant one entitlement. A refund does not delete the original grant.
pub fn entitlement_decision(already_granted: bool, refunded: bool) -> Result<&'static str, &'static str> {
    if already_granted {
        return Err("one order can grant only one entitlement");
    }
    if refunded { Ok("REFUNDED") } else { Ok("GRANTED") }
}
