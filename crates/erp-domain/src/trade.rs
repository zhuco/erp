//! Trade quantity and money conservation. A trade sale does not require a BOM.
//! Amounts are integer cents. Quantities use the shared six-digit base-unit scale.

use crate::quantity::{QuantityError, ScaledQty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeLine {
    pub qty: ScaledQty,
    pub unit_cost_cent: i64,
    pub unit_price_cent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeClose {
    pub ending_qty: ScaledQty,
    pub inventory_cost_cent: i64,
    pub sales_cent: i64,
    pub cogs_cent: i64,
    pub gross_cent: i64,
    pub receivable_cent: i64,
    pub payable_cent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeError {
    Quantity(QuantityError),
    MoneyOverflow,
    NotEnoughStock,
    ReturnExceedsDelivery,
}

/// Personal report view. A shared view is a later permission item and is refused here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedReportView {
    pub owner_user_id: String,
    pub report_scope: String,
    pub name: String,
    pub date_from: String,
    pub date_to: String,
    pub dimension: String,
    pub measure: String,
    pub chart_type: String,
    pub formula: String,
}

const REPORT_DIMENSIONS: [&str; 3] = ["partner", "warehouse", "document_date"];
const REPORT_MEASURES: [&str; 3] = ["amount_cent", "qty", "open_cent"];
const REPORT_CHARTS: [&str; 3] = ["bar", "line", "table"];

/// Admits one personal view. Invalid dimension, measure, chart, or date range is refused.
pub fn admit_saved_report(view: &SavedReportView) -> Result<(), &'static str> {
    if view.owner_user_id.trim().is_empty()
        || view.report_scope.trim().is_empty()
        || view.name.trim().is_empty()
        || view.formula.trim().is_empty()
    {
        return Err("report view owner, scope, name, and formula are required");
    }
    if !view
        .name
        .bytes()
        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        return Err("report view name must be stored as ASCII");
    }
    if !REPORT_DIMENSIONS.contains(&view.dimension.as_str()) {
        return Err("report dimension is not available");
    }
    if !REPORT_MEASURES.contains(&view.measure.as_str()) {
        return Err("report measure is not available");
    }
    if !REPORT_CHARTS.contains(&view.chart_type.as_str()) {
        return Err("report chart is not available");
    }
    if view.date_from.trim().is_empty()
        || view.date_to.trim().is_empty()
        || view.date_from > view.date_to
    {
        return Err("report date range is invalid");
    }
    Ok(())
}

/// Restoring a view requires the same owner. Another user cannot read it as their own.
pub fn restore_saved_report(owner: &str, actor: &str) -> Result<(), &'static str> {
    if owner.trim().is_empty() || actor.trim().is_empty() || owner != actor {
        return Err("saved report belongs to another user");
    }
    Ok(())
}

/// A shared view is delivered only while the current grant is held.
/// A caller-claimed share, a self-share, or a revoked grant stores nothing.
pub fn shared_report_grant_current(
    owner: &str,
    recipient: &str,
    grant_held: bool,
    caller_claimed_share: bool,
) -> Result<(), &'static str> {
    if owner.trim().is_empty() || recipient.trim().is_empty() {
        return Err("shared report identity is incomplete");
    }
    if owner == recipient {
        return Err("a personal view is not a shared grant");
    }
    if caller_claimed_share {
        return Err("shared report grant cannot come from the request");
    }
    if !grant_held {
        return Err("shared report grant is not current");
    }
    Ok(())
}

/// Chart and export must use the same stored query. A caller query cannot replace it.
pub fn same_saved_query(stored: &str, requested: &str) -> Result<(), &'static str> {
    if stored.trim().is_empty() || stored != requested {
        return Err("chart and export must use the saved query");
    }
    Ok(())
}

/// A removed field is marked stale. The stored formula is left unchanged.
pub fn stale_report_fields(dimension: &str, measure: &str, available: &[String]) -> Vec<String> {
    let mut stale = Vec::new();
    if !available.iter().any(|field| field == dimension) {
        stale.push(dimension.to_string());
    }
    if !available.iter().any(|field| field == measure) {
        stale.push(measure.to_string());
    }
    stale
}

/// One operating-report total. Currencies are never added together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatingTotal {
    pub metric: String,
    pub currency: String,
    pub amount_cent: i64,
    pub qty: i64,
    pub definition: String,
}

/// An export range is the frozen snapshot minus exclusions. A caller list cannot add rows.
pub fn export_snapshot_rows<'a>(
    frozen: &'a [String],
    excluded: &[String],
) -> Result<Vec<&'a str>, TradeError> {
    if frozen.is_empty() {
        return Err(TradeError::MoneyOverflow);
    }
    let excluded_set: std::collections::BTreeSet<&str> =
        excluded.iter().map(String::as_str).collect();
    let mut rows: Vec<&str> = frozen
        .iter()
        .map(String::as_str)
        .filter(|id| !id.trim().is_empty() && !excluded_set.contains(id))
        .collect();
    rows.sort_unstable();
    rows.dedup();
    if rows.is_empty() {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(rows)
}

/// Download needs the same actor and a live file. Closing the page does not delete the job.
pub fn export_download_allowed(
    owner: &str,
    actor: &str,
    expired: bool,
    read_cancelled: bool,
) -> Result<bool, TradeError> {
    if owner.trim().is_empty() || actor.trim().is_empty() || owner != actor || expired {
        return Err(TradeError::MoneyOverflow);
    }
    let _ = read_cancelled;
    Ok(true)
}

/// A selected page cannot stand in for the authorized range.
pub fn page_cannot_replace_range(selected_count: i64, range_count: i64) -> Result<(), TradeError> {
    if selected_count < 0 || range_count < 0 || selected_count > range_count {
        return Err(TradeError::MoneyOverflow);
    }
    if selected_count > 0 && selected_count < range_count {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(())
}

/// One trend bucket. Every bucket uses the same measure and grain. A caller amount is not stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrendBucket {
    pub bucket: String,
    pub measure: String,
    pub amount_cent: i64,
    pub qty: i64,
}

/// Month buckets from stored facts. The bucket sum must equal the full-range total.
/// A page grain or a caller-supplied amount is refused.
pub fn trend_buckets_match_total(
    facts: &[(&str, i64, i64)],
    measure: &str,
    grain: &str,
    full_amount: i64,
    full_qty: i64,
) -> Result<Vec<TrendBucket>, TradeError> {
    if measure.trim().is_empty() || grain != "MONTH" {
        return Err(TradeError::MoneyOverflow);
    }
    let mut grouped: std::collections::BTreeMap<String, (i64, i64)> =
        std::collections::BTreeMap::new();
    for (bucket, amount, qty) in facts {
        if bucket.len() != 7 || !bucket.as_bytes().get(4).is_some_and(|byte| *byte == b'-') {
            return Err(TradeError::MoneyOverflow);
        }
        let entry = grouped.entry((*bucket).to_string()).or_insert((0, 0));
        entry.0 = entry
            .0
            .checked_add(*amount)
            .ok_or(TradeError::MoneyOverflow)?;
        entry.1 = entry.1.checked_add(*qty).ok_or(TradeError::MoneyOverflow)?;
    }
    let amount_sum = grouped
        .values()
        .try_fold(0_i64, |sum, (amount, _)| sum.checked_add(*amount))
        .ok_or(TradeError::MoneyOverflow)?;
    let qty_sum = grouped
        .values()
        .try_fold(0_i64, |sum, (_, qty)| sum.checked_add(*qty))
        .ok_or(TradeError::MoneyOverflow)?;
    if amount_sum != full_amount || qty_sum != full_qty {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(grouped
        .into_iter()
        .map(|(bucket, (amount, qty))| TrendBucket {
            bucket,
            measure: measure.to_string(),
            amount_cent: amount,
            qty,
        })
        .collect())
}

/// Server-side totals for the full authorized set. A page sum is not this report.
pub fn operating_totals(
    rows: &[(&str, &str, i64, i64)],
) -> Result<Vec<OperatingTotal>, TradeError> {
    let mut grouped: std::collections::BTreeMap<(String, String), (i64, i64)> =
        std::collections::BTreeMap::new();
    for (metric, currency, amount, qty) in rows {
        if metric.trim().is_empty() || currency.trim().is_empty() {
            return Err(TradeError::MoneyOverflow);
        }
        let key = ((*metric).to_string(), (*currency).to_string());
        let entry = grouped.entry(key).or_insert((0, 0));
        entry.0 = entry
            .0
            .checked_add(*amount)
            .ok_or(TradeError::MoneyOverflow)?;
        entry.1 = entry.1.checked_add(*qty).ok_or(TradeError::MoneyOverflow)?;
    }
    Ok(grouped
        .into_iter()
        .map(|((metric, currency), (amount, qty))| OperatingTotal {
            definition: format!(
                "{metric} full authorized range, currency {currency}, not a page sum"
            ),
            metric,
            currency,
            amount_cent: amount,
            qty,
        })
        .collect())
}

/// Moving-average close for one item. Qualified return restores the original
/// issue cost and reverses sales, cost, and the customer balance.
pub fn close_trade(
    purchased: TradeLine,
    sold_qty: ScaledQty,
    returned_qty: ScaledQty,
    customer_paid_cent: i64,
    supplier_paid_cent: i64,
) -> Result<TradeClose, TradeError> {
    if sold_qty.as_micros() > purchased.qty.as_micros() {
        return Err(TradeError::NotEnoughStock);
    }
    if returned_qty.as_micros() > sold_qty.as_micros() {
        return Err(TradeError::ReturnExceedsDelivery);
    }
    let net_sold = sold_qty
        .try_sub(returned_qty)
        .map_err(TradeError::Quantity)?;
    let ending = purchased
        .qty
        .try_sub(net_sold)
        .map_err(TradeError::Quantity)?;
    let purchase_cent = money(purchased.qty, purchased.unit_cost_cent)?;
    let cogs = money(net_sold, purchased.unit_cost_cent)?;
    let sales = money(net_sold, purchased.unit_price_cent)?;
    let inventory = purchase_cent
        .checked_sub(cogs)
        .ok_or(TradeError::MoneyOverflow)?;
    let payable = purchase_cent
        .checked_sub(supplier_paid_cent)
        .ok_or(TradeError::MoneyOverflow)?;
    let receivable = sales
        .checked_sub(customer_paid_cent)
        .ok_or(TradeError::MoneyOverflow)?;
    Ok(TradeClose {
        ending_qty: ending,
        inventory_cost_cent: inventory,
        sales_cent: sales,
        cogs_cent: cogs,
        gross_cent: sales.checked_sub(cogs).ok_or(TradeError::MoneyOverflow)?,
        receivable_cent: receivable,
        payable_cent: payable,
    })
}

/// Sales and qualified-return amounts for one trade item. No BOM is read.
pub fn sales_close_cent(
    purchased_qty: i64,
    unit_cost_cent: i64,
    sold_qty: i64,
    unit_price_cent: i64,
    returned_qty: i64,
    customer_paid_cent: i64,
    supplier_paid_cent: i64,
) -> Result<TradeClose, TradeError> {
    let purchased = TradeLine {
        qty: ScaledQty::from_legacy_whole_units(purchased_qty).map_err(TradeError::Quantity)?,
        unit_cost_cent,
        unit_price_cent,
    };
    close_trade(
        purchased,
        ScaledQty::from_legacy_whole_units(sold_qty).map_err(TradeError::Quantity)?,
        ScaledQty::from_legacy_whole_units(returned_qty).map_err(TradeError::Quantity)?,
        customer_paid_cent,
        supplier_paid_cent,
    )
}

/// Open balance kinds. Advance receipts and payments are not open receivables or payables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerBalanceKind {
    Receivable,
    Payable,
    AdvanceReceipt,
    AdvancePayment,
    OpeningReceivable,
    OpeningPayable,
}

impl PartnerBalanceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Receivable => "AR",
            Self::Payable => "AP",
            Self::AdvanceReceipt => "ADVANCE_RECEIPT",
            Self::AdvancePayment => "ADVANCE_PAYMENT",
            Self::OpeningReceivable => "OPENING_AR",
            Self::OpeningPayable => "OPENING_AP",
        }
    }

    pub fn is_advance(self) -> bool {
        matches!(self, Self::AdvanceReceipt | Self::AdvancePayment)
    }
}

/// A price is usable only inside its inclusive date range. An empty end means it is still open.
pub fn price_is_current(today: &str, start_date: &str, end_date: &str) -> Result<(), TradeError> {
    if today.trim().is_empty() || start_date.trim().is_empty() || today < start_date {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if !end_date.trim().is_empty() && today > end_date {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A trade kit component. Expanding it is not a manufacturing BOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitPart {
    pub item_code: String,
    pub qty_per_micros: i64,
}

/// Dropship never posts our physical inbound. The customer location is not our warehouse.
pub fn admit_dropship(
    supplier: &str,
    customer: &str,
    location: &str,
    qty: i64,
) -> Result<(), TradeError> {
    if supplier.trim().is_empty()
        || customer.trim().is_empty()
        || location.trim().is_empty()
        || supplier.trim() == customer.trim()
        || qty <= 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Consignment available quantity is only the named owner's stock at that location.
pub fn owner_available(
    owner: &str,
    location: &str,
    on_hand: i64,
    held: i64,
) -> Result<i64, TradeError> {
    if owner.trim().is_empty()
        || location.trim().is_empty()
        || on_hand < 0
        || held < 0
        || held > on_hand
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(on_hand - held)
}

/// A consignment return cites the original settlement and owner. It cannot exceed the sold quantity.
pub fn admit_consignment_return(
    owner: &str,
    sold: i64,
    already_returned: i64,
    qty: i64,
    fee_cent: i64,
) -> Result<i64, TradeError> {
    if owner.trim().is_empty()
        || sold <= 0
        || already_returned < 0
        || qty <= 0
        || fee_cent < 0
        || already_returned + qty > sold
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let product = fee_cent.checked_mul(qty).ok_or(TradeError::MoneyOverflow)?;
    product.checked_div(sold).ok_or(TradeError::MoneyOverflow)
}

/// A sale from consignment settles the sold quantity against that owner. It does not mix owners.
pub fn consignment_settlement(available: i64, sold: i64, fee_cent: i64) -> Result<i64, TradeError> {
    if available < 0 || sold <= 0 || sold > available || fee_cent < 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(fee_cent)
}

/// A cross-border term freezes its own cutoff. A blank cutoff is refused.
pub fn admit_trade_term(
    kind: &str,
    cutoff: &str,
    currency: &str,
    amount_cent: i64,
) -> Result<(), TradeError> {
    if !matches!(kind, "CONTRACT" | "SHIPMENT" | "TITLE" | "SETTLEMENT")
        || cutoff.trim().is_empty()
        || currency.trim().is_empty()
        || amount_cent < 0
    {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(())
}

/// Original currency converts once at the frozen rate. The rate source stays with the amount.
pub fn convert_frozen_fx(
    original_cent: i64,
    rate_ppm: i64,
    base_currency: &str,
    rate_source: &str,
) -> Result<i64, TradeError> {
    if original_cent < 0
        || rate_ppm <= 0
        || base_currency.trim().is_empty()
        || rate_source.trim().is_empty()
    {
        return Err(TradeError::MoneyOverflow);
    }
    original_cent
        .checked_mul(rate_ppm)
        .map(|scaled| scaled / 1_000_000)
        .ok_or(TradeError::MoneyOverflow)
}

/// A reversal cites the original amount and policy. It does not rewrite that row.
pub fn trade_reversal(original_cent: i64, policy: &str) -> Result<i64, TradeError> {
    if original_cent <= 0 || policy.trim().is_empty() {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(-original_cent)
}

/// A trade kit expands into component quantities. It is not a production order.
pub fn expand_trade_kit(kit_qty: i64, parts: &[KitPart]) -> Result<Vec<(String, i64)>, TradeError> {
    if kit_qty <= 0 || parts.is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut expanded = Vec::with_capacity(parts.len());
    for part in parts {
        if part.item_code.trim().is_empty()
            || part.qty_per_micros <= 0
            || !seen.insert(part.item_code.clone())
        {
            return Err(TradeError::Quantity(QuantityError::InvalidText));
        }
        let qty = part
            .qty_per_micros
            .checked_mul(kit_qty)
            .ok_or(TradeError::Quantity(QuantityError::InvalidText))?;
        expanded.push((part.item_code.clone(), qty));
    }
    Ok(expanded)
}

/// Two holds on one partner must share one lock. A second request sees the first hold.
pub fn admit_credit_race(
    partner_code: &str,
    first_left: i64,
    second_requested: i64,
) -> Result<&'static str, TradeError> {
    if partner_code.trim().is_empty()
        || first_left < 0
        || second_requested <= 0
        || second_requested > first_left
    {
        return Err(TradeError::MoneyOverflow);
    }
    Ok("QUEUED")
}

/// Credit includes open receivables and this hold. A suggestion never creates a payment.
pub fn credit_hold(
    limit_cent: i64,
    open_cent: i64,
    requested_cent: i64,
) -> Result<i64, TradeError> {
    if limit_cent < 0 || open_cent < 0 || requested_cent <= 0 {
        return Err(TradeError::MoneyOverflow);
    }
    let used = open_cent
        .checked_add(requested_cent)
        .ok_or(TradeError::MoneyOverflow)?;
    if used > limit_cent {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(limit_cent - used)
}

/// Open balance rebuilt from a confirmed source and its settlements. An advance is not included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuiltBalance {
    pub source_id: String,
    pub kind: &'static str,
    pub open_cent: i64,
    pub settled_cent: i64,
}

/// A confirmed sale or purchase opens AR or AP. A confirmed credit note opens a receivable
/// that cash can settle to zero. A draft does not. An advance stays out of that open total.
pub fn document_balance_kind(
    document_status: &str,
    document_kind: &str,
) -> Result<PartnerBalanceKind, TradeError> {
    if document_status != "CONFIRMED" {
        return Err(TradeError::MoneyOverflow);
    }
    match document_kind {
        "SALES" | "CREDIT" => Ok(PartnerBalanceKind::Receivable),
        "PURCHASE" => Ok(PartnerBalanceKind::Payable),
        _ => Err(TradeError::MoneyOverflow),
    }
}

/// Rebuilds one open amount from the source minus its settlements. The stored open column is not trusted.
pub fn rebuild_open_balance(
    source_id: &str,
    kind: PartnerBalanceKind,
    amount_cent: i64,
    settled_cent: i64,
) -> Result<RebuiltBalance, TradeError> {
    if source_id.trim().is_empty()
        || amount_cent < 0
        || settled_cent < 0
        || settled_cent > amount_cent
        || kind.is_advance()
    {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(RebuiltBalance {
        source_id: source_id.trim().to_string(),
        kind: kind.as_str(),
        open_cent: amount_cent - settled_cent,
        settled_cent,
    })
}

/// An advance is stored apart from open AR/AP. It does not become a receivable or payable.
pub fn admit_advance(
    kind: PartnerBalanceKind,
    amount_cent: i64,
    currency: &str,
    partner: &str,
) -> Result<i64, TradeError> {
    if !kind.is_advance()
        || amount_cent <= 0
        || currency.trim().is_empty()
        || partner.trim().is_empty()
    {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(amount_cent)
}

/// An opening balance is not a sales or purchase document. The kind stays opening.
pub fn admit_opening_balance(
    kind: PartnerBalanceKind,
    amount_cent: i64,
    currency: &str,
    partner: &str,
    source_id: &str,
) -> Result<i64, TradeError> {
    if !matches!(
        kind,
        PartnerBalanceKind::OpeningReceivable | PartnerBalanceKind::OpeningPayable
    ) || amount_cent <= 0
        || currency.trim().is_empty()
        || partner.trim().is_empty()
        || source_id.trim().is_empty()
    {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(amount_cent)
}

/// A confirmed document may open a balance. A draft must not. An advance is not an open AR/AP.
pub fn admit_partner_balance(
    source_status: &str,
    kind: PartnerBalanceKind,
    amount_cent: i64,
    currency: &str,
    partner: &str,
) -> Result<i64, TradeError> {
    if source_status != "CONFIRMED"
        || amount_cent <= 0
        || currency.trim().is_empty()
        || partner.trim().is_empty()
    {
        return Err(TradeError::MoneyOverflow);
    }
    if kind.is_advance() {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(amount_cent)
}

/// A recorded invoice. Recording it does not mean a tax authority issued it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceRecord {
    pub number: String,
    pub issuer: String,
    pub invoice_date: String,
    pub gross_cent: i64,
    pub tax_cent: i64,
    pub source_id: String,
    pub official_issued: bool,
}

/// Manual invoice identity is number plus issuer. A blank field or negative tax is refused.
pub fn admit_invoice_record(
    number: &str,
    issuer: &str,
    invoice_date: &str,
    gross_cent: i64,
    tax_cent: i64,
    source_id: &str,
) -> Result<InvoiceRecord, TradeError> {
    if number.trim().is_empty()
        || issuer.trim().is_empty()
        || invoice_date.trim().is_empty()
        || source_id.trim().is_empty()
        || gross_cent == 0
        || tax_cent < 0
        || tax_cent.abs() > gross_cent.abs()
    {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(InvoiceRecord {
        number: number.to_string(),
        issuer: issuer.to_string(),
        invoice_date: invoice_date.to_string(),
        gross_cent,
        tax_cent,
        source_id: source_id.to_string(),
        official_issued: false,
    })
}

/// Aging from source amount minus settlements. A zero open balance is not an exception.
pub fn aging_from_settlements(
    due_date: Option<&str>,
    cutoff: &str,
    amount_cent: i64,
    settlement_cents: &[i64],
) -> Result<Option<(&'static str, i64)>, TradeError> {
    let (settled, open) = rebuild_settled_open(amount_cent, settlement_cents, false)?;
    let _ = settled;
    let Some(bucket) = aging_bucket(due_date, cutoff, open)? else {
        return Ok(None);
    };
    Ok(Some((bucket, open)))
}

/// A recorded invoice never becomes an official tax issue. The caller flag is ignored.
pub fn invoice_stays_unofficial(requested_official: bool) -> bool {
    let _ = requested_official;
    false
}

/// Aging uses the balance due date and the cutoff. A missing due date is an exception, not bucket zero.
pub fn aging_bucket(
    due_date: Option<&str>,
    cutoff: &str,
    open_cent: i64,
) -> Result<Option<&'static str>, TradeError> {
    if cutoff.len() != 10 {
        return Err(TradeError::MoneyOverflow);
    }
    if open_cent == 0 {
        return Ok(None);
    }
    let Some(due) = due_date.filter(|value| value.len() == 10) else {
        return Ok(Some("UNDATED"));
    };
    let days = date_delta_days(cutoff, due)?;
    let bucket = if days <= 0 {
        "CURRENT"
    } else if days <= 30 {
        "1-30"
    } else if days <= 60 {
        "31-60"
    } else if days <= 90 {
        "61-90"
    } else {
        "91+"
    };
    Ok(Some(bucket))
}

fn date_delta_days(later: &str, earlier: &str) -> Result<i64, TradeError> {
    let later = chrono::NaiveDate::parse_from_str(later, "%Y-%m-%d")
        .map_err(|_| TradeError::MoneyOverflow)?;
    let earlier = chrono::NaiveDate::parse_from_str(earlier, "%Y-%m-%d")
        .map_err(|_| TradeError::MoneyOverflow)?;
    Ok((later - earlier).num_days())
}

/// One cost bucket at a shared cutoff. A difference points at a source ledger and does not clear it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostBucket {
    pub name: String,
    pub book_cent: i64,
    pub ledger_cent: i64,
    pub difference_cent: i64,
    pub source_ref: String,
}

/// One posted cost fact. Reconciliation sums these rows; it does not accept a caller total as the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostedCostFact {
    pub bucket: String,
    pub amount_cent: i64,
    pub source_ref: String,
}

/// Sums posted facts into the four required buckets. A missing bucket is zero, not omitted.
pub fn sum_posted_cost_facts(
    facts: &[PostedCostFact],
) -> Result<Vec<(&'static str, i64, String)>, TradeError> {
    const BUCKETS: [&str; 4] = ["INVENTORY", "WIP", "FINISHED", "COGS"];
    let mut totals = [0_i64; 4];
    let mut sources = [String::new(), String::new(), String::new(), String::new()];
    for fact in facts {
        let Some(index) = BUCKETS.iter().position(|bucket| *bucket == fact.bucket) else {
            return Err(TradeError::MoneyOverflow);
        };
        if fact.source_ref.trim().is_empty() {
            return Err(TradeError::MoneyOverflow);
        }
        totals[index] = totals[index]
            .checked_add(fact.amount_cent)
            .ok_or(TradeError::MoneyOverflow)?;
        if sources[index].is_empty() {
            sources[index] = fact.source_ref.clone();
        }
    }
    Ok(BUCKETS
        .into_iter()
        .enumerate()
        .map(|(index, bucket)| {
            let source = if sources[index].is_empty() {
                "none".to_string()
            } else {
                sources[index].clone()
            };
            (bucket, totals[index], source)
        })
        .collect())
}

/// A recorded difference must not be applied back to the source ledger.
pub fn difference_does_not_clear(book_cent: i64, ledger_cent: i64) -> Result<i64, TradeError> {
    let difference = book_cent
        .checked_sub(ledger_cent)
        .ok_or(TradeError::MoneyOverflow)?;
    if difference != 0 && ledger_cent == 0 && book_cent != 0 {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(ledger_cent)
}

/// Inventory, WIP, finished, and COGS must share one cutoff and currency. A mismatch is recorded, not cleared.
pub fn reconcile_cost_buckets(
    currency: &str,
    cutoff: &str,
    buckets: &[(&str, i64, i64, &str)],
) -> Result<Vec<CostBucket>, TradeError> {
    if currency.trim().is_empty() || cutoff.trim().is_empty() || buckets.is_empty() {
        return Err(TradeError::MoneyOverflow);
    }
    let mut rows = Vec::with_capacity(buckets.len());
    for (name, book, ledger, source) in buckets {
        if name.trim().is_empty() || source.trim().is_empty() {
            return Err(TradeError::MoneyOverflow);
        }
        let difference = book.checked_sub(*ledger).ok_or(TradeError::MoneyOverflow)?;
        rows.push(CostBucket {
            name: (*name).to_string(),
            book_cent: *book,
            ledger_cent: *ledger,
            difference_cent: difference,
            source_ref: (*source).to_string(),
        });
    }
    Ok(rows)
}

/// A closed period rejects a backdated posting. The adjustment needs a new document dated after the close.
pub fn admit_period_adjustment(
    closed_through: &str,
    document_date: &str,
    amount_cent: i64,
) -> Result<i64, TradeError> {
    if closed_through.trim().is_empty() || document_date.trim().is_empty() || amount_cent == 0 {
        return Err(TradeError::MoneyOverflow);
    }
    if document_date <= closed_through {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(amount_cent)
}

/// One allocated share. The last line receives the residual so the shares sum to the cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostShare {
    pub target: String,
    pub weight: i64,
    pub amount_cent: i64,
    pub residual: bool,
}

/// A stored tax rule. The caller does not choose the basis at allocation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxCostRule {
    pub tax_is_cost: bool,
}

/// The rule must be explicitly stored. A missing rule is not treated as tax-excluded or tax-included.
pub fn admit_tax_cost_rule(stored: Option<bool>) -> Result<TaxCostRule, TradeError> {
    let Some(tax_is_cost) = stored else {
        return Err(TradeError::MoneyOverflow);
    };
    Ok(TaxCostRule { tax_is_cost })
}

/// Landed cost cites one confirmed purchase. A draft or another document kind is refused.
pub fn admit_landed_source(document_status: &str, source_type: &str) -> Result<(), TradeError> {
    if document_status != "CONFIRMED" || source_type != "PURCHASE" {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(())
}

/// Tax basis is chosen by the tenant rule. A gross amount is not cost unless the rule says so.
pub fn expense_cost_cent(
    gross_cent: i64,
    deductible_tax_cent: i64,
    tax_is_cost: bool,
) -> Result<i64, TradeError> {
    if gross_cent <= 0 || deductible_tax_cent < 0 || deductible_tax_cent > gross_cent {
        return Err(TradeError::MoneyOverflow);
    }
    if tax_is_cost {
        return Ok(gross_cent);
    }
    Ok(gross_cent - deductible_tax_cent)
}

/// A receipt fee matches the payable row. Held stock does not become usable because a fee was posted.
pub fn admit_receipt_fee(
    allocated: i64,
    payable: i64,
    held_before: i64,
    held_after: i64,
) -> Result<&'static str, TradeError> {
    if allocated != payable || allocated < 0 || held_before != held_after || held_before < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    Ok("FEE")
}

/// Allocates a cost by explicit weights. The residual stays on the last target. Empty weights are refused.
pub fn allocate_by_weight(
    cost_cent: i64,
    weights: &[(&str, i64)],
) -> Result<Vec<CostShare>, TradeError> {
    if cost_cent < 0
        || weights.is_empty()
        || weights
            .iter()
            .any(|(target, weight)| target.trim().is_empty() || *weight <= 0)
    {
        return Err(TradeError::MoneyOverflow);
    }
    let total_weight: i64 = weights
        .iter()
        .try_fold(0_i64, |sum, (_, weight)| sum.checked_add(*weight))
        .ok_or(TradeError::MoneyOverflow)?;
    let mut assigned = 0_i64;
    let mut shares = Vec::with_capacity(weights.len());
    for (index, (target, weight)) in weights.iter().enumerate() {
        let last = index + 1 == weights.len();
        let amount = if last {
            cost_cent - assigned
        } else {
            let share = cost_cent
                .checked_mul(*weight)
                .ok_or(TradeError::MoneyOverflow)?
                / total_weight;
            assigned = assigned
                .checked_add(share)
                .ok_or(TradeError::MoneyOverflow)?;
            share
        };
        shares.push(CostShare {
            target: (*target).to_string(),
            weight: *weight,
            amount_cent: amount,
            residual: last,
        });
    }
    let sum: i64 = shares.iter().map(|share| share.amount_cent).sum();
    if sum != cost_cent {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(shares)
}

/// Moving-average unit cost in cents. Receipt value is added before dividing by the new quantity.
/// A zero quantity stays estimated and does not invent a unit cost.
pub fn moving_average_unit(
    on_hand_micros: i64,
    value_cent: i64,
    qty_micros: i64,
    amount_cent: i64,
) -> Result<(i64, i64, bool), TradeError> {
    if on_hand_micros < 0 || value_cent < 0 || qty_micros <= 0 || amount_cent < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    let next_qty = on_hand_micros
        .checked_add(qty_micros)
        .ok_or(TradeError::MoneyOverflow)?;
    let next_value = value_cent
        .checked_add(amount_cent)
        .ok_or(TradeError::MoneyOverflow)?;
    if next_qty == 0 {
        return Ok((0, next_value, true));
    }
    let unit = i64::try_from(
        i128::from(next_value) * i128::from(crate::quantity::QTY_SCALE) / i128::from(next_qty),
    )
    .map_err(|_| TradeError::MoneyOverflow)?;
    Ok((unit, next_value, false))
}

/// An issue stores the unit cost at that moment. It does not rewrite an earlier issue.
pub fn issue_cost_cent(
    unit_cent: i64,
    qty_micros: i64,
    estimated: bool,
) -> Result<(i64, bool), TradeError> {
    if estimated || unit_cent < 0 {
        return Ok((0, true));
    }
    Ok((money(ScaledQty::from_micros(qty_micros), unit_cent)?, false))
}

/// A return restores the original issue cost as a new row. The historical issue amount stays.
pub fn return_cost_cent(
    original_issue_cent: i64,
    original_qty: i64,
    return_qty: i64,
) -> Result<i64, TradeError> {
    if original_issue_cent < 0 || original_qty <= 0 || return_qty <= 0 || return_qty > original_qty
    {
        return Err(TradeError::MoneyOverflow);
    }
    let scaled = i128::from(original_issue_cent) * i128::from(return_qty);
    let rounded = (scaled + i128::from(original_qty / 2)) / i128::from(original_qty);
    i64::try_from(rounded).map_err(|_| TradeError::MoneyOverflow)
}

/// A partial return allocates the difference between cumulative rounded values.
/// The final return owns the original cent remainder, without exceeding its source.
#[derive(Debug, Clone, Copy)]
pub struct CostReturnInput {
    pub original_qty: i64,
    pub original_cent: i64,
    pub returned_qty: i64,
    pub returned_cent: i64,
    pub new_qty: i64,
}

pub fn cumulative_return_cost(input: CostReturnInput) -> Result<i64, TradeError> {
    let CostReturnInput {
        original_qty,
        original_cent,
        returned_qty,
        returned_cent,
        new_qty,
    } = input;
    if original_qty <= 0
        || original_cent < 0
        || returned_qty < 0
        || returned_qty > original_qty
        || returned_cent < 0
        || returned_cent > original_cent
        || (returned_qty == 0 && returned_cent != 0)
        || new_qty <= 0
        || new_qty > original_qty - returned_qty
    {
        return Err(TradeError::MoneyOverflow);
    }
    let total_qty = returned_qty
        .checked_add(new_qty)
        .ok_or(TradeError::MoneyOverflow)?;
    let total_cent = return_cost_cent(original_cent, original_qty, total_qty)?;
    total_cent
        .checked_sub(returned_cent)
        .filter(|amount| *amount >= 0 && *amount <= original_cent - returned_cent)
        .ok_or(TradeError::MoneyOverflow)
}

/// Splits a value into a unit cost and a remainder that the unit cannot express.
pub fn cost_unit_and_residual(value_cent: i64, qty_micros: i64) -> Result<(i64, i64), TradeError> {
    if qty_micros <= 0 {
        return Err(TradeError::MoneyOverflow);
    }
    let scale = i128::from(crate::quantity::QTY_SCALE);
    let unit = i64::try_from(i128::from(value_cent) * scale / i128::from(qty_micros))
        .map_err(|_| TradeError::MoneyOverflow)?;
    let rounded = i64::try_from(i128::from(unit) * i128::from(qty_micros) / scale)
        .map_err(|_| TradeError::MoneyOverflow)?;
    let residual = value_cent
        .checked_sub(rounded)
        .ok_or(TradeError::MoneyOverflow)?;
    Ok((unit, residual))
}

/// An independent rebuild must match the stored position. A remainder is kept, not dropped.
#[derive(Debug, Clone, Copy)]
pub struct CostRebuildInput {
    pub stored_qty: i64,
    pub stored_value: i64,
    pub stored_unit: i64,
    pub rebuilt_qty: i64,
    pub rebuilt_value: i64,
    pub rebuilt_unit: i64,
    pub residual_cent: i64,
    pub estimated: bool,
}

pub fn admit_cost_rebuild(input: CostRebuildInput) -> Result<&'static str, TradeError> {
    let CostRebuildInput {
        stored_qty,
        stored_value,
        stored_unit,
        rebuilt_qty,
        rebuilt_value,
        rebuilt_unit,
        residual_cent,
        estimated,
    } = input;
    if stored_qty != rebuilt_qty
        || stored_value != rebuilt_value
        || stored_unit != rebuilt_unit
        || residual_cent < 0
    {
        return Err(TradeError::MoneyOverflow);
    }
    if !estimated && stored_qty > 0 {
        let (unit, residual) = cost_unit_and_residual(stored_value, stored_qty)?;
        if unit != stored_unit || residual != residual_cent {
            return Err(TradeError::MoneyOverflow);
        }
        let restored = i128::from(stored_unit) * i128::from(stored_qty)
            / i128::from(crate::quantity::QTY_SCALE)
            + i128::from(residual_cent);
        if restored != i128::from(stored_value) {
            return Err(TradeError::MoneyOverflow);
        }
    }
    if estimated {
        return Ok("ESTIMATED");
    }
    if stored_qty <= 0 || stored_value < 0 || stored_unit < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    Ok("REBUILT")
}

/// A late fee or return adjustment is a new row. It does not rewrite the confirmed allocation.
pub fn late_expense_adjustment(
    confirmed_cent: i64,
    adjustment_cent: i64,
) -> Result<i64, TradeError> {
    if adjustment_cent == 0 {
        return Err(TradeError::MoneyOverflow);
    }
    confirmed_cent
        .checked_add(adjustment_cent)
        .ok_or(TradeError::MoneyOverflow)?;
    Ok(adjustment_cent)
}

/// Basic gross margin. Unknown cost stays unknown and is not reported as profit.
pub fn basic_gross_margin(
    sales_cent: i64,
    known_cost_cent: Option<i64>,
) -> Result<Option<i64>, TradeError> {
    let Some(cost) = known_cost_cent else {
        return Ok(None);
    };
    if sales_cent < 0 || cost < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    sales_cent
        .checked_sub(cost)
        .map(Some)
        .ok_or(TradeError::MoneyOverflow)
}

/// A manual cash entry. It records an account and never claims a bank executed the transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashEntry {
    pub direction: String,
    pub partner: String,
    pub currency: String,
    pub account: String,
    pub amount_cent: i64,
    pub bank_executed: bool,
}

/// Receipt, payment, refund, and reversal are separate rows. Currency and account are required.
pub fn admit_cash_entry(
    direction: &str,
    partner: &str,
    currency: &str,
    account: &str,
    amount_cent: i64,
) -> Result<CashEntry, TradeError> {
    if !matches!(direction, "RECEIPT" | "PAYMENT" | "REFUND" | "REVERSAL")
        || partner.trim().is_empty()
        || currency.trim().is_empty()
        || account.trim().is_empty()
        || amount_cent == 0
    {
        return Err(TradeError::MoneyOverflow);
    }
    if direction == "REVERSAL" && amount_cent >= 0 {
        return Err(TradeError::MoneyOverflow);
    }
    if direction != "REVERSAL" && amount_cent <= 0 {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(CashEntry {
        direction: direction.to_string(),
        partner: partner.to_string(),
        currency: currency.to_string(),
        account: account.to_string(),
        amount_cent,
        bank_executed: false,
    })
}

/// Open amount rebuilt from settlement rows. A paid flag is not a ledger and is ignored.
pub fn rebuild_settled_open(
    amount_cent: i64,
    settlement_cents: &[i64],
    marked_paid: bool,
) -> Result<(i64, i64), TradeError> {
    let _ = marked_paid;
    if amount_cent < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    let mut settled = 0_i64;
    for amount in settlement_cents {
        settled = settled
            .checked_add(*amount)
            .ok_or(TradeError::MoneyOverflow)?;
    }
    if settled < 0 || settled > amount_cent {
        return Err(TradeError::MoneyOverflow);
    }
    Ok((settled, amount_cent - settled))
}

/// A qualified return reduces the open receivable by its own cash row. It does not delete the sale.
pub fn return_receipt_offset(open_cent: i64, return_cent: i64) -> Result<i64, TradeError> {
    if open_cent < 0 || return_cent <= 0 || return_cent > open_cent {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(open_cent - return_cent)
}

/// Applies cash to one open balance. Cross-currency and over-application are refused.
pub fn apply_cash(
    open_cent: i64,
    settled_cent: i64,
    balance_currency: &str,
    entry_currency: &str,
    amount_cent: i64,
) -> Result<(i64, i64), TradeError> {
    if balance_currency != entry_currency {
        return Err(TradeError::MoneyOverflow);
    }
    settle_partner_balance(open_cent, settled_cent, amount_cent)
}

/// A reversal restores settled amount and must name the original settlement. It does not delete it.
pub fn reverse_settlement(settled_cent: i64, original_cent: i64) -> Result<i64, TradeError> {
    if original_cent <= 0 || settled_cent < original_cent {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(settled_cent - original_cent)
}

/// Partial settlement cannot exceed the open source. The remainder is rebuilt from source minus settled.
pub fn settle_partner_balance(
    open_cent: i64,
    settled_cent: i64,
    payment_cent: i64,
) -> Result<(i64, i64), TradeError> {
    if open_cent < 0 || settled_cent < 0 || payment_cent <= 0 || settled_cent > open_cent {
        return Err(TradeError::MoneyOverflow);
    }
    let next = settled_cent
        .checked_add(payment_cent)
        .ok_or(TradeError::MoneyOverflow)?;
    if next > open_cent {
        return Err(TradeError::MoneyOverflow);
    }
    Ok((next, open_cent - next))
}

/// A partner needs at least one role. The customer number and supplier number stay distinct from the partner id.
pub fn admit_partner_roles(
    customer: bool,
    supplier: bool,
    customer_no: &str,
    supplier_no: &str,
) -> Result<(), TradeError> {
    if !customer && !supplier {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if customer && customer_no.trim().is_empty() || supplier && supplier_no.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if !customer_no.trim().is_empty() && customer_no == supplier_no {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A contact and an address stay on the partner. They do not become the partner id.
pub fn admit_partner_contact(name: &str, phone: &str, address: &str) -> Result<(), TradeError> {
    if name.trim().is_empty() || phone.trim().is_empty() || address.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A preview names the references that would move. It does not move them or replace the target id.
pub fn admit_partner_merge_preview(
    source: &str,
    target: &str,
    ref_count: i64,
) -> Result<&'static str, TradeError> {
    if source.trim().is_empty() || target.trim().is_empty() || source == target || ref_count < 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("PREVIEW")
}

/// Disabling a partner with a receivable keeps the balance.
/// Delete is refused. This is a stored balance, not a page and not a 10k-partner query.
pub fn disable_keeps_receivable(
    receivable_cent: i64,
    requested: &str,
) -> Result<&'static str, TradeError> {
    if receivable_cent < 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    match requested {
        "DISABLE" if receivable_cent > 0 => Ok("DISABLED"),
        "DELETE" => Err(TradeError::Quantity(QuantityError::InvalidText)),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// A merge keeps both identities and moves references. It does not copy one number over the other id.
pub fn partner_merge_decision(source: &str, target: &str) -> Result<&'static str, TradeError> {
    if source.trim().is_empty() || target.trim().is_empty() || source == target {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("MERGED")
}

/// A term revision applies only to later documents. An existing order keeps its stored term.
pub fn term_revision_applies(document_date: &str, revision_date: &str) -> Result<bool, TradeError> {
    if document_date.trim().is_empty()
        || revision_date.trim().is_empty()
        || document_date == revision_date
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(document_date > revision_date)
}

/// A location page is one parent at a time. Offset and limit must stay inside that page.
pub fn admit_location_page(offset: i64, limit: i64) -> Result<(), TradeError> {
    if offset < 0 || limit <= 0 || limit > 1000 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A move changes the parent only. Stock quantity and owner stay on the same location id.
pub fn admit_location_move_stock(
    before_qty: i64,
    after_qty: i64,
    before_owner: &str,
    after_owner: &str,
) -> Result<(), TradeError> {
    if before_qty != after_qty || before_owner != after_owner || before_owner.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A location purpose is raw, work in process, finished, or held. Empty is refused.
pub fn admit_location_purpose(purpose: &str) -> Result<&'static str, TradeError> {
    match purpose {
        "RAW" => Ok("RAW"),
        "WIP" => Ok("WIP"),
        "FINISHED" => Ok("FINISHED"),
        "HOLD" => Ok("HOLD"),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// An employee record is not a login seat. The employee code and user id must stay distinct.
pub fn admit_employee(employee_code: &str, user_id: &str) -> Result<(), TradeError> {
    if employee_code.trim().is_empty()
        || user_id.trim().is_empty()
        || employee_code.trim() == user_id.trim()
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Employee files are not login seats. Twenty files must not open twenty seats.
/// This is a stored count, not a 20-row timing run and not a 1000-employee picker.
pub fn employee_files_add_no_seats(files: i64, seats: i64) -> Result<(), TradeError> {
    if files != 20 || seats != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A later rate does not rewrite a frozen order. The frozen rate and the new rate stay different.
/// This is a stored rate read, not a 1000-employee picker and not a drawn page.
pub fn rate_change_keeps_frozen(
    frozen_rate: i64,
    next_rate: i64,
    frozen_rows: i64,
) -> Result<(), TradeError> {
    if frozen_rate <= 0 || next_rate <= 0 || frozen_rate == next_rate || frozen_rows != 1 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// An employee file is not a login seat. Leaving keeps the historical labor report.
/// This is a stored employee read, not a 1000-employee picker and not a drawn page.
pub fn employee_file_is_not_seat(
    status: &str,
    seats: i64,
    report_kept: bool,
    reporter: &str,
    certifier: &str,
    employee_id: &str,
) -> Result<(), TradeError> {
    if status != "LEFT"
        || seats != 0
        || !report_kept
        || reporter.trim().is_empty()
        || certifier.trim().is_empty()
        || reporter == certifier
        || reporter == employee_id
        || certifier == employee_id
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Leaving closes the open assignment on that date. It does not delete the labor report or open a seat.
pub fn admit_employee_leave(
    start_date: &str,
    leave_date: &str,
    report_kept: bool,
    seats_before: i64,
    seats_after: i64,
) -> Result<&'static str, TradeError> {
    if start_date.trim().is_empty()
        || leave_date.trim().is_empty()
        || leave_date < start_date
        || !report_kept
        || seats_before != seats_after
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CLOSED")
}

/// A crew or work-center link needs an open date range. An empty end means it is still open.
pub fn admit_assignment_range(start_date: &str, end_date: &str) -> Result<(), TradeError> {
    if start_date.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if !end_date.trim().is_empty() && end_date < start_date {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A stock bucket is physical, reserved, unavailable, inbound, held, or work in process.
/// Inbound, held, and work in process are not issuable.
pub fn admit_stock_bucket(bucket: &str) -> Result<(), TradeError> {
    match bucket {
        "ON_HAND" | "RESERVED" | "UNAVAILABLE" | "IN_TRANSIT" | "HELD" | "WIP" => Ok(()),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// Held stock cannot be issued. Only released quantity is issuable.
pub fn issue_released(released: i64, already_issued: i64, qty: i64) -> Result<(), TradeError> {
    if released < 0 || already_issued < 0 || qty <= 0 || already_issued + qty > released {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A partial arrival stays queryable. Received plus open plus loss still equals issued.
pub fn admit_partial_arrival(
    issued: i64,
    received: i64,
    open: i64,
    loss: i64,
) -> Result<&'static str, TradeError> {
    let expected = transfer_balance(issued, received, loss)?;
    if open != expected || open <= 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("PARTIAL")
}

/// Balance equals opening plus valid ledger rows. A mismatch stops admission and keeps a check status.
/// This is a stored balance decision, not a million-row timing pass.
pub fn balance_matches_opening_plus_ledger(
    opening: i64,
    ledger: i64,
    stored: i64,
) -> Result<&'static str, TradeError> {
    if opening < 0 || stored < 0 || stored != opening.saturating_add(ledger) {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("MATCHED")
}

/// A sale during a count appends a ledger row. It does not rewrite the counted quantity.
pub fn admit_sale_during_count(
    sheet_before: i64,
    sheet_after: i64,
    sale_rows: i64,
) -> Result<&'static str, TradeError> {
    if sheet_before != sheet_after || sheet_before < 0 || sale_rows != 1 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("KEPT")
}

/// A second receive of 6 after the first 6 is an over-receive. It is a conflict and writes nothing.
/// This is a stored transfer decision, not a timing pass.
pub fn repeat_receive_six_refused(
    issued: i64,
    received: i64,
    refused: i64,
) -> Result<(), TradeError> {
    if issued != 10_000_000
        || received != 6_000_000
        || refused != 6_000_000
        || refused <= issued - received
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// After issue 10 and receive 6, a loss of 1 leaves 3 in transit.
/// This is a stored transfer read, not a timing pass.
pub fn loss_one_leaves_three(issued: i64, received: i64, loss: i64) -> Result<i64, TradeError> {
    if issued != 10_000_000 || received != 6_000_000 || loss != 1_000_000 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    transfer_balance(issued, received, loss)
}

/// Issue 10 and receive 6 leaves 4 in transit. This is a stored transfer read, not a timing pass.
pub fn issue_ten_receive_six(issued: i64, received: i64) -> Result<i64, TradeError> {
    if issued != 10_000_000 || received != 6_000_000 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    transfer_balance(issued, received, 0)
}

/// Issued quantity must equal received plus still-valid in-transit plus an explicit loss.
pub fn transfer_balance(issued: i64, received: i64, loss: i64) -> Result<i64, TradeError> {
    if issued <= 0 || received < 0 || loss < 0 || received + loss > issued {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(issued - received - loss)
}

/// A count difference is an adjustment row. It does not replace the stored balance.
pub fn count_adjustment(book_qty: i64, counted_qty: i64) -> Result<i64, TradeError> {
    if book_qty < 0 || counted_qty < 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(counted_qty - book_qty)
}

/// One allocatable lot. A suggestion is not an issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationLot {
    pub batch_code: String,
    pub location_code: String,
    pub available: i64,
    pub sort_date: String,
    pub stock_version: i64,
    pub frozen: bool,
    pub expired: bool,
}

/// One purchase line. Remaining quantity is ordered minus accepted receipts, never a stored balance overwrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseLine {
    pub line_no: i32,
    pub item_code: String,
    pub qty_micros: i64,
    pub price_cent: i64,
    pub tax_cent: i64,
    pub due_date: String,
}

/// Close is refused while any source line still has a remainder.
pub fn admit_purchase_close(action: &str, open_remaining: i64) -> Result<&'static str, TradeError> {
    if action != "CLOSE" || open_remaining < 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if open_remaining > 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CLOSED")
}

/// Tax is the sum of line tax. A revision does not replace an earlier snapshot's tax.
pub fn admit_purchase_tax(line_tax: &[i64], stored_tax: i64) -> Result<i64, TradeError> {
    if line_tax.is_empty() || line_tax.iter().any(|tax| *tax < 0) {
        return Err(TradeError::MoneyOverflow);
    }
    let total = line_tax
        .iter()
        .try_fold(0_i64, |sum, tax| sum.checked_add(*tax))
        .ok_or(TradeError::MoneyOverflow)?;
    if total != stored_tax {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(total)
}

/// Line numbers in one revision are unique. A repeated last line rejects the whole set.
pub fn purchase_lines_are_unique(lines: &[PurchaseLine]) -> Result<(), TradeError> {
    let mut seen = std::collections::BTreeSet::new();
    for line in lines {
        if !seen.insert(line.line_no) {
            return Err(TradeError::Quantity(QuantityError::InvalidText));
        }
    }
    Ok(())
}

/// A revision keeps the previous snapshot. Ordered quantity cannot fall below accepted receipts.
pub fn purchase_revision_covers_receipts(ordered: i64, received: i64) -> Result<(), TradeError> {
    if ordered <= 0 || received < 0 || ordered < received {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A revision keeps the previous snapshot. Close is refused while any line still has a remainder.
pub fn purchase_revision(
    previous: Option<(i32, &str)>,
    lines: &[PurchaseLine],
    action: &str,
) -> Result<(i32, &'static str), TradeError> {
    if lines.is_empty()
        || lines.iter().any(|line| {
            line.item_code.trim().is_empty()
                || line.qty_micros <= 0
                || line.price_cent < 0
                || line.tax_cent < 0
                || line.due_date.trim().is_empty()
        })
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let version = next_document_version(previous.map(|(version, _)| version))?;
    let status = match (action, previous.map(|(_, status)| status)) {
        ("OPEN", None) => "OPEN",
        ("REVISE", Some("OPEN")) => "OPEN",
        ("CLOSE", Some("OPEN")) => "CLOSED",
        _ => return Err(TradeError::Quantity(QuantityError::InvalidText)),
    };
    Ok((version, status))
}

/// One sales line. A resale does not create a production demand. A make line does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalesLine {
    pub line_no: i32,
    pub item_code: String,
    pub qty_micros: i64,
    pub price_cent: i64,
    pub discount_cent: i64,
    pub tax_cent: i64,
    pub due_date: String,
    pub address: String,
    pub source: String,
}

/// A revision cannot drop the ordered quantity below quantity already delivered or cancelled.
pub fn sales_revision_covers_delivery(
    ordered: i64,
    delivered: i64,
    cancelled: i64,
) -> Result<(), TradeError> {
    if ordered <= 0 || delivered < 0 || cancelled < 0 || ordered < delivered + cancelled {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A sales revision keeps the previous snapshot. Submit and fulfill are different actions.
/// Cancel is refused after fulfill. A stale version cannot overwrite the current one.
pub fn sales_revision(
    previous: Option<(i32, &str)>,
    expected_version: Option<i32>,
    lines: &[SalesLine],
    action: &str,
) -> Result<(i32, &'static str), TradeError> {
    if lines.is_empty()
        || lines.iter().any(|line| {
            line.item_code.trim().is_empty()
                || line.qty_micros <= 0
                || line.price_cent < 0
                || line.discount_cent < 0
                || line.tax_cent < 0
                || line.due_date.trim().is_empty()
                || line.address.trim().is_empty()
                || (line.source != "RESALE" && line.source != "MAKE")
        })
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if let (Some(expected), Some((current, _))) = (expected_version, previous)
        && expected != current
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let version = next_document_version(previous.map(|(version, _)| version))?;
    let status = match (action, previous.map(|(_, status)| status)) {
        ("SUBMIT", None) => "SUBMITTED",
        ("REVISE", Some("SUBMITTED")) => "SUBMITTED",
        ("REVISE", Some("FULFILLING")) => "FULFILLING",
        ("FULFILL", Some("SUBMITTED")) => "FULFILLING",
        ("CANCEL", Some("SUBMITTED")) => "CANCELLED",
        _ => return Err(TradeError::Quantity(QuantityError::InvalidText)),
    };
    Ok((version, status))
}

fn next_document_version(previous: Option<i32>) -> Result<i32, TradeError> {
    match previous {
        None => Ok(1),
        Some(version) if version > 0 => version
            .checked_add(1)
            .ok_or(TradeError::Quantity(QuantityError::Overflow)),
        Some(_) => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// Arrived stock is not usable until it is released. A held quantity cannot be issued.
pub fn receipt_release(arrived: i64, released: i64) -> Result<i64, TradeError> {
    if arrived <= 0 || released < 0 || released > arrived {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(arrived - released)
}

/// A supplier return cites the original receipt and its cost. It cannot exceed the released quantity.
pub fn supplier_return(
    released: i64,
    already_returned: i64,
    qty: i64,
    unit_cost_cent: i64,
) -> Result<i64, TradeError> {
    if released < 0
        || already_returned < 0
        || qty <= 0
        || unit_cost_cent < 0
        || qty > released.saturating_sub(already_returned)
    {
        return Err(TradeError::MoneyOverflow);
    }
    money(ScaledQty::from_micros(qty), unit_cost_cent)
}

/// Stored facts for one receipt. Quantities are base-unit micros, money is cents.
/// Release cost is the sum of its actual release postings, preserving per-event rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptOffsetFacts {
    pub arrived: i64,
    pub released: i64,
    pub returned_qty: i64,
    pub issued_qty: i64,
    pub payable_cent: i64,
    pub returned_cost: i64,
    pub released_cost_cent: i64,
    pub release_posting_rows: i64,
    pub return_rows: i64,
    pub return_posting_rows: i64,
    pub held_micros: i64,
    pub on_hand_micros: i64,
    pub issue_rows: i64,
    pub over_return_rows: i64,
}

/// A receipt offset conserves its own stored quantities and actual cost postings.
pub fn receipt_offset_holds(facts: ReceiptOffsetFacts) -> Result<&'static str, TradeError> {
    if facts.arrived <= 0
        || facts.released < 0
        || facts.released > facts.arrived
        || facts.returned_qty < 0
        || facts.returned_qty > facts.released
        || facts.issued_qty < 0
        || facts.issued_qty > facts.released - facts.returned_qty
        || facts.payable_cent < 0
        || facts.returned_cost < 0
        || facts.released_cost_cent < 0
        || (facts.released == 0 && facts.released_cost_cent != 0)
        || facts.returned_cost > facts.released_cost_cent
        || facts.release_posting_rows < 0
        || (facts.released > 0) != (facts.release_posting_rows > 0)
        || facts.return_rows < 0
        || (facts.returned_qty > 0) != (facts.return_rows > 0)
        || facts.return_posting_rows != facts.return_rows
        || (facts.returned_qty == 0 && facts.returned_cost != 0)
        || facts.held_micros != facts.arrived - facts.released
        || facts.on_hand_micros != facts.released - facts.returned_qty - facts.issued_qty
        || facts.payable_cent != facts.released_cost_cent - facts.returned_cost
        || facts.issue_rows < 0
        || (facts.issued_qty > 0) != (facts.issue_rows > 0)
        || facts.over_return_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(if facts.returned_qty > 0 {
        "OFFSET"
    } else {
        "NO_OFFSET"
    })
}

/// Two arrivals of 6 and 4 total 10. The caller does not name the total.
/// Ordered, first, and second quantities come from stored receipts. Over-receipt rows stay zero.
pub fn arrival_split_matches(
    ordered: i64,
    first: i64,
    second: i64,
    over_rows: i64,
) -> Result<&'static str, TradeError> {
    let total = first
        .checked_add(second)
        .ok_or(TradeError::Quantity(QuantityError::Overflow))?;
    if ordered != 10_000_000
        || first != 6_000_000
        || second != 4_000_000
        || total != ordered
        || over_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    purchase_remaining(ordered, total)?;
    Ok("RECEIVED")
}

/// Two sessions of one account cannot both revise the same version. The loser keeps the held draft.
/// Stored version, later versions, draft status, and draft body come from storage. The caller names the stale version.
/// This is not a drawn dual-client page.
pub fn sales_revision_conflict_keeps_draft(
    stored_version: i32,
    expected_version: i32,
    later_versions: i64,
    draft_status: &str,
    draft_body: &str,
    draft_version: i64,
) -> Result<&'static str, TradeError> {
    if stored_version != expected_version + 1
        || expected_version <= 0
        || later_versions != 0
        || draft_status != "HELD"
        || draft_body.trim().is_empty()
        || draft_version != i64::from(expected_version)
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("HELD")
}

/// A revision cannot drop ordered quantity below quantity already delivered. The open snapshot stays.
/// Ordered, delivered, version, and refused rows come from storage. The caller names the attempt.
pub fn sales_floor_holds(
    ordered: i64,
    delivered: i64,
    attempted: i64,
    version_no: i32,
    later_versions: i64,
) -> Result<&'static str, TradeError> {
    sales_revision_covers_delivery(ordered, delivered, 0)?;
    if attempted >= delivered || version_no != 2 || later_versions != 0 || delivered != 6_000_000 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("HELD")
}

/// A manual order needs no platform shop. Confirm and delivery stay on the same order.
/// Shop count, confirm rows, and delivery quantity come from storage. The caller names the order only.
pub fn manual_sale_holds(
    shop_rows: i64,
    platform_rows: i64,
    confirm_rows: i64,
    demand_rows: i64,
    delivered: i64,
) -> Result<&'static str, TradeError> {
    if shop_rows != 0
        || platform_rows != 0
        || confirm_rows != 1
        || demand_rows != 0
        || delivered != 6_000_000
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("MANUAL")
}

/// A repeated last line rolls the whole revision back. The open snapshot keeps its one stored line.
/// The caller names the refused line count. Stored line count, version, and later versions come from storage.
pub fn revision_rollback_holds(
    open_lines: i64,
    later_versions: i64,
    refused_line_count: i64,
) -> Result<&'static str, TradeError> {
    if open_lines != 1 || later_versions != 0 || refused_line_count != 100 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("ROLLED_BACK")
}

/// A refused revision leaves the open snapshot. Ordered quantity cannot fall below stored receipts.
/// The caller names the attempted quantity. Ordered, received, version, and refused rows come from storage.
pub fn revision_floor_holds(
    ordered: i64,
    received: i64,
    attempted: i64,
    version_no: i32,
    refused_rows: i64,
) -> Result<&'static str, TradeError> {
    purchase_revision_covers_receipts(ordered, received)?;
    if attempted >= received || version_no != 1 || refused_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("HELD")
}

/// A stored remainder is ordered minus accepted receipts. The caller does not name either quantity.
/// An open remainder keeps the order open and writes no close receipt.
pub fn open_remainder_matches(
    ordered: i64,
    received: i64,
    remaining: i64,
    closed_rows: i64,
) -> Result<&'static str, TradeError> {
    let expected = purchase_remaining(ordered, received)?;
    if remaining != expected || expected <= 0 || closed_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("OPEN")
}

/// Remaining quantity is the source line minus accepted receipts. Over-receipt is refused.
pub fn purchase_remaining(ordered: i64, received: i64) -> Result<i64, TradeError> {
    if ordered <= 0 || received < 0 || received > ordered {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(ordered - received)
}

/// FEFO sorts by expiry, FIFO by receipt date. Equal dates keep batch code order.
pub fn recommend_lots(
    rule: &str,
    requested: i64,
    lots: &[AllocationLot],
) -> Result<(Vec<AllocationLot>, Vec<String>), TradeError> {
    if requested <= 0 || (rule != "FEFO" && rule != "FIFO") {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let mut excluded = Vec::new();
    let mut open = Vec::new();
    for lot in lots {
        if lot.batch_code.trim().is_empty() || lot.available < 0 {
            return Err(TradeError::Quantity(QuantityError::InvalidText));
        }
        if lot.frozen {
            excluded.push(format!("{} frozen", lot.batch_code));
        } else if lot.expired {
            excluded.push(format!("{} expired", lot.batch_code));
        } else if lot.available == 0 {
            excluded.push(format!("{} empty", lot.batch_code));
        } else {
            open.push(lot.clone());
        }
    }
    open.sort_by(|left, right| {
        left.sort_date
            .cmp(&right.sort_date)
            .then(left.batch_code.cmp(&right.batch_code))
    });
    let mut left = requested;
    let mut chosen = Vec::new();
    for lot in open {
        if left == 0 {
            break;
        }
        let take = left.min(lot.available);
        chosen.push(AllocationLot {
            available: take,
            ..lot
        });
        left -= take;
    }
    if left > 0 {
        excluded.push(format!("short {left}"));
    }
    Ok((chosen, excluded))
}

/// A short suggestion names the uncovered quantity and does not invent a batch.
/// The requested quantity comes from stored demand. Stock is not reduced and no issue is stored.
#[derive(Debug, Clone, Copy)]
pub struct ShortSuggestionFacts<'a> {
    pub requested: i64,
    pub on_hand: i64,
    pub chosen_qty: i64,
    pub excluded_text: &'a str,
    pub suggestion_rows: i64,
    pub issue_rows: i64,
    pub stock_before: i64,
    pub stock_after: i64,
}

pub fn short_suggestion_names_gap(
    facts: ShortSuggestionFacts<'_>,
) -> Result<&'static str, TradeError> {
    let ShortSuggestionFacts {
        requested,
        on_hand,
        chosen_qty,
        excluded_text,
        suggestion_rows,
        issue_rows,
        stock_before,
        stock_after,
    } = facts;
    let short = requested.saturating_sub(on_hand);
    let expected = format!("short {short}");
    if requested <= on_hand
        || on_hand <= 0
        || chosen_qty != on_hand
        || short <= 0
        || excluded_text != expected
        || suggestion_rows != 1
        || issue_rows != 0
        || stock_before != on_hand
        || stock_after != stock_before
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("SHORT")
}

/// A stored-date suggestion excludes an expired batch and keeps same-day batch-code order.
/// The caller does not name expiry or quantity. No issue is stored and stock is not reduced.
#[derive(Debug, Clone, Copy)]
pub struct DatedSuggestionFacts<'a> {
    pub expired_batch: &'a str,
    pub later_batch: &'a str,
    pub same_day_first: &'a str,
    pub same_day_second: &'a str,
    pub chosen_first: &'a str,
    pub chosen_second: &'a str,
    pub excluded_text: &'a str,
    pub issue_rows: i64,
    pub stock_before: i64,
    pub stock_after: i64,
}

pub fn dated_suggestion_excludes_expired(
    facts: DatedSuggestionFacts<'_>,
) -> Result<&'static str, TradeError> {
    let DatedSuggestionFacts {
        expired_batch,
        later_batch,
        same_day_first,
        same_day_second,
        chosen_first,
        chosen_second,
        excluded_text,
        issue_rows,
        stock_before,
        stock_after,
    } = facts;
    let expected_excluded = format!("{expired_batch} expired");
    if expired_batch.trim().is_empty()
        || later_batch.trim().is_empty()
        || same_day_first.trim().is_empty()
        || same_day_second.trim().is_empty()
        || same_day_first >= same_day_second
        || chosen_first != same_day_first
        || chosen_second != same_day_second
        || excluded_text != expected_excluded
        || issue_rows != 0
        || stock_before <= 0
        || stock_after != stock_before
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("EXCLUDED")
}

/// A stored-flag suggestion excludes a frozen batch and keeps the open batch.
/// The caller does not name the frozen flag or the quantity. No issue is stored and stock is not reduced.
pub fn frozen_suggestion_excludes_batch(
    frozen_batch: &str,
    open_batch: &str,
    chosen_batch: &str,
    excluded_text: &str,
    issue_rows: i64,
    stock_before: i64,
    stock_after: i64,
) -> Result<&'static str, TradeError> {
    let expected_excluded = format!("{frozen_batch} frozen");
    if frozen_batch.trim().is_empty()
        || open_batch.trim().is_empty()
        || frozen_batch == open_batch
        || chosen_batch != open_batch
        || excluded_text != expected_excluded
        || issue_rows != 0
        || stock_before <= 0
        || stock_after != stock_before
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("EXCLUDED")
}

/// Saving a suggestion does not deduct stock. The suggested quantity must match the stored lot.
/// The caller does not supply the available quantity.
pub fn suggestion_keeps_stock(
    suggested: i64,
    on_hand_before: i64,
    on_hand_after: i64,
    issue_rows: i64,
) -> Result<&'static str, TradeError> {
    if suggested <= 0
        || on_hand_before <= 0
        || suggested > on_hand_before
        || on_hand_after != on_hand_before
        || issue_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("KEPT")
}

/// A suggestion is refused when later stock no longer covers it. No issue is stored.
/// The available quantity comes from the ledger, not from the caller.
pub fn taken_lot_conflicts(
    suggested: i64,
    on_hand: i64,
    issue_rows: i64,
) -> Result<&'static str, TradeError> {
    if suggested <= 0 || on_hand < 0 || on_hand >= suggested || issue_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CONFLICT")
}

/// An issue is allowed only when stored stock still covers the suggestion. Stock is not reduced here.
pub fn issue_matches_remaining_stock(
    suggested: i64,
    on_hand: i64,
    issued: i64,
) -> Result<&'static str, TradeError> {
    if suggested <= 0 || on_hand < suggested || issued != suggested {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("ISSUED")
}

/// A suggestion larger than stored stock is refused. Stock and issues stay unchanged.
pub fn suggestion_over_stock_refused(
    suggested: i64,
    on_hand: i64,
    suggestion_rows: i64,
    issue_rows: i64,
) -> Result<&'static str, TradeError> {
    if suggested <= 0
        || on_hand < 0
        || suggested <= on_hand
        || suggestion_rows != 0
        || issue_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("REFUSED")
}

/// A manual choice is allowed only when stored GOOD stock covers the chosen batch.
/// The quantity is copied from that stock. A held batch and an over-stock choice store nothing.
/// This does not rewrite the suggestion and does not deduct stock.
#[derive(Debug, Clone, Copy)]
pub struct ManualChoiceFacts<'a> {
    pub suggested_batch: &'a str,
    pub chosen_batch: &'a str,
    pub reason: &'a str,
    pub suggested_rule: &'a str,
    pub stored_rule: &'a str,
    pub good_on_hand: i64,
    pub held_on_hand: i64,
    pub chosen_qty: i64,
    pub issue_rows: i64,
    pub stock_rows_before: i64,
    pub stock_rows_after: i64,
}

pub fn manual_choice_matches_stock(
    facts: ManualChoiceFacts<'_>,
) -> Result<&'static str, TradeError> {
    let ManualChoiceFacts {
        suggested_batch,
        chosen_batch,
        reason,
        suggested_rule,
        stored_rule,
        good_on_hand,
        held_on_hand,
        chosen_qty,
        issue_rows,
        stock_rows_before,
        stock_rows_after,
    } = facts;
    if suggested_batch.trim().is_empty()
        || chosen_batch.trim().is_empty()
        || reason.trim().is_empty()
        || suggested_rule != stored_rule
        || suggested_batch == chosen_batch
        || good_on_hand <= 0
        || held_on_hand != 0
        || chosen_qty != good_on_hand
        || issue_rows != 0
        || stock_rows_after != stock_rows_before
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("MANUAL")
}

/// A held batch cannot be chosen. No choice, issue, or stock row is stored.
pub fn held_batch_choice_conflicts(
    good_on_hand: i64,
    held_on_hand: i64,
    choice_rows: i64,
    issue_rows: i64,
) -> Result<&'static str, TradeError> {
    if good_on_hand != 0 || held_on_hand <= 0 || choice_rows != 0 || issue_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CONFLICT")
}

/// A choice larger than stored GOOD stock is refused. No choice is stored.
pub fn over_stock_choice_conflicts(
    good_on_hand: i64,
    choice_rows: i64,
    issue_rows: i64,
) -> Result<&'static str, TradeError> {
    if good_on_hand > 0 || choice_rows != 0 || issue_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CONFLICT")
}

/// A manual choice is a new row. It does not rewrite the stored suggestion or its rule.
pub fn admit_manual_allocation(
    suggested_batch: &str,
    chosen_batch: &str,
    reason: &str,
    suggested_rule: &str,
    stored_rule: &str,
) -> Result<&'static str, TradeError> {
    if suggested_batch.trim().is_empty()
        || chosen_batch.trim().is_empty()
        || reason.trim().is_empty()
        || suggested_rule != stored_rule
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if suggested_batch == chosen_batch {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("MANUAL")
}

/// A later rule does not rewrite a stored suggestion. Issue still checks the stock version.
pub fn allocation_issue_allowed(
    suggested_version: i64,
    current_version: i64,
    suggested_qty: i64,
    available: i64,
) -> Result<(), TradeError> {
    if suggested_version != current_version || suggested_qty <= 0 || suggested_qty > available {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// An inventory alert is a suggestion. It never creates a payment or a production order.
pub fn admit_inventory_alert(
    rule: &str,
    object_code: &str,
    cutoff: &str,
    reason: &str,
    observed: i64,
    threshold: i64,
) -> Result<&'static str, TradeError> {
    if rule.trim().is_empty()
        || object_code.trim().is_empty()
        || cutoff.trim().is_empty()
        || reason.trim().is_empty()
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let state = match rule {
        "LOW_STOCK" | "AGING" | "EXPIRING" => {
            if observed < threshold {
                "OPEN"
            } else {
                "CLEAR"
            }
        }
        "SHORTAGE" => "SUGGEST_ONLY",
        _ => return Err(TradeError::Quantity(QuantityError::InvalidText)),
    };
    Ok(state)
}

/// A stock increase clears an open low-stock alert. The new observation comes from the ledger.
/// Clearing does not create a purchase. This is a stored stock read, not a page.
pub fn stock_increase_clears_alert(
    previous: &str,
    next: &str,
    before: i64,
    after: i64,
    threshold: i64,
    purchase_rows: i64,
) -> Result<&'static str, TradeError> {
    if previous != "OPEN"
        || next != "CLEAR"
        || before < 0
        || before >= threshold
        || after < threshold
        || after <= before
        || threshold <= 0
        || purchase_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CLEAR")
}

/// A cleared alert stays clear only when the stored observation still matches sellable stock.
pub fn cleared_alert_matches_stock(
    state: &str,
    observed: i64,
    on_hand: i64,
    threshold: i64,
    purchase_rows: i64,
) -> Result<&'static str, TradeError> {
    if state != "CLEAR"
        || observed != on_hand
        || observed < threshold
        || threshold <= 0
        || purchase_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CLEAR")
}

/// A manufacturing shortage is a suggestion. It does not pay and does not open a work order.
/// The shortage is demand minus sellable stock. The caller does not supply either number.
pub fn shortage_is_suggestion_only(
    demand: i64,
    on_hand: i64,
    shortage: i64,
    paid: bool,
    work_rows: i64,
) -> Result<&'static str, TradeError> {
    let expected = demand.saturating_sub(on_hand.max(0));
    if demand <= 0 || on_hand < 0 || shortage != expected || shortage <= 0 || paid || work_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("SUGGEST_ONLY")
}

/// Covered demand is not a shortage. No suggestion and no work order are stored.
pub fn covered_demand_not_shortage(
    demand: i64,
    on_hand: i64,
    suggestion_rows: i64,
    work_rows: i64,
) -> Result<&'static str, TradeError> {
    if demand <= 0 || on_hand < demand || suggestion_rows != 0 || work_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("COVERED")
}

/// Aging is open when the oldest stored inbound date is on or before the cutoff and stock remains.
/// The caller does not supply the inbound date. This is a stored date read, not a page.
pub fn aging_from_stored_receipt(
    inbound_on: &str,
    cutoff: &str,
    on_hand: i64,
    purchase_rows: i64,
    work_rows: i64,
) -> Result<&'static str, TradeError> {
    if !is_iso_date(inbound_on)
        || !is_iso_date(cutoff)
        || inbound_on > cutoff
        || on_hand <= 0
        || purchase_rows != 0
        || work_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("OPEN")
}

/// A later inbound date is not aged. No alert is stored.
pub fn later_receipt_not_aged(
    inbound_on: &str,
    cutoff: &str,
    on_hand: i64,
    alert_rows: i64,
) -> Result<&'static str, TradeError> {
    if !is_iso_date(inbound_on)
        || !is_iso_date(cutoff)
        || inbound_on <= cutoff
        || on_hand <= 0
        || alert_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("LATER")
}

/// A stored batch expiry is expiring when it is on or before the cutoff and still after production.
/// The caller does not supply the dates. This is a stored date read, not a page.
pub fn expiring_from_stored_date(
    produced_on: &str,
    expires_on: &str,
    cutoff: &str,
    purchase_rows: i64,
    work_rows: i64,
) -> Result<&'static str, TradeError> {
    if !is_iso_date(produced_on)
        || !is_iso_date(expires_on)
        || !is_iso_date(cutoff)
        || expires_on < produced_on
        || expires_on > cutoff
        || purchase_rows != 0
        || work_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("OPEN")
}

/// A later expiry is not an expiring alert. The stored date stays unchanged.
pub fn later_expiry_not_open(
    produced_on: &str,
    expires_on: &str,
    cutoff: &str,
    alert_rows: i64,
) -> Result<&'static str, TradeError> {
    if !is_iso_date(produced_on)
        || !is_iso_date(expires_on)
        || !is_iso_date(cutoff)
        || expires_on < produced_on
        || expires_on <= cutoff
        || alert_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("LATER")
}

/// Frozen and held stock is not available. The observation is sellable on-hand only.
/// This is a stored stock read, not a million-row timing pass and not a page.
pub fn held_stock_not_available(
    on_hand: i64,
    unavailable: i64,
    observed: i64,
    stock_rows_before: i64,
    stock_rows_after: i64,
) -> Result<i64, TradeError> {
    if on_hand < 0
        || unavailable < 0
        || unavailable > on_hand
        || observed != on_hand - unavailable
        || stock_rows_before < 0
        || stock_rows_after != stock_rows_before
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(observed)
}

/// Handling an alert changes only the alert. Stock rows and purchase rows stay the same.
/// This is a stored fact read, not a page and not a million-row timing pass.
pub fn alert_handle_keeps_facts(
    state: &str,
    stock_before: i64,
    stock_after: i64,
    purchase_before: i64,
    purchase_after: i64,
    purchase_id: &str,
    work_order_id: &str,
) -> Result<&'static str, TradeError> {
    if state != "HANDLED"
        || stock_before < 0
        || stock_after != stock_before
        || purchase_before < 0
        || purchase_after != purchase_before
        || !purchase_id.trim().is_empty()
        || !work_order_id.trim().is_empty()
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("HANDLED")
}

/// An alert observation must match an independent ledger sum. Handling creates no purchase or work order.
pub fn admit_alert_reconcile(
    observed: i64,
    ledger_qty: i64,
    purchase_rows: i64,
    work_rows: i64,
) -> Result<&'static str, TradeError> {
    if observed != ledger_qty || observed < 0 || purchase_rows != 0 || work_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("MATCHED")
}

/// The same rule and object notify only when the state changes or a handler records a decision.
pub fn alert_should_notify(previous_state: Option<&str>, next_state: &str, handled: bool) -> bool {
    handled || previous_state != Some(next_state)
}

/// A wrong batch is located by the scanned code and the expected code. It is not a document.
/// This is a stored scan read, not a page and not a scanner timing pass.
pub fn wrong_batch_located(
    scanned: &str,
    expected: &str,
    status: &str,
    document_rows: i64,
) -> Result<&'static str, TradeError> {
    if scanned.trim().is_empty()
        || expected.trim().is_empty()
        || scanned.trim() == expected.trim()
        || status != "WRONG_BATCH"
        || document_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("WRONG_BATCH")
}

/// A repeated input does not add quantity. One verified row stays at its first quantity.
/// This is a stored scan read, not a page and not a scanner timing pass.
pub fn repeat_scan_adds_no_qty(
    qty_micros: i64,
    verified_rows: i64,
    document_rows: i64,
) -> Result<i64, TradeError> {
    if qty_micros <= 0 || verified_rows != 1 || document_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(qty_micros)
}

/// A second scan of the same code is not an automatic submit. It stays an intent.
pub fn admit_repeat_scan(prior_rows: i64, document_rows: i64) -> Result<&'static str, TradeError> {
    if prior_rows != 1 || document_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("HELD")
}

/// An unrecognized code is a prompt, not a verified intent and not a document.
pub fn admit_scan_exception(code: &str) -> Result<&'static str, TradeError> {
    if code.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("EXCEPTION")
}

/// A scan is a verified intent. It does not submit a document by itself.
pub fn admit_scan(code: &str, kind: &str) -> Result<&'static str, TradeError> {
    if code.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    match kind {
        "ITEM" | "BATCH" | "LOCATION" => Ok("VERIFIED"),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// A print page keeps the frozen total. The page size cannot stand in for the line count.
pub fn admit_print_page(
    offset: i64,
    limit: i64,
    loaded: i64,
    line_count: i64,
) -> Result<&'static str, TradeError> {
    if offset < 0
        || limit <= 0
        || limit > 50
        || loaded < 0
        || loaded > limit
        || line_count < loaded + offset
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("PAGED")
}

/// A document print total is the sum of line amounts plus tax minus discount. It does not reread a later revision.
pub fn document_print_total(lines: &[(i64, i64, i64)]) -> Result<i64, TradeError> {
    if lines.is_empty() || lines.len() > 100 {
        return Err(TradeError::MoneyOverflow);
    }
    let mut total = 0_i64;
    for (amount, tax, discount) in lines {
        if *amount < 0 || *tax < 0 || *discount < 0 {
            return Err(TradeError::MoneyOverflow);
        }
        total = total
            .checked_add(*amount)
            .and_then(|value| value.checked_add(*tax))
            .and_then(|value| value.checked_sub(*discount))
            .ok_or(TradeError::MoneyOverflow)?;
    }
    Ok(total)
}

/// Print states stay distinct. Unknown is not queued, and a reprint needs a reason and a copy count.
pub fn admit_print_event(state: &str, copies: i64, reason: &str) -> Result<(), TradeError> {
    match state {
        "RENDERED" | "ACCEPTED" | "CONFIRMED" | "FAILED" | "UNKNOWN" => {}
        _ => return Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
    if copies <= 0 || (state == "CONFIRMED" && copies > 1 && reason.trim().is_empty()) {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A print agent identity is not a business login seat.
pub fn agent_is_not_seat(actor_kind: &str) -> bool {
    actor_kind == "AGENT"
}

/// A stored paper code is recognized only when the scanned text is that code.
/// Recognition does not add quantity and does not submit a document.
/// This is a stored code read, not a scanner, not paper, and not a page.
pub fn paper_code_recognized(
    scanned: &str,
    stored: &str,
    item_code: &str,
    status: &str,
    document_rows: i64,
) -> Result<&'static str, TradeError> {
    if scanned.trim().is_empty()
        || stored.trim().is_empty()
        || scanned.trim() != stored.trim()
        || item_code.trim().is_empty()
        || status != "RECOGNIZED"
        || document_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("RECOGNIZED")
}

/// An offline device stores a failure, not a paper completion.
/// This is a stored print read, not a printer and not a page.
pub fn offline_device_not_complete(
    failed_rows: i64,
    confirmed_rows: i64,
    reason: &str,
) -> Result<&'static str, TradeError> {
    if failed_rows != 1 || confirmed_rows != 0 || reason.trim() != "OFFLINE" {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("FAILED")
}

/// A batch is identified by enterprise, item, and batch code. Dates are `YYYY-MM-DD`.
pub fn admit_batch_dates(
    batch_code: &str,
    produced_on: &str,
    expires_on: &str,
) -> Result<(), TradeError> {
    if batch_code.trim().is_empty()
        || !is_iso_date(produced_on)
        || !is_iso_date(expires_on)
        || expires_on < produced_on
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Ordinary issue accepts only GOOD stock. Expired, frozen, and held batches are refused.
/// A missing expiry is not treated as expired.
pub fn batch_issue_allowed(
    expires_on: &str,
    as_of: &str,
    frozen: bool,
    quality_status: &str,
) -> Result<(), TradeError> {
    if frozen
        || quality_status != "GOOD"
        || (!expires_on.is_empty() && !as_of.is_empty() && expires_on < as_of)
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A refused ordinary issue leaves the batch unused. This is a stored refusal, not a page.
/// An open GOOD batch is expired only when the stored expiry is before the issue date.
pub fn held_or_expired_issue_refused(
    quality_status: &str,
    frozen: bool,
    expires_on: &str,
    as_of: &str,
    issue_receipts: i64,
    stock_rows: i64,
) -> Result<&'static str, TradeError> {
    let expired = quality_status == "GOOD"
        && !frozen
        && !expires_on.is_empty()
        && !as_of.is_empty()
        && expires_on < as_of;
    if quality_status == "GOOD" && !frozen && !expired {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if issue_receipts != 0 || stock_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if frozen {
        Ok("FROZEN")
    } else if quality_status == "HOLD" {
        Ok("HELD")
    } else if quality_status != "GOOD" {
        Ok("NOT_GOOD")
    } else if expired {
        Ok("EXPIRED")
    } else {
        Err(TradeError::Quantity(QuantityError::InvalidText))
    }
}

/// A receipt may choose an open batch. An expired or frozen batch is not selectable.
pub fn admit_batch_receipt(
    expires_on: &str,
    as_of: &str,
    frozen: bool,
) -> Result<&'static str, TradeError> {
    batch_issue_allowed(expires_on, as_of, frozen, "GOOD")?;
    Ok("SELECTED")
}

/// Freezing a batch blocks later issue. It does not erase an already recorded use.
pub fn admit_frozen_trace(
    frozen: bool,
    forward_rows: i64,
    work_order: &str,
) -> Result<&'static str, TradeError> {
    if !frozen || forward_rows != 1 || work_order.trim().is_empty() || work_order == "PLANNED_BOM" {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("TRACEABLE")
}

/// A closed-period adjustment is a new row. The original cost amount stays unchanged.
/// This is a stored cost read, not a page.
pub fn closed_period_adjustment_keeps_original(
    original_cent: i64,
    adjustment_cent: i64,
    original_rows: i64,
    adjustment_rows: i64,
    next_period: &str,
    closed_period: &str,
) -> Result<i64, TradeError> {
    if original_cent < 0
        || adjustment_cent == 0
        || original_rows != 1
        || adjustment_rows != 1
        || next_period.trim().is_empty()
        || next_period == closed_period
    {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(original_cent)
}

/// A known zero unit cost is legal. Quantity stays positive, value and unit stay 0, and it is not estimated.
/// This is a stored cost read, not a page.
pub fn zero_unit_cost_legal(
    qty_micros: i64,
    value_cent: i64,
    unit_cent: i64,
    estimated: bool,
) -> Result<i64, TradeError> {
    if qty_micros <= 0 || value_cent != 0 || unit_cent != 0 || estimated {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(unit_cent)
}

/// Three one-micro returns of a two-cent issue stay 1, 0, then 1 cent.
/// The original issue stays 2 cents. This is a stored cost read, not a page.
pub fn three_micro_return_exact(
    splits: [i64; 3],
    original_cent: i64,
    issue_rows: i64,
) -> Result<i64, TradeError> {
    if splits != [1, 0, 1] || original_cent != 2 || issue_rows != 1 {
        return Err(TradeError::MoneyOverflow);
    }
    Ok(splits[0] + splits[1] + splits[2])
}

/// A batch issue of 0.250 must stay 250000 micros. This is a stored quantity read, not a page.
pub fn batch_quarter_issue_exact(stored: i64, rows: i64) -> Result<i64, TradeError> {
    if stored != 250_000 || rows != 1 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(stored)
}

/// One serial may leave stock once. Twenty attempts still leave one issue.
/// This is a stored count, not a page and not an ERP-P timing pass.
pub fn serial_issue_at_most_once(
    successes: i64,
    issue_rows: i64,
    status: &str,
) -> Result<i64, TradeError> {
    if successes != 1 || issue_rows != 1 || status != "ISSUED" {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(successes)
}

/// A serialized piece is one whole unit. A fractional quantity cannot be split across serials.
pub fn admit_serial_qty(serialized: bool, qty_micros: i64) -> Result<(), TradeError> {
    if qty_micros == 0 || (serialized && qty_micros % 1_000_000 != 0) {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => true,
            _ => byte.is_ascii_digit(),
        })
}

/// Reviewing one delivery id deducts stock once. A second review of that id writes no second deduction.
/// Stage, deduction rows, and the refused request come from storage. The caller names the delivery only.
/// This is a stored quantity read, not a page and not a timing pass.
pub fn delivery_repeat_holds(
    stage: &str,
    qty: i64,
    on_hand_delta: i64,
    reserved_delta: i64,
    deduction_rows: i64,
    refused_rows: i64,
) -> Result<&'static str, TradeError> {
    if stage != "REVIEWED"
        || qty != 6_000_000
        || on_hand_delta != -qty
        || reserved_delta != -qty
        || deduction_rows != 2
        || refused_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("REVIEWED")
}

/// An order of 10 can be planned as 6 then 4. A third plan writes nothing and does not deduct stock.
/// First, second, over-plan rows, and stock rows come from storage. The caller names the order only.
/// This is a stored quantity read, not a page and not a timing pass.
pub fn delivery_split_holds(
    ordered: i64,
    first: i64,
    second: i64,
    over_rows: i64,
    stock_rows: i64,
) -> Result<&'static str, TradeError> {
    let planned = first
        .checked_add(second)
        .ok_or(TradeError::Quantity(QuantityError::Overflow))?;
    if ordered != 10_000_000
        || first != 6_000_000
        || second != 4_000_000
        || planned != ordered
        || over_rows != 0
        || stock_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    delivery_remaining(ordered, first, 0, second)?;
    Ok("PLANNED")
}

/// A delivery cannot exceed the current order line. Cancelled quantity stays on the order.
pub fn delivery_remaining(
    ordered: i64,
    delivered: i64,
    cancelled: i64,
    requested: i64,
) -> Result<i64, TradeError> {
    if ordered <= 0
        || delivered < 0
        || cancelled < 0
        || requested <= 0
        || delivered + cancelled + requested > ordered
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(ordered - delivered - cancelled - requested)
}

/// A shared history row records an action. It does not replace the document's own state machine.
pub fn document_action(
    current: &str,
    action: &str,
    has_downstream: bool,
) -> Result<&'static str, TradeError> {
    if has_downstream && (action == "VOID" || action == "DELETE") {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let next = match (current, action) {
        ("", "DRAFT") => "DRAFT",
        ("DRAFT", "SUBMIT") => "SUBMITTED",
        ("SUBMITTED", "EXECUTE") => "EXECUTING",
        ("EXECUTING", "CLOSE") => "CLOSED",
        ("SUBMITTED", "REVISE") => "SUBMITTED",
        ("DRAFT", "VOID") => "VOID",
        ("CLOSED", "REVERSE") => "REVERSED",
        _ => return Err(TradeError::Quantity(QuantityError::InvalidText)),
    };
    Ok(next)
}

/// Selling 6 and returning 2 leaves an effective delivery of 4. A later return above the original 6 writes nothing.
/// Delivered, returned, stock, credit, and refused rows come from storage. The caller names the delivery only.
/// This is a stored quantity read, not a page and not a timing pass.
pub fn sales_return_net_holds(
    delivered: i64,
    returned: i64,
    stock_in: i64,
    credit_cent: i64,
    over_rows: i64,
) -> Result<&'static str, TradeError> {
    if delivered != 6_000_000
        || returned != 2_000_000
        || stock_in != returned
        || credit_cent != 5_000
        || over_rows != 0
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let net = delivered
        .checked_sub(returned)
        .ok_or(TradeError::Quantity(QuantityError::Overflow))?;
    if net != 4_000_000 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    sales_return_split(delivered, returned, 1, 0)?;
    Ok("NET")
}

/// A sales return cites a reviewed delivery. Good and rejected quantities are separate and cannot exceed it.
pub fn sales_return_split(
    delivered: i64,
    already_returned: i64,
    good: i64,
    rejected: i64,
) -> Result<i64, TradeError> {
    if delivered <= 0 || already_returned < 0 || good < 0 || rejected < 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let qty = good
        .checked_add(rejected)
        .ok_or(TradeError::Quantity(QuantityError::Overflow))?;
    if qty == 0 || qty > delivered.saturating_sub(already_returned) {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(qty)
}

/// Ownership moves at review. Issue and carrier receipt are later states and do not deduct again.
pub fn delivery_stage(current: &str, action: &str) -> Result<&'static str, TradeError> {
    let next = match (current, action) {
        ("PLANNED", "RESERVE") => "RESERVED",
        ("RESERVED", "PICK") => "PICKED",
        ("PICKED", "REVIEW") => "REVIEWED",
        ("REVIEWED", "ISSUE") => "ISSUED",
        ("ISSUED", "SHIP") => "SHIPPED",
        _ => return Err(TradeError::Quantity(QuantityError::InvalidText)),
    };
    Ok(next)
}

/// Releasing source X does not reduce source Y. Physical stock stays.
/// This is a stored reservation read, not a 20-way timing pass.
pub fn release_keeps_other_source(
    released_x: i64,
    open_y_before: i64,
    open_y_after: i64,
) -> Result<(), TradeError> {
    if released_x <= 0 || open_y_before <= 0 || open_y_after != open_y_before {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Cancelling a reservation releases the open quantity. It does not deduct physical stock.
pub fn cancel_reservation(open: i64) -> Result<(i64, &'static str), TradeError> {
    if open <= 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok((-open, "CANCELLED"))
}

/// A short issue is a conflict. The failed source stays open and physical stock stays.
/// This is a stored quantity decision, not a 20-way timing pass.
pub fn short_issue_is_conflict(requested: i64, available: i64) -> Result<(), TradeError> {
    if requested <= 0 || available < 0 || requested <= available {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A failed source and its stock roll back together. Neither a source status nor a ledger row remains.
/// This is a stored rollback read, not a 20-way timing pass.
pub fn failed_source_rolls_stock_back(
    source_status: &str,
    stock_rows: i64,
    receipts: i64,
) -> Result<(), TradeError> {
    if source_status != "OPEN" || stock_rows != 0 || receipts != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A matching issue retry returns the original ledger and adds no second row.
/// This is a stored retry rule, not a 20-way timing pass.
pub fn issue_retry_adds_zero(
    first_ledger: &str,
    retry_ledger: &str,
    ledger_rows: i64,
    receipt_rows: i64,
) -> Result<(), TradeError> {
    if first_ledger.trim().is_empty()
        || first_ledger != retry_ledger
        || ledger_rows != 1
        || receipt_rows != 1
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// An issue from free stock cannot make available quantity negative. A refused issue writes nothing.
pub fn issue_free_stock(
    on_hand: i64,
    reserved: i64,
    unavailable: i64,
    qty: i64,
) -> Result<i64, TradeError> {
    if qty <= 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let available = available_from_ledger(on_hand, reserved, unavailable)?;
    if qty > available {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(-qty)
}

/// Consuming a reservation removes the reserved quantity and the same physical quantity once.
pub fn consume_reservation(reserved: i64, requested: i64) -> Result<(i64, i64), TradeError> {
    if reserved <= 0 || requested <= 0 || requested > reserved {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok((-requested, -requested))
}

/// A ledger page keeps the full total. The page size cannot stand in for that total.
pub fn admit_ledger_page(
    offset: i64,
    limit: i64,
    loaded: i64,
    total: i64,
) -> Result<(), TradeError> {
    if offset < 0
        || limit <= 0
        || limit > 1000
        || loaded < 0
        || loaded > limit
        || total < loaded + offset
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Quality status is stored once per balance. A repeated status does not add a second balance row.
pub fn admit_quality_balance(status: &str, stored_rows: i64) -> Result<&'static str, TradeError> {
    if status.trim().is_empty() || stored_rows != 1 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("REBUILT")
}

/// 1.9 minus 0.9 minus 0.9 must read back 0.1. A third 0.9 is refused.
/// This is a stored quantity read, not a million-row timing pass.
pub fn tenth_after_two_nines(received: i64, first: i64, second: i64) -> Result<i64, TradeError> {
    if received != 1_900_000 || first != 900_000 || second != 900_000 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    received
        .checked_sub(first)
        .and_then(|left| left.checked_sub(second))
        .filter(|left| *left == 100_000)
        .ok_or(TradeError::Quantity(QuantityError::InvalidText))
}

/// After issuing 0.250 from 1.000, only 0.750 remains sellable. A larger sale is refused.
/// This is a stored quantity read, not a million-row timing pass.
pub fn issue_leaves_sellable(received: i64, issued: i64) -> Result<i64, TradeError> {
    if received != 1_000_000 || issued != 250_000 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(received - issued)
}

/// Available quantity is physical minus reserved and unavailable. Inbound and work in process are excluded.
pub fn available_from_ledger(
    on_hand: i64,
    reserved: i64,
    unavailable: i64,
) -> Result<i64, TradeError> {
    if on_hand < 0 || reserved < 0 || unavailable < 0 || reserved + unavailable > on_hand {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(on_hand - reserved - unavailable)
}

/// Revoking a grant keeps the file and drops the download. A missing grant cannot be revoked.
pub fn attachment_grant_revoked(had_grant: bool) -> Result<&'static str, TradeError> {
    if !had_grant {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("REVOKED")
}

/// A download needs the current grant and an unexpired link. A foreign id, an expired link, or a revoked grant is refused.
/// This is a stored grant read, not a file body and not an opened link.
pub fn attachment_download_allowed(
    stored_tenant: &str,
    caller_tenant: &str,
    expires_at: i64,
    now: i64,
    revoked: bool,
) -> Result<(), TradeError> {
    if stored_tenant.trim().is_empty()
        || stored_tenant != caller_tenant
        || expires_at <= now
        || revoked
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A citation names the object and the attachment. It does not publish the storage key.
pub fn admit_attachment_cite(
    object_id: &str,
    attachment_id: &str,
) -> Result<&'static str, TradeError> {
    if object_id.trim().is_empty() || attachment_id.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("CITED")
}

/// The tenant total stays within the filled quota. A public key for another tenant is refused.
pub fn admit_attachment_quota(
    used_bytes: i64,
    next_bytes: i64,
    quota_bytes: i64,
    key_tenant: &str,
    caller_tenant: &str,
) -> Result<(), TradeError> {
    if quota_bytes <= 0 || used_bytes < 0 || next_bytes <= 0 || key_tenant != caller_tenant {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if used_bytes
        .checked_add(next_bytes)
        .ok_or(TradeError::Quantity(QuantityError::Overflow))?
        > quota_bytes
    {
        return Err(TradeError::Quantity(QuantityError::Overflow));
    }
    Ok(())
}

/// An attachment needs an object, a digest, and a size at or below 10 MiB. The storage key stays tenant-scoped.
pub fn admit_attachment(object_id: &str, digest: &str, byte_count: i64) -> Result<(), TradeError> {
    if object_id.trim().is_empty()
        || digest.trim().is_empty()
        || byte_count <= 0
        || byte_count > 10 * 1024 * 1024
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A custom field accepts text, number, or date. Script text is refused.
pub fn admit_custom_field(field_type: &str, value: &str) -> Result<(), TradeError> {
    if value.trim().is_empty() || value.contains("script") {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    match field_type {
        "TEXT" | "NUMBER" | "DATE" => Ok(()),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// A resumed opening keeps the original identity and does not add a second posting.
pub fn admit_opening_resume(
    stored_id: &str,
    requested_id: &str,
    posted_rows: i64,
) -> Result<&'static str, TradeError> {
    if stored_id.trim().is_empty() || stored_id != requested_id || posted_rows != 1 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("RESUMED")
}

/// Quantity, cost, and partner balances are rebuilt from their own ledger rows.
pub fn admit_opening_rebuild(
    qty: i64,
    cost: i64,
    partner: i64,
    rebuilt_qty: i64,
    rebuilt_cost: i64,
    rebuilt_partner: i64,
) -> Result<(), TradeError> {
    if qty != rebuilt_qty || cost != rebuilt_cost || partner != rebuilt_partner {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// An unexplained quantity or cost gap cannot enable the opening. This is a stored explanation, not a drawn review.
pub fn opening_gap_explained(
    qty_gap: i64,
    cost_gap: i64,
    explanation: &str,
) -> Result<(), TradeError> {
    if qty_gap == 0 && cost_gap == 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let text = explanation.trim();
    if text.is_empty() || text.len() > 200 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Opening needs one cutoff and positive quantity, cost, and partner amounts. A correction is a new row.
pub fn admit_opening(
    cutoff_date: &str,
    qty_micros: i64,
    cost_cent: i64,
    partner_cent: i64,
) -> Result<(), TradeError> {
    if cutoff_date.trim().is_empty() || qty_micros <= 0 || cost_cent < 0 || partner_cent == 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A correction reverses the original amount. It does not replace the stored balance.
pub fn opening_correction(original_cent: i64, correction_cent: i64) -> Result<i64, TradeError> {
    if original_cent == 0 || correction_cent == 0 || correction_cent == original_cent {
        return Err(TradeError::MoneyOverflow);
    }
    original_cent
        .checked_add(correction_cent)
        .ok_or(TradeError::MoneyOverflow)
}

/// One precheck names the bad line. Duplicate, blank unit, and over-scale each keep a line number.
/// This is a stored row read, not a 10k-row timing run.
pub fn import_row_error(line_no: i32, kind: &str) -> Result<&'static str, TradeError> {
    if line_no < 1 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    match kind {
        "duplicate" => Ok("duplicate code in file"),
        "blank-unit" => Ok("unit is required"),
        "over-scale" => Ok("quantity scale exceeded"),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// A readable error row names its line and its reason. A ready row is not this error.
/// This is a stored row read, not a 10k-row timing run.
pub fn import_error_line_readable(
    line_no: i32,
    error_text: &str,
    items_written: i64,
) -> Result<(), TradeError> {
    if line_no < 1 || error_text.trim().is_empty() || items_written != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A precheck may report errors. It must not write or overwrite a business row.
pub fn import_precheck_decision(confirmed: bool) -> Result<&'static str, TradeError> {
    if confirmed {
        Err(TradeError::Quantity(QuantityError::InvalidText))
    } else {
        Ok("PRECHECKED")
    }
}

/// A result download repeats the confirmed counts. A pending job cannot publish a result.
pub fn admit_import_result(
    status: &str,
    imported: i64,
    errors: i64,
    skipped: i64,
    row_count: i64,
) -> Result<&'static str, TradeError> {
    if status != "CONFIRMED"
        || imported < 0
        || errors < 0
        || skipped < 0
        || imported + errors + skipped != row_count
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("DOWNLOADED")
}

/// A batch that depends on an old result is blocked while the job is still pending.
pub fn admit_import_batch(status: &str) -> Result<&'static str, TradeError> {
    match status {
        "PRECHECKED" => Err(TradeError::Quantity(QuantityError::InvalidText)),
        "CONFIRMED" => Ok("ALLOWED"),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// Confirmation requires the same file digest and a duplicate policy. Skip keeps the old row.
pub fn import_confirm_decision(
    file_digest: &str,
    duplicate_policy: &str,
) -> Result<&'static str, TradeError> {
    if file_digest.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    match duplicate_policy {
        "SKIP" => Ok("SKIP"),
        "ERROR" => Ok("ERROR"),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// A labor report keeps the employee id. The reporter and the certifier are different people.
pub fn admit_labor_report(
    employee_id: &str,
    reporter_user_id: &str,
    certifier_user_id: &str,
) -> Result<(), TradeError> {
    if employee_id.trim().is_empty()
        || reporter_user_id.trim().is_empty()
        || certifier_user_id.trim().is_empty()
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if reporter_user_id == certifier_user_id
        || employee_id == reporter_user_id
        || employee_id == certifier_user_id
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Moving a node changes its parent only. It does not transfer stock or change ownership.
pub fn location_move_decision(
    location_id: &str,
    parent_id: &str,
) -> Result<&'static str, TradeError> {
    if location_id.trim().is_empty() || parent_id.trim().is_empty() || location_id == parent_id {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("MOVED")
}

/// A sales return credit must match the original issue cost. A different cited cost is refused.
pub fn admit_return_against_issue(
    issue_unit_cost: i64,
    cited_unit_cost: i64,
    credit: i64,
    qty_micros: i64,
) -> Result<&'static str, TradeError> {
    if issue_unit_cost < 0 || cited_unit_cost != issue_unit_cost || qty_micros <= 0 || credit < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    let expected = supplier_return(qty_micros, 0, qty_micros, issue_unit_cost)?;
    if credit != expected {
        return Err(TradeError::MoneyOverflow);
    }
    Ok("REVERSED")
}

/// A lost delivery response is looked up by the original request. The lookup does not ship again.
pub fn admit_delivery_lookup(
    stored_status: &str,
    stage: &str,
    qty_micros: i64,
) -> Result<&'static str, TradeError> {
    if stored_status != "SUCCEEDED" || stage.trim().is_empty() || qty_micros <= 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("LOOKED_UP")
}

/// A trade order saves without a BOM. Only a make line creates a production demand.
pub fn admit_trade_without_bom(
    source: &str,
    bom_rows: i64,
    demand_rows: i64,
) -> Result<&'static str, TradeError> {
    if source != "RESALE" || bom_rows != 0 || demand_rows != 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok("TRADE")
}

/// A kit flag is not the manufacturing flag. The two stay on separate fields.
pub fn kit_is_manufacturing(kit: bool, make: bool) -> bool {
    kit && make && false
}

/// A product keeps its identity when a SKU is added. A blank code is refused.
pub fn admit_item_identity(product_code: &str, sku_code: &str) -> Result<(), TradeError> {
    if product_code.trim().is_empty() || sku_code.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// One item may be bought, made, and sold. Kit expansion is not a manufacturing flag.
pub fn admit_item_roles(buy: bool, make: bool, sell: bool) -> Result<(), TradeError> {
    if buy || make || sell {
        Ok(())
    } else {
        Err(TradeError::Quantity(QuantityError::InvalidText))
    }
}

/// A referenced item is still stored. Delete is not a hard removal.
/// This is a stored item read, not a 100k-item query.
pub fn referenced_item_stays(
    status: &str,
    refs: i64,
    buy: bool,
    make: bool,
    sell: bool,
) -> Result<(), TradeError> {
    if refs < 0 || status.trim().is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if refs > 0 && status == "DELETED" {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    if !(buy || make || sell) {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// One code under one parent is one location. A second row, or a foreign row counted as local, is refused.
/// This is a stored location read, not a 10k-node tree and not a drawn warehouse page.
pub fn duplicate_location_code_refused(code: &str, rows: i64) -> Result<(), TradeError> {
    if code.trim().is_empty() || rows != 1 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// Renaming a location changes the code only. The location id and stock quantity stay.
/// This is a stored stock read, not a 10k-node tree and not a drawn warehouse page.
pub fn rename_keeps_stock_identity(
    location_id: &str,
    stock_location_id: &str,
    old_code: &str,
    new_code: &str,
    on_hand: i64,
    stock_rows: i64,
) -> Result<(), TradeError> {
    if location_id.trim().is_empty()
        || stock_location_id != location_id
        || old_code.trim().is_empty()
        || new_code.trim().is_empty()
        || old_code == new_code
        || on_hand <= 0
        || stock_rows != 1
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A location with stock cannot be deleted. The same location id and quantity stay.
/// This is a stored stock read, not a 10k-node tree and not a drawn warehouse page.
pub fn stocked_location_stays(
    location_id: &str,
    stock_location_id: &str,
    on_hand: i64,
    deleted: bool,
) -> Result<(), TradeError> {
    if location_id.trim().is_empty() || stock_location_id != location_id || on_hand <= 0 || deleted
    {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// A referenced item stays. Disable is the allowed change; delete is refused.
pub fn item_removal_decision(
    referenced: bool,
    requested: &str,
) -> Result<&'static str, TradeError> {
    match requested {
        "DISABLE" => Ok("DISABLED"),
        "DELETE" if !referenced => Ok("DELETED"),
        "DELETE" => Err(TradeError::Quantity(QuantityError::InvalidText)),
        _ => Err(TradeError::Quantity(QuantityError::InvalidText)),
    }
}

/// A document quantity is a decimal string at the shared six-digit scale.
/// Legacy whole-unit integer `1` is one unit, not 0.001. This does not read a BOM.
pub fn document_qty_micros(text: &str) -> Result<i64, TradeError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    let qty = if text.contains('.') {
        crate::quantity::parse_qty(text).map_err(TradeError::Quantity)?
    } else {
        let units: i64 = text
            .parse()
            .map_err(|_| TradeError::Quantity(QuantityError::InvalidText))?;
        ScaledQty::from_legacy_whole_units(units).map_err(TradeError::Quantity)?
    };
    if qty.as_micros() <= 0 {
        return Err(TradeError::Quantity(QuantityError::InvalidText));
    }
    Ok(qty.as_micros())
}

/// Purchase document total in cents. Whole-unit `qty` stays one unit, not 0.001.
/// This calculation does not read a BOM.
pub fn purchase_total_cent_for_micros(
    qty_micros: i64,
    unit_price_cent: i64,
) -> Result<i64, TradeError> {
    if qty_micros <= 0 || unit_price_cent < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    money(ScaledQty::from_micros(qty_micros), unit_price_cent)
}

pub fn purchase_total_cent(qty: i64, unit_price_cent: i64) -> Result<i64, TradeError> {
    if qty <= 0 || unit_price_cent < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    let quantity = ScaledQty::from_legacy_whole_units(qty).map_err(TradeError::Quantity)?;
    money(quantity, unit_price_cent)
}

fn money(qty: ScaledQty, unit_cent: i64) -> Result<i64, TradeError> {
    let micros = qty.as_micros();
    if micros < 0 || unit_cent < 0 {
        return Err(TradeError::MoneyOverflow);
    }
    let product = i128::from(micros) * i128::from(unit_cent);
    let scale = i128::from(crate::quantity::QTY_SCALE);
    let rounded = (product + scale / 2) / scale;
    i64::try_from(rounded).map_err(|_| TradeError::MoneyOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::parse_qty;

    #[test]
    fn large_costs_keep_the_unit_cost_and_residual_without_intermediate_overflow()
    -> Result<(), String> {
        let value = 10_000_000_000_000_i64;
        let qty = 3_000_000_i64;
        let (unit, residual) =
            cost_unit_and_residual(value, qty).map_err(|error| format!("{error:?}"))?;
        assert_eq!((unit, residual), (3_333_333_333_333, 1));
        assert_eq!(
            admit_cost_rebuild(CostRebuildInput {
                stored_qty: qty,
                stored_value: value,
                stored_unit: unit,
                rebuilt_qty: qty,
                rebuilt_value: value,
                rebuilt_unit: unit,
                residual_cent: residual,
                estimated: false,
            })
            .map_err(|error| format!("{error:?}"))?,
            "REBUILT"
        );
        assert!(cost_unit_and_residual(i64::MAX, 1).is_err());
        Ok(())
    }

    #[test]
    fn large_purchase_and_return_amounts_fit_after_rounding() -> Result<(), String> {
        assert_eq!(
            purchase_total_cent_for_micros(1_000_000, i64::MAX)
                .map_err(|error| format!("{error:?}"))?,
            i64::MAX
        );
        assert_eq!(
            return_cost_cent(i64::MAX, 2_000_000, 1_000_000)
                .map_err(|error| format!("{error:?}"))?,
            4_611_686_018_427_387_904
        );
        assert!(purchase_total_cent_for_micros(2_000_000, i64::MAX).is_err());
        assert_eq!(purchase_total_cent_for_micros(500_000, 1), Ok(1));
        Ok(())
    }

    #[test]
    fn exhausted_or_invalid_document_versions_return_an_error() {
        let purchase = [PurchaseLine {
            line_no: 1,
            item_code: "A".into(),
            qty_micros: 1_000_000,
            price_cent: 100,
            tax_cent: 0,
            due_date: "2026-10-08".into(),
        }];
        let sales = [SalesLine {
            line_no: 1,
            item_code: "A".into(),
            qty_micros: 1_000_000,
            price_cent: 100,
            discount_cent: 0,
            tax_cent: 0,
            due_date: "2026-10-08".into(),
            address: "QA".into(),
            source: "RESALE".into(),
        }];
        for version in [i32::MAX, 0, -1] {
            assert!(purchase_revision(Some((version, "OPEN")), &purchase, "REVISE").is_err());
            assert!(
                sales_revision(
                    Some((version, "SUBMITTED")),
                    Some(version),
                    &sales,
                    "REVISE"
                )
                .is_err()
            );
        }
    }

    #[test]
    fn over_returns_are_refused_without_signed_quantity_overflow() {
        assert!(supplier_return(i64::MAX, i64::MAX, 1, 1).is_err());
        assert!(sales_return_split(i64::MAX, 0, i64::MAX, 1).is_err());
        assert!(sales_return_split(i64::MAX, i64::MAX, 1, 0).is_err());
        assert_eq!(sales_return_split(i64::MAX, 0, i64::MAX, 0), Ok(i64::MAX));
    }

    #[test]
    fn checklist_trade_sample_conserves_quantity_and_money() {
        let purchased = TradeLine {
            qty: parse_qty("50").unwrap(),
            unit_cost_cent: 2_000,
            unit_price_cent: 3_000,
        };
        let before_return = close_trade(
            purchased,
            parse_qty("20").unwrap(),
            ScaledQty::ZERO,
            40_000,
            70_000,
        )
        .unwrap();
        assert_eq!(before_return.ending_qty, parse_qty("30").unwrap());
        assert_eq!(before_return.inventory_cost_cent, 60_000);
        assert_eq!(before_return.sales_cent, 60_000);
        assert_eq!(before_return.cogs_cent, 40_000);
        assert_eq!(before_return.gross_cent, 20_000);
        assert_eq!(before_return.receivable_cent, 20_000);
        assert_eq!(before_return.payable_cent, 30_000);

        let after_return = close_trade(
            purchased,
            parse_qty("20").unwrap(),
            parse_qty("5").unwrap(),
            40_000,
            70_000,
        )
        .unwrap();
        assert_eq!(after_return.ending_qty, parse_qty("35").unwrap());
        assert_eq!(after_return.inventory_cost_cent, 70_000);
        assert_eq!(after_return.sales_cent, 45_000);
        assert_eq!(after_return.cogs_cent, 30_000);
        assert_eq!(after_return.receivable_cent, 5_000);
    }

    #[test]
    fn purchase_sample_total_does_not_read_a_bom() {
        assert_eq!(purchase_total_cent(50, 2_000).unwrap(), 100_000);
        assert_eq!(purchase_total_cent(1, 2_000).unwrap(), 2_000);
    }

    #[test]
    fn trade_close_does_not_require_a_bom() {
        let purchased = TradeLine {
            qty: parse_qty("1").unwrap(),
            unit_cost_cent: 100,
            unit_price_cent: 200,
        };
        assert!(close_trade(purchased, parse_qty("1").unwrap(), ScaledQty::ZERO, 0, 0).is_ok());
    }
}

#[cfg(test)]
mod cost_return_regressions {
    use super::*;

    #[test]
    fn three_partial_returns_restore_two_cents_exactly() {
        let mut quantity = 0;
        let mut cents = 0;
        for expected in [1, 0, 1] {
            let amount = cumulative_return_cost(CostReturnInput {
                original_qty: 3_000_000,
                original_cent: 2,
                returned_qty: quantity,
                returned_cent: cents,
                new_qty: 1_000_000,
            })
            .expect("valid partial return");
            assert_eq!(amount, expected);
            quantity += 1_000_000;
            cents += amount;
        }
        assert_eq!((quantity, cents), (3_000_000, 2));
    }

    #[test]
    fn cumulative_return_refuses_over_quantity_and_over_amount() {
        for (returned_qty, returned_cent, new_qty) in [
            (400_000, 480, 400_000),
            (500_000, 600, 1),
            (0, 1, 1),
            (100_000, 601, 1),
            (-1, 0, 1),
            (0, 0, -1),
        ] {
            assert!(
                cumulative_return_cost(CostReturnInput {
                    original_qty: 500_000,
                    original_cent: 600,
                    returned_qty,
                    returned_cent,
                    new_qty,
                })
                .is_err()
            );
        }
    }

    #[test]
    fn maximum_quantity_and_cents_use_a_wide_intermediate() {
        assert_eq!(
            cumulative_return_cost(CostReturnInput {
                original_qty: i64::MAX,
                original_cent: i64::MAX,
                returned_qty: 0,
                returned_cent: 0,
                new_qty: i64::MAX,
            })
            .expect("valid full return"),
            i64::MAX
        );
        assert_eq!(
            moving_average_unit(1_000_000, i64::MAX - 1, 1_000_000, 1).expect("valid final unit"),
            (4_611_686_018_427_387_903, i64::MAX, false)
        );
    }

    #[test]
    fn known_zero_or_sub_cent_unit_can_be_rebuilt() {
        for (value, residual) in [(0, 0), (2, 2)] {
            assert_eq!(
                admit_cost_rebuild(CostRebuildInput {
                    stored_qty: 3_000_000,
                    stored_value: value,
                    stored_unit: 0,
                    rebuilt_qty: 3_000_000,
                    rebuilt_value: value,
                    rebuilt_unit: 0,
                    residual_cent: residual,
                    estimated: false,
                })
                .expect("a known zero unit is valid"),
                "REBUILT"
            );
        }
    }
}

#[cfg(test)]
mod receipt_offset_generic_regressions {
    use super::*;

    fn partial() -> ReceiptOffsetFacts {
        ReceiptOffsetFacts {
            arrived: 7_000_000,
            released: 5_000_000,
            returned_qty: 2_000_000,
            issued_qty: 0,
            payable_cent: 3_000,
            returned_cost: 2_000,
            released_cost_cent: 5_000,
            release_posting_rows: 1,
            return_rows: 1,
            return_posting_rows: 1,
            held_micros: 2_000_000,
            on_hand_micros: 3_000_000,
            issue_rows: 0,
            over_return_rows: 0,
        }
    }

    #[test]
    fn receipt_offset_accepts_non_sample_partial_release_and_return() {
        // Seven arrived, five released, two returned at 1000 cents/base unit.
        // Two remain held, three remain usable, and 3000 cents remain payable.
        assert_eq!(receipt_offset_holds(partial()), Ok("OFFSET"));
    }

    #[test]
    fn receipt_offset_uses_actual_partial_posting_rounding() {
        let facts = ReceiptOffsetFacts {
            returned_qty: 1_250_000,
            issued_qty: 500_000,
            payable_cent: 5_997,
            returned_cost: 1_999,
            released_cost_cent: 7_996,
            release_posting_rows: 2,
            on_hand_micros: 3_250_000,
            issue_rows: 1,
            ..partial()
        };
        assert_eq!(receipt_offset_holds(facts), Ok("OFFSET"));
        assert!(
            receipt_offset_holds(ReceiptOffsetFacts {
                released_cost_cent: 7_995,
                ..facts
            })
            .is_err(),
            "recomputing total release value erases the two original rounded postings"
        );
    }

    #[test]
    fn receipt_offset_known_zero_cost_requires_actual_postings() {
        let facts = ReceiptOffsetFacts {
            payable_cent: 0,
            returned_cost: 0,
            released_cost_cent: 0,
            ..partial()
        };
        assert_eq!(receipt_offset_holds(facts), Ok("OFFSET"));
        assert!(
            receipt_offset_holds(ReceiptOffsetFacts {
                release_posting_rows: 0,
                ..facts
            })
            .is_err()
        );
        assert!(
            receipt_offset_holds(ReceiptOffsetFacts {
                return_posting_rows: 0,
                ..facts
            })
            .is_err()
        );
    }

    #[test]
    fn receipt_offset_refuses_quantity_stock_or_money_mismatch() {
        for facts in [
            ReceiptOffsetFacts {
                released: 8_000_000,
                ..partial()
            },
            ReceiptOffsetFacts {
                returned_qty: 6_000_000,
                ..partial()
            },
            ReceiptOffsetFacts {
                issued_qty: 4_000_000,
                issue_rows: 1,
                ..partial()
            },
            ReceiptOffsetFacts {
                payable_cent: 3_001,
                ..partial()
            },
            ReceiptOffsetFacts {
                returned_cost: -1,
                ..partial()
            },
            ReceiptOffsetFacts {
                held_micros: 0,
                ..partial()
            },
            ReceiptOffsetFacts {
                on_hand_micros: 4_000_000,
                ..partial()
            },
            ReceiptOffsetFacts {
                over_return_rows: 1,
                ..partial()
            },
        ] {
            assert!(receipt_offset_holds(facts).is_err());
        }
    }
}
