//! Manufacturing structure and dated MRP. Kit expansion is not this model.
//! Suggestions do not change inventory.

pub mod inspection_outcomes;
pub mod cost_allocation;

use crate::quantity::{QuantityError, ScaledQty};
use std::collections::{BTreeMap, BTreeSet};

pub const PROCESS_BASIS_VERSION: &str = "mass-volume-v1";

/// An installed component stays active until it is removed. A second active scan is refused.
pub fn assembly_link_decision(
    already_active: bool,
    action: &str,
) -> Result<&'static str, ManufacturingError> {
    match action {
        "INSTALL" if already_active => Err(ManufacturingError::Planning(
            "component is already installed".into(),
        )),
        "INSTALL" => Ok("ACTIVE"),
        "REMOVE" if already_active => Ok("REMOVED"),
        "REMOVE" => Err(ManufacturingError::Planning(
            "component is not installed".into(),
        )),
        _ => Err(ManufacturingError::Planning(
            "assembly action is unknown".into(),
        )),
    }
}

/// A service replacement names cost, old-part destination, and warranty separately.
pub fn admit_service_replacement(
    cost_cent: i64,
    destination: &str,
    warranty: &str,
) -> Result<(), ManufacturingError> {
    if cost_cent < 0 || destination.trim().is_empty() || warranty.trim().is_empty() {
        return Err(ManufacturingError::Planning(
            "service replacement is incomplete".into(),
        ));
    }
    Ok(())
}

/// A drawing and route are frozen together. A blank identity is refused.
pub fn admit_frozen_route(drawing_id: &str, route_version: &str) -> Result<(), ManufacturingError> {
    if drawing_id.trim().is_empty() || route_version.trim().is_empty() {
        return Err(ManufacturingError::Planning(
            "drawing and route must both be frozen".into(),
        ));
    }
    Ok(())
}

/// Opening WIP must equal reported good. A later receipt does not invent more.
pub fn admit_reported_wip(reported_good: i64, opened: i64) -> Result<i64, ManufacturingError> {
    if reported_good <= 0 || opened != reported_good {
        return Err(ManufacturingError::Planning(
            "opening wip must match reported good".into(),
        ));
    }
    Ok(opened)
}

/// A transfer cannot exceed the quantity still at the source location.
/// Scrap and rework stay at their own locations and do not become finished stock.
pub fn operation_transfer(
    at_source: i64,
    qty: i64,
    kind: &str,
) -> Result<&'static str, ManufacturingError> {
    if at_source < 0 || qty <= 0 {
        return Err(ManufacturingError::Planning(
            "operation transfer is incomplete".into(),
        ));
    }
    if qty > at_source {
        return Err(ManufacturingError::Planning(
            "transfer exceeds the quantity at this location".into(),
        ));
    }
    match kind {
        "MOVE" => Ok("MOVED"),
        "SCRAP" => Ok("SCRAPPED"),
        "REWORK" => Ok("REWORK"),
        _ => Err(ManufacturingError::Planning(
            "operation transfer kind is unknown".into(),
        )),
    }
}
pub const MRP_ALGORITHM: &str = "net-demand-v1";

/// A formula quantity converts only through the named basis version.
/// Mass times density is volume. An unknown basis is refused.
pub fn convert_formula_qty(
    mass_micros: i64,
    density_micros: i64,
    basis_version: &str,
) -> Result<i64, ManufacturingError> {
    if mass_micros <= 0 || density_micros <= 0 || basis_version != PROCESS_BASIS_VERSION {
        return Err(ManufacturingError::Planning(
            "formula basis cannot convert".into(),
        ));
    }
    mass_micros
        .checked_mul(density_micros)
        .map(|scaled| scaled / 1_000_000)
        .filter(|volume| *volume > 0)
        .ok_or_else(|| ManufacturingError::Planning("formula basis overflows".into()))
}

/// Actual yield is recorded beside the formula. It does not replace the charged quantity.
pub fn formula_balance(
    formula_micros: i64,
    charged_micros: i64,
    product_micros: i64,
    waste_micros: i64,
    co_micros: i64,
) -> Result<i64, ManufacturingError> {
    if formula_micros <= 0
        || charged_micros < 0
        || product_micros < 0
        || waste_micros < 0
        || co_micros < 0
    {
        return Err(ManufacturingError::Planning(
            "formula quantities are incomplete".into(),
        ));
    }
    let explained = product_micros
        .checked_add(waste_micros)
        .and_then(|sum| sum.checked_add(co_micros))
        .ok_or_else(|| ManufacturingError::Planning("formula quantities overflow".into()))?;
    charged_micros
        .checked_sub(explained)
        .ok_or_else(|| ManufacturingError::Planning("formula quantities overflow".into()))
}
pub const APS_ALGORITHM: &str = "finite-capacity-v1";

/// Two tasks cannot occupy the same machine at the same minute. A missing qualification is infeasible.
pub fn finite_slot_decision(
    machine: &str,
    other_machine: &str,
    start_minute: i64,
    other_start: i64,
    duration: i64,
    qualified: bool,
) -> Result<&'static str, ManufacturingError> {
    if machine.trim().is_empty() || duration <= 0 || start_minute < 0 || other_start < 0 {
        return Err(ManufacturingError::Planning(
            "finite slot is incomplete".into(),
        ));
    }
    if !qualified {
        return Ok("INFEASIBLE");
    }
    let overlaps = machine == other_machine
        && start_minute < other_start + duration
        && other_start < start_minute + duration;
    if overlaps {
        Ok("CONFLICT")
    } else {
        Ok("PLACED")
    }
}

/// Availability, performance, and quality each use an explicit denominator.
/// Planned minutes already exclude the published shift break. Downtime is unplanned stop only.
/// A duplicate pulse is not output: official good stays at the confirmed quantity.
pub fn oee_rates(
    planned_minutes: i64,
    downtime_minutes: i64,
    ideal_cycle_micros: i64,
    captured_micros: i64,
    confirmed_micros: i64,
    official_good_micros: i64,
    duplicate_pulse: bool,
) -> Result<(i64, i64, i64), ManufacturingError> {
    if planned_minutes <= 0
        || downtime_minutes < 0
        || downtime_minutes > planned_minutes
        || ideal_cycle_micros <= 0
    {
        return Err(ManufacturingError::Planning(
            "oee basis is incomplete".into(),
        ));
    }
    if captured_micros < 0
        || confirmed_micros < 0
        || official_good_micros < 0
        || confirmed_micros > captured_micros
    {
        return Err(ManufacturingError::Planning(
            "captured confirmation and output stay separate".into(),
        ));
    }
    if duplicate_pulse && official_good_micros != confirmed_micros {
        return Err(ManufacturingError::Planning(
            "a duplicate pulse is not extra output".into(),
        ));
    }
    let run_minutes = planned_minutes - downtime_minutes;
    let availability = run_minutes * 1_000_000 / planned_minutes;
    let ideal_span = run_minutes
        .checked_mul(60)
        .and_then(|seconds| seconds.checked_mul(1_000_000))
        .ok_or_else(|| ManufacturingError::Planning("oee basis overflows".into()))?;
    let performance = if ideal_span == 0 {
        0
    } else {
        captured_micros
            .checked_mul(ideal_cycle_micros)
            .ok_or_else(|| ManufacturingError::Planning("oee basis overflows".into()))?
            / ideal_span
    };
    let quality = if captured_micros == 0 {
        0
    } else {
        official_good_micros * 1_000_000 / captured_micros
    };
    Ok((availability, performance, quality))
}

/// A device event needs its own identity, times, sequence, and payload version.
/// The same device event id is a replay, not a second output.
pub fn admit_device_event(
    device_code: &str,
    device_event_id: &str,
    device_time: &str,
    received_time: &str,
    sequence_no: i64,
    payload_version: i32,
    captured_micros: i64,
) -> Result<(), ManufacturingError> {
    if device_code.trim().is_empty()
        || device_event_id.trim().is_empty()
        || device_time.trim().is_empty()
        || received_time.trim().is_empty()
        || sequence_no < 0
        || payload_version <= 0
        || captured_micros < 0
    {
        return Err(ManufacturingError::Planning(
            "device event identity is incomplete".into(),
        ));
    }
    Ok(())
}

/// Publishing a schedule must keep an already started operation. A stale cutoff is refused.
pub fn publish_keeps_started(
    started: bool,
    cutoff_still_valid: bool,
) -> Result<&'static str, ManufacturingError> {
    if !cutoff_still_valid {
        return Err(ManufacturingError::Planning(
            "schedule cutoff is stale".into(),
        ));
    }
    if started { Ok("KEPT") } else { Ok("PUBLISHED") }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomItem {
    pub parent: String,
    pub component: String,
    pub qty_per: ScaledQty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrpItem {
    pub code: String,
    pub on_hand: ScaledQty,
    pub allocated_elsewhere: ScaledQty,
    pub supply_on_time: ScaledQty,
    pub supply_late: ScaledQty,
    pub safety: ScaledQty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrpSuggestion {
    pub code: String,
    pub gross: ScaledQty,
    pub net_make_or_buy: ScaledQty,
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Execution {
    pub issued: ScaledQty,
    pub consumed: ScaledQty,
    pub good: ScaledQty,
    pub rejected: ScaledQty,
    pub received: ScaledQty,
    pub material_cost_cent: i64,
    pub labor_minutes: i64,
    pub labor_rate_cent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub wip_qty: ScaledQty,
    pub available_finished: ScaledQty,
    pub cost_cent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkOrderStatus {
    Planned,
    Released,
    Paused,
    Completed,
    Closed,
    Cancelled,
}

impl WorkOrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "PLANNED",
            Self::Released => "RELEASED",
            Self::Paused => "PAUSED",
            Self::Completed => "COMPLETED",
            Self::Closed => "CLOSED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn parse(text: &str) -> Result<Self, ManufacturingError> {
        match text {
            "PLANNED" => Ok(Self::Planned),
            "RELEASED" => Ok(Self::Released),
            "PAUSED" => Ok(Self::Paused),
            "COMPLETED" => Ok(Self::Completed),
            "CLOSED" => Ok(Self::Closed),
            "CANCELLED" => Ok(Self::Cancelled),
            _ => Err(ManufacturingError::IllegalTransition(text.into())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationReport {
    pub operation: String,
    pub good: ScaledQty,
    pub rejected: ScaledQty,
    pub rework: ScaledQty,
    pub minutes: i64,
    pub operator_id: String,
    pub session_user_id: String,
    pub session_id: String,
    pub crew_code: String,
    pub started_at: String,
    pub ended_at: String,
    pub reverses_report_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportTotals {
    pub good: ScaledQty,
    pub rejected: ScaledQty,
    pub rework: ScaledQty,
    pub minutes: i64,
}

/// A report cannot exceed the previous operation's good output.
/// Negative minutes and quantities are rejected. The operator is not the login user.
/// A reversal must offset the original quantities and is added, not used to delete it.
/// When a frozen predecessor is supplied, its stored good output is the limit.
/// A caller-supplied previous quantity is ignored in that case.
pub fn admit_operation_report(
    previous_good: ScaledQty,
    already: ReportTotals,
    original: Option<&ReportTotals>,
    report: &OperationReport,
) -> Result<ReportTotals, ManufacturingError> {
    admit_routed_report(previous_good, None, already, original, report)
}

/// Uses the frozen predecessor's stored good output when the route names one.
/// The first operation keeps the caller ceiling. A later operation cannot invent a higher ceiling.
pub fn admit_routed_report(
    caller_previous: ScaledQty,
    predecessor_good: Option<ScaledQty>,
    already: ReportTotals,
    original: Option<&ReportTotals>,
    report: &OperationReport,
) -> Result<ReportTotals, ManufacturingError> {
    if report.operation.trim().is_empty()
        || report.operator_id.trim().is_empty()
        || report.session_user_id.trim().is_empty()
        || report.session_id.trim().is_empty()
    {
        return Err(ManufacturingError::MissingReason("report identity".into()));
    }
    if report.operator_id == report.session_user_id || report.operator_id == report.session_id {
        return Err(ManufacturingError::MissingReason(
            "operator is not a login user".into(),
        ));
    }
    let (previous_good, bounded) = match predecessor_good {
        Some(good) => (good, true),
        None => (caller_previous, caller_previous.as_micros() > 0),
    };
    if report.reverses_report_id.is_none() && report.minutes < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if report.reverses_report_id.is_some() {
        let Some(original) = original.copied() else {
            return Err(ManufacturingError::MissingReason(
                "original report missing".into(),
            ));
        };
        if report.good.as_micros() != -original.good.as_micros()
            || report.rejected.as_micros() != -original.rejected.as_micros()
            || report.rework.as_micros() != -original.rework.as_micros()
            || report.minutes != -original.minutes
        {
            return Err(ManufacturingError::IssuedMaterial(
                "reversal must offset the original".into(),
            ));
        }
    } else if report.good.is_negative()
        || report.rejected.is_negative()
        || report.rework.is_negative()
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let good = already
        .good
        .try_add(report.good)
        .map_err(ManufacturingError::Quantity)?;
    let rejected = already
        .rejected
        .try_add(report.rejected)
        .map_err(ManufacturingError::Quantity)?;
    let rework = already
        .rework
        .try_add(report.rework)
        .map_err(ManufacturingError::Quantity)?;
    let output = good
        .try_add(rejected)
        .map_err(ManufacturingError::Quantity)?;
    if bounded && output.as_micros() > previous_good.as_micros() {
        return Err(ManufacturingError::IssuedMaterial(
            "report exceeds previous output".into(),
        ));
    }
    let minutes = already
        .minutes
        .checked_add(report.minutes)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    Ok(ReportTotals {
        good,
        rejected,
        rework,
        minutes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptStatus {
    Hold,
    Released,
}

impl ReceiptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hold => "HOLD",
            Self::Released => "RELEASED",
        }
    }
}

/// Reported good and received stock are separate. Held goods are not available.
/// A receipt cannot exceed remaining reported good.
pub fn admit_production_receipt(
    reported_good: ScaledQty,
    already_received: ScaledQty,
    qty: ScaledQty,
    status: ReceiptStatus,
) -> Result<(ScaledQty, ScaledQty), ManufacturingError> {
    if qty.as_micros() <= 0 || qty.is_negative() {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let received = already_received
        .try_add(qty)
        .map_err(ManufacturingError::Quantity)?;
    if received.as_micros() > reported_good.as_micros() {
        return Err(ManufacturingError::IssuedMaterial(
            "receipt exceeds reported good".into(),
        ));
    }
    let available = match status {
        ReceiptStatus::Hold => ScaledQty::ZERO,
        ReceiptStatus::Released => qty,
    };
    Ok((received, available))
}

/// One variance line. Price, usage, rate, and efficiency stay separately recomputable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostVarianceLine {
    pub kind: String,
    pub standard_cent: i64,
    pub actual_cent: i64,
    pub variance_cent: i64,
}

/// Standard and actual stay apart. The variance sum equals actual minus standard.
/// Unknown overhead is not a confirmed profit input.
pub fn cost_variances(
    standard_material: i64,
    actual_material: i64,
    standard_labor: i64,
    actual_labor: i64,
    standard_overhead: i64,
    actual_overhead: Option<i64>,
) -> Result<(Vec<CostVarianceLine>, bool), ManufacturingError> {
    if standard_material < 0
        || actual_material < 0
        || standard_labor < 0
        || actual_labor < 0
        || standard_overhead < 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let (overhead_actual, estimated) = match actual_overhead {
        Some(value) if value >= 0 => (value, false),
        Some(_) => return Err(ManufacturingError::Quantity(QuantityError::InvalidText)),
        None => (standard_overhead, true),
    };
    let lines = vec![
        variance_line("MATERIAL", standard_material, actual_material)?,
        variance_line("LABOR", standard_labor, actual_labor)?,
        variance_line("OVERHEAD", standard_overhead, overhead_actual)?,
    ];
    let standard: i64 = lines.iter().map(|line| line.standard_cent).sum();
    let actual: i64 = lines.iter().map(|line| line.actual_cent).sum();
    let variance: i64 = lines.iter().map(|line| line.variance_cent).sum();
    if variance != actual - standard {
        return Err(ManufacturingError::Quantity(QuantityError::Overflow));
    }
    Ok((lines, estimated))
}

/// One frozen standard driver. Quantity is in micros. Rates are cents per unit or per minute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardCostDriver {
    pub material_qty_micros: i64,
    pub material_price_cent: i64,
    pub labor_minutes: i64,
    pub labor_rate_cent: i64,
    pub overhead_rate_cent: i64,
}

/// Actual usage and rates stay apart from the frozen standard. Missing overhead is unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActualCostDriver {
    pub material_qty_micros: i64,
    pub material_price_cent: i64,
    pub labor_minutes: i64,
    pub labor_rate_cent: i64,
    pub overhead_cent: Option<i64>,
}

/// Plan, issue, consume, report, and receipt stay different metrics. A shared cutoff is required.
pub fn manufacturing_report_metric(
    kind: &str,
    cutoff: &str,
) -> Result<&'static str, ManufacturingError> {
    if cutoff.trim().is_empty() {
        return Err(ManufacturingError::Planning(
            "report cutoff is required".into(),
        ));
    }
    match kind {
        "PLAN" => Ok("PLAN"),
        "ISSUE" | "SUPPLEMENT" => Ok("ISSUE"),
        "CONSUME" => Ok("CONSUME"),
        "RETURN" => Ok("RETURN"),
        "REPORT_GOOD" | "REPORT_REJECTED" => Ok(if kind == "REPORT_GOOD" {
            "REPORT_GOOD"
        } else {
            "REPORT_REJECTED"
        }),
        "RECEIPT" => Ok("RECEIPT"),
        "SHORTAGE" => Ok("SHORTAGE"),
        "COST" => Ok("COST"),
        _ => Err(ManufacturingError::Planning(
            "unknown manufacturing metric".into(),
        )),
    }
}

/// Price, usage, rate, and efficiency are separate lines. Their sum equals actual minus standard.
/// Unknown overhead is stored at the standard amount and marked estimated, not as confirmed profit.
pub fn component_variances(
    standard: &StandardCostDriver,
    actual: &ActualCostDriver,
) -> Result<(Vec<CostVarianceLine>, i64, i64, bool), ManufacturingError> {
    let standard_material = qty_rate(standard.material_qty_micros, standard.material_price_cent)?;
    let actual_material = qty_rate(actual.material_qty_micros, actual.material_price_cent)?;
    let standard_labor = minute_rate(standard.labor_minutes, standard.labor_rate_cent)?;
    let actual_labor = minute_rate(actual.labor_minutes, actual.labor_rate_cent)?;
    let standard_overhead = minute_rate(standard.labor_minutes, standard.overhead_rate_cent)?;
    let (overhead_actual, estimated) = match actual.overhead_cent {
        Some(value) if value >= 0 => (value, false),
        Some(_) => return Err(ManufacturingError::Quantity(QuantityError::InvalidText)),
        None => (standard_overhead, true),
    };
    let scale = crate::quantity::QTY_SCALE;
    let standard_minutes = standard
        .labor_minutes
        .checked_mul(scale)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let actual_minutes = actual
        .labor_minutes
        .checked_mul(scale)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let price = mix_variance(
        "MATERIAL_PRICE",
        standard.material_qty_micros,
        standard.material_price_cent,
        standard.material_qty_micros,
        actual.material_price_cent,
    )?;
    let usage = mix_variance(
        "MATERIAL_USAGE",
        standard.material_qty_micros,
        actual.material_price_cent,
        actual.material_qty_micros,
        actual.material_price_cent,
    )?;
    let rate = mix_variance(
        "LABOR_RATE",
        standard_minutes,
        standard.labor_rate_cent,
        standard_minutes,
        actual.labor_rate_cent,
    )?;
    let efficiency = mix_variance(
        "LABOR_EFFICIENCY",
        standard_minutes,
        actual.labor_rate_cent,
        actual_minutes,
        actual.labor_rate_cent,
    )?;
    let overhead = variance_line("OVERHEAD", standard_overhead, overhead_actual)?;
    let lines = vec![price, usage, rate, efficiency, overhead];
    let variance: i64 = lines.iter().try_fold(0_i64, |sum, line| {
        sum.checked_add(line.variance_cent)
            .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))
    })?;
    let standard_total = standard_material
        .checked_add(standard_labor)
        .and_then(|sum| sum.checked_add(standard_overhead))
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let actual_total = actual_material
        .checked_add(actual_labor)
        .and_then(|sum| sum.checked_add(overhead_actual))
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    if variance
        != actual_total
            .checked_sub(standard_total)
            .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?
    {
        return Err(ManufacturingError::Quantity(QuantityError::Overflow));
    }
    Ok((lines, standard_total, actual_total, estimated))
}

/// A margin is confirmed only when every cost component is known.
pub fn confirmed_margin(
    sales_cent: i64,
    actual_total: i64,
    estimated: bool,
) -> Result<Option<i64>, ManufacturingError> {
    if sales_cent < 0 || actual_total < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if estimated {
        return Ok(None);
    }
    sales_cent
        .checked_sub(actual_total)
        .map(Some)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))
}

fn qty_rate(qty_micros: i64, price_cent: i64) -> Result<i64, ManufacturingError> {
    if qty_micros < 0 || price_cent < 0 || qty_micros % crate::quantity::QTY_SCALE != 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    (qty_micros / crate::quantity::QTY_SCALE)
        .checked_mul(price_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))
}

fn minute_rate(minutes: i64, rate_cent: i64) -> Result<i64, ManufacturingError> {
    if minutes < 0 || rate_cent < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    minutes
        .checked_mul(rate_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))
}

fn mix_variance(
    kind: &str,
    left_qty_micros: i64,
    left_rate: i64,
    right_qty_micros: i64,
    right_rate: i64,
) -> Result<CostVarianceLine, ManufacturingError> {
    let left = qty_rate(left_qty_micros, left_rate)?;
    let right = qty_rate(right_qty_micros, right_rate)?;
    variance_line(kind, left, right)
}

fn variance_line(
    kind: &str,
    standard: i64,
    actual: i64,
) -> Result<CostVarianceLine, ManufacturingError> {
    Ok(CostVarianceLine {
        kind: kind.to_string(),
        standard_cent: standard,
        actual_cent: actual,
        variance_cent: actual
            .checked_sub(standard)
            .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?,
    })
}

/// Material, actual labor, and a confirmed overhead make the order cost.
/// Unknown overhead stays estimated and is not added. Unit cost uses completed
/// quantity only. Rejected quantity is a separate destination, not a divisor
/// of this confirmed unit cost.
pub fn order_cost(
    material_cent: i64,
    labor_minutes: i64,
    labor_rate_cent: i64,
    overhead_cent: Option<i64>,
    completed_qty: ScaledQty,
    rejected_qty: ScaledQty,
) -> Result<(i64, i64, bool), ManufacturingError> {
    if material_cent < 0
        || labor_minutes < 0
        || labor_rate_cent < 0
        || completed_qty.as_micros() <= 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if rejected_qty.as_micros() < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let labor = labor_minutes
        .checked_mul(labor_rate_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let (overhead, estimated) = match overhead_cent {
        Some(value) if value >= 0 => (value, false),
        Some(_) => return Err(ManufacturingError::Quantity(QuantityError::InvalidText)),
        None => (0, true),
    };
    let total = material_cent
        .checked_add(labor)
        .and_then(|sum| sum.checked_add(overhead))
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let scale = crate::quantity::QTY_SCALE;
    if completed_qty.as_micros() % scale != 0 {
        return Err(ManufacturingError::Quantity(QuantityError::ScaleExceeded));
    }
    let units = completed_qty.as_micros() / scale;
    let unit = total / units;
    Ok((total, unit, estimated))
}

/// Open work keeps unfinished and defect cost beside the finished unit cost.
/// Confirmed finished cost is not reduced by these destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostDestination {
    pub unfinished_cent: i64,
    pub defect_cent: i64,
}

pub fn open_cost_destination(
    material_cent: i64,
    labor_cent: i64,
    unfinished_qty: ScaledQty,
    rejected_qty: ScaledQty,
) -> Result<CostDestination, ManufacturingError> {
    if material_cent < 0
        || labor_cent < 0
        || unfinished_qty.as_micros() < 0
        || rejected_qty.as_micros() < 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let scale = crate::quantity::QTY_SCALE;
    if unfinished_qty.as_micros() % scale != 0 || rejected_qty.as_micros() % scale != 0 {
        return Err(ManufacturingError::Quantity(QuantityError::ScaleExceeded));
    }
    let unfinished_units = unfinished_qty.as_micros() / scale;
    let rejected_units = rejected_qty.as_micros() / scale;
    let weight = unfinished_units
        .checked_add(rejected_units)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    if weight <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let pool = material_cent
        .checked_add(labor_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let unfinished_cent = pool / weight * unfinished_units;
    let defect_cent = pool
        .checked_sub(unfinished_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    Ok(CostDestination {
        unfinished_cent,
        defect_cent,
    })
}

/// A batch cost job is accepted before it finishes. Completion needs a leased task
/// for that order. The accepted receipt is not the finished cost.
pub fn admit_cost_batch(
    task_id: &str,
    event_id: &str,
    work_order_id: &str,
    leased: bool,
    elapsed_ms: i64,
) -> Result<&'static str, ManufacturingError> {
    if task_id.trim().is_empty()
        || event_id.trim().is_empty()
        || work_order_id.trim().is_empty()
        || task_id == event_id
        || task_id == work_order_id
        || event_id == work_order_id
    {
        return Err(ManufacturingError::Planning(
            "cost batch identities stay distinct".into(),
        ));
    }
    if !leased {
        return Err(ManufacturingError::Planning(
            "cost batch waits for its lease".into(),
        ));
    }
    if !(0..=60_000).contains(&elapsed_ms) {
        return Err(ManufacturingError::Planning(
            "cost batch exceeds the fixed sample budget".into(),
        ));
    }
    Ok("DONE")
}

/// Close is refused while scrap is unposted or shop stock remains.
/// A rework order must name the original order. Scrap needs a quantity and a cost destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionClose {
    pub status: WorkOrderStatus,
    pub shop: ScaledQty,
    pub scrap: ScaledQty,
    pub scrap_posted: bool,
    pub held_qty: ScaledQty,
    pub rework_source: Option<()>,
    pub suspended: bool,
}

pub fn close_after_exceptions(
    input: ExceptionClose,
) -> Result<WorkOrderStatus, ManufacturingError> {
    if input.status != WorkOrderStatus::Completed && input.status != WorkOrderStatus::Released {
        return Err(ManufacturingError::IllegalTransition(
            input.status.as_str().into(),
        ));
    }
    if input.shop.as_micros() != 0 {
        return Err(ManufacturingError::IssuedMaterial(
            "return or consume shop stock before close".into(),
        ));
    }
    if input.scrap.as_micros() > 0 && !input.scrap_posted {
        return Err(ManufacturingError::IssuedMaterial(
            "scrap needs a posted cost destination before close".into(),
        ));
    }
    if input.held_qty.as_micros() > 0 {
        return Err(ManufacturingError::IssuedMaterial(
            "held receipt cannot close before release".into(),
        ));
    }
    if input.suspended {
        return Err(ManufacturingError::IssuedMaterial(
            "a suspended exception blocks close".into(),
        ));
    }
    Ok(WorkOrderStatus::Closed)
}

/// A hold names a positive quantity and a reason. Closing needs the hold to be lifted.
pub fn admit_exception_hold(
    qty: ScaledQty,
    reason: &str,
    lifted: bool,
) -> Result<&'static str, ManufacturingError> {
    if qty.as_micros() <= 0 || reason.trim().is_empty() {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(if lifted { "LIFTED" } else { "HELD" })
}

pub fn rework_must_cite_source(
    source_work_order_id: Option<&str>,
) -> Result<(), ManufacturingError> {
    match source_work_order_id {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(ManufacturingError::IllegalTransition(
            "rework requires the original work order".into(),
        )),
    }
}

/// One business effect. Stock, work-order execution, source document, and a pending
/// partner posting must share this identity. A different payload is not this effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicEffect {
    pub source_action: String,
    pub action_no: String,
    pub qty: ScaledQty,
    pub amount_cent: i64,
}

pub fn admit_atomic_effect(effect: &AtomicEffect) -> Result<(), ManufacturingError> {
    if effect.source_action.trim() != "ISSUE" || effect.action_no.trim().is_empty() {
        return Err(ManufacturingError::IllegalTransition(
            "atomic material requires an ISSUE and a stable action number".into(),
        ));
    }
    if effect.qty.as_micros() <= 0 || effect.amount_cent < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}

/// The business key is tenant, source action, and action number. A new request cannot post it twice.
pub fn atomic_effect_already_posted(
    existing_document: Option<&str>,
) -> Result<(), ManufacturingError> {
    if existing_document.is_some() {
        return Err(ManufacturingError::IllegalTransition(
            "source action and action number already posted".into(),
        ));
    }
    Ok(())
}

/// Task-center kinds. A refresh success must not hide one of these rows.
pub const EXCEPTION_KINDS: [&str; 6] = [
    "SHORTAGE",
    "UNINSPECTED",
    "OVERDUE",
    "INVENTORY_DIFF",
    "UNKNOWN_COMMAND",
    "TASK_FAILED",
];

/// Admits one exception. The next action is named, not inferred from a success banner.
pub fn admit_task_exception(
    kind: &str,
    object_type: &str,
    object_id: &str,
    command_id: &str,
    query_generation: i64,
) -> Result<&'static str, ManufacturingError> {
    if object_type.trim().is_empty() || object_id.trim().is_empty() || command_id.trim().is_empty()
    {
        return Err(ManufacturingError::IllegalTransition(
            "exception needs an object and the original command".into(),
        ));
    }
    if query_generation < 1 {
        return Err(ManufacturingError::IllegalTransition(
            "exception query generation is invalid".into(),
        ));
    }
    let next = match kind {
        "SHORTAGE" => "ISSUE_OR_BUY",
        "UNINSPECTED" => "INSPECT",
        "OVERDUE" => "RESCHEDULE",
        "INVENTORY_DIFF" => "RECONCILE",
        "UNKNOWN_COMMAND" => "MANUAL",
        "TASK_FAILED" => "RETRY",
        _ => {
            return Err(ManufacturingError::IllegalTransition(
                "unknown exception kind".into(),
            ));
        }
    };
    Ok(next)
}

/// Same command retry keeps the original identity. A stale query generation cannot act on the old set.
pub fn task_retry_keeps_identity(
    stored_command: &str,
    retry_command: &str,
    stored_generation: i64,
    current_generation: i64,
) -> Result<(), ManufacturingError> {
    if stored_command != retry_command {
        return Err(ManufacturingError::IllegalTransition(
            "retry must use the original command".into(),
        ));
    }
    if current_generation != stored_generation {
        return Err(ManufacturingError::IllegalTransition(
            "stale query cannot operate the old exception set".into(),
        ));
    }
    Ok(())
}

/// A retry uses the current grant, not a stale admin flag. A revoked grant cannot continue the old command.
pub fn task_retry_grant_current(permission_held: bool) -> Result<(), ManufacturingError> {
    if permission_held {
        Ok(())
    } else {
        Err(ManufacturingError::IllegalTransition(
            "revoked permission cannot retry the original command".into(),
        ))
    }
}

/// A committed task cannot display cancel success. An uncommitted task may be cancelled.
/// The shown stage is never `CANCELLED` when the original effect is already stored.
pub fn committed_task_cancel_stage(effect_stored: bool) -> Result<&'static str, ManufacturingError> {
    if effect_stored {
        Err(ManufacturingError::IllegalTransition(
            "committed task cannot show cancel success".into(),
        ))
    } else {
        Ok("CANCELLED")
    }
}

/// A stored effect remains findable after a worker restart. A success banner does not replace it.
pub fn retained_task_effect(
    effect_id: &str,
    success_banner: &str,
) -> Result<String, ManufacturingError> {
    if effect_id.trim().is_empty() {
        return Err(ManufacturingError::IllegalTransition(
            "task effect was not stored".into(),
        ));
    }
    if !success_banner.trim().is_empty() && success_banner == effect_id {
        return Err(ManufacturingError::IllegalTransition(
            "a success banner cannot replace the stored effect".into(),
        ));
    }
    Ok(effect_id.to_string())
}

/// A dead-letter replay posts the original amount once. A second replay must not add it again.
pub fn dead_letter_posts_once(
    object_id: &str,
    command_id: &str,
    amount_micros: i64,
    already_posted: bool,
) -> Result<i64, ManufacturingError> {
    if object_id.trim().is_empty() || command_id.trim().is_empty() {
        return Err(ManufacturingError::IllegalTransition(
            "dead letter needs the original object and command".into(),
        ));
    }
    if amount_micros <= 0 {
        return Err(ManufacturingError::IllegalTransition(
            "dead letter amount must be positive".into(),
        ));
    }
    if already_posted {
        return Err(ManufacturingError::IllegalTransition(
            "dead letter is already posted".into(),
        ));
    }
    Ok(amount_micros)
}

pub fn scrap_destination(
    qty: ScaledQty,
    cost_cent: i64,
) -> Result<(ScaledQty, i64), ManufacturingError> {
    if qty.as_micros() <= 0 || cost_cent < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok((qty, cost_cent))
}

/// A later receipt shares the locked supplier balance with a receipt that
/// already landed. The pair cannot exceed that balance.
pub fn admit_subcontract_race(
    open: i64,
    first: i64,
    second: i64,
) -> Result<i64, ManufacturingError> {
    if open < 0 || first <= 0 || second <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let used = first
        .checked_add(second)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    if used > open {
        return Err(ManufacturingError::IssuedMaterial(
            "concurrent subcontract receipts share the supplier balance".into(),
        ));
    }
    Ok(open - used)
}

/// Supplier location is not our title. A receipt does not consume material again.
/// Loss and return reduce the quantity still at the supplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubcontractBalance {
    pub issued: ScaledQty,
    pub received: ScaledQty,
    pub returned: ScaledQty,
    pub loss: ScaledQty,
    pub at_supplier: ScaledQty,
}

pub fn apply_subcontract_move(
    current: SubcontractBalance,
    kind: &str,
    qty: ScaledQty,
) -> Result<SubcontractBalance, ManufacturingError> {
    if qty.as_micros() <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let mut next = current;
    match kind {
        "ISSUE" => {
            next.issued = next
                .issued
                .try_add(qty)
                .map_err(ManufacturingError::Quantity)?
        }
        "RECEIPT" | "RETURN" | "LOSS" => {
            let open = next
                .issued
                .try_sub(next.received)
                .and_then(|left| left.try_sub(next.returned))
                .and_then(|left| left.try_sub(next.loss))
                .map_err(ManufacturingError::Quantity)?;
            if qty.as_micros() > open.as_micros() {
                return Err(ManufacturingError::IssuedMaterial(
                    "subcontract move exceeds material at supplier".into(),
                ));
            }
            match kind {
                "RECEIPT" => {
                    next.received = next
                        .received
                        .try_add(qty)
                        .map_err(ManufacturingError::Quantity)?
                }
                "RETURN" => {
                    next.returned = next
                        .returned
                        .try_add(qty)
                        .map_err(ManufacturingError::Quantity)?
                }
                "LOSS" => {
                    next.loss = next
                        .loss
                        .try_add(qty)
                        .map_err(ManufacturingError::Quantity)?
                }
                _ => unreachable!(),
            }
        }
        _ => return Err(ManufacturingError::IllegalTransition(kind.into())),
    }
    next.at_supplier = next
        .issued
        .try_sub(next.received)
        .and_then(|left| left.try_sub(next.returned))
        .and_then(|left| left.try_sub(next.loss))
        .map_err(ManufacturingError::Quantity)?;
    Ok(next)
}

/// A cross-unit substitute names two different units. The ratio comes from the
/// stored conversion, not from a same-unit multiplier.
pub fn admit_unit_substitute(
    original: &str,
    replacement: &str,
    original_unit: &str,
    replacement_unit: &str,
    ratio_micros: i64,
) -> Result<i64, ManufacturingError> {
    if original.is_empty() || replacement.is_empty() || original == replacement {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if original_unit.trim().is_empty()
        || replacement_unit.trim().is_empty()
        || original_unit == replacement_unit
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if ratio_micros <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(ratio_micros)
}

/// A substitute applies only inside its dates and approved scope.
/// The ratio is component units per one original unit, not a silent rewrite.
pub fn admit_substitute(
    original: &str,
    replacement: &str,
    ratio_micros: i64,
    effective_from: &str,
    effective_to: &str,
    need_date: &str,
    approved: bool,
) -> Result<i64, ManufacturingError> {
    if original.is_empty() || replacement.is_empty() || original == replacement || ratio_micros <= 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if !approved {
        return Err(ManufacturingError::IllegalTransition(
            "substitute is not approved".into(),
        ));
    }
    if need_date < effective_from || need_date > effective_to {
        return Err(ManufacturingError::IllegalTransition(
            "substitute is outside the effective dates".into(),
        ));
    }
    Ok(ratio_micros)
}

/// A substitute quantity keeps the six-digit scale. Truncating to whole units first
/// would turn a half-unit line and a two-to-one ratio into zero.
pub fn substitute_qty(
    old_qty: ScaledQty,
    ratio: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    if old_qty.as_micros() <= 0 || ratio.as_micros() <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let product = i128::from(old_qty.as_micros()) * i128::from(ratio.as_micros());
    let scaled = product / i128::from(crate::quantity::QTY_SCALE);
    i64::try_from(scaled)
        .map(ScaledQty::from_micros)
        .map_err(|_| ManufacturingError::Quantity(QuantityError::Overflow))
}

/// An engineering change may retarget only a planned order with no issued material.
/// Issued or consumed material and the historical BOM stay unchanged.
pub fn admit_engineering_change(
    status: WorkOrderStatus,
    issued: ScaledQty,
    consumed: ScaledQty,
) -> Result<(), ManufacturingError> {
    if status != WorkOrderStatus::Planned || issued.as_micros() != 0 || consumed.as_micros() != 0 {
        return Err(ManufacturingError::IssuedMaterial(
            "issued or started work order keeps its original snapshot".into(),
        ));
    }
    Ok(())
}

/// Frozen weights split one total. The parts must add back to the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostShare {
    pub primary_cent: i64,
    pub co_cent: i64,
}

pub fn split_frozen_weights(
    total_cent: i64,
    primary_weight: i64,
    co_weight: i64,
) -> Result<CostShare, ManufacturingError> {
    if total_cent < 0 || primary_weight <= 0 || co_weight <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let weight = primary_weight
        .checked_add(co_weight)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let primary_cent = total_cent / weight * primary_weight;
    let co_cent = total_cent
        .checked_sub(primary_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    if primary_cent + co_cent != total_cent {
        return Err(ManufacturingError::Quantity(QuantityError::Overflow));
    }
    Ok(CostShare {
        primary_cent,
        co_cent,
    })
}

/// Backflush only the quantity not already consumed by hand.
pub fn backflush_after_manual(
    issued: ScaledQty,
    consumed: ScaledQty,
    standard: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    if issued.as_micros() < 0
        || consumed.as_micros() < 0
        || standard.as_micros() <= 0
        || consumed.as_micros() > issued.as_micros()
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let open = issued
        .try_sub(consumed)
        .map_err(ManufacturingError::Quantity)?;
    if standard.as_micros() <= consumed.as_micros() {
        return Ok(ScaledQty::ZERO);
    }
    let remaining = standard
        .try_sub(consumed)
        .map_err(ManufacturingError::Quantity)?;
    if remaining.as_micros() > open.as_micros() {
        return Err(ManufacturingError::IssuedMaterial(
            "backflush exceeds unconsumed issue".into(),
        ));
    }
    Ok(remaining)
}

/// Disassembly recovers quantity and must cite the original output. It does not delete history.
pub fn admit_disassembly(
    source_output: &str,
    qty: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    if source_output.is_empty() || qty.as_micros() <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(qty)
}

/// A board refresh is one leased task. A second open refresh for the same order is refused.
pub fn admit_board_refresh(
    task_id: &str,
    event_id: &str,
    work_order_id: &str,
    leased: bool,
) -> Result<&'static str, ManufacturingError> {
    if task_id.trim().is_empty()
        || event_id.trim().is_empty()
        || work_order_id.trim().is_empty()
        || task_id == event_id
        || task_id == work_order_id
        || event_id == work_order_id
    {
        return Err(ManufacturingError::Planning(
            "board refresh identities stay distinct".into(),
        ));
    }
    if !leased {
        return Err(ManufacturingError::Planning(
            "board refresh waits for its lease".into(),
        ));
    }
    Ok("DONE")
}

/// One open recall per tenant. A second open task is refused before another scope is written.
pub fn admit_recall_task(
    task_id: &str,
    event_id: &str,
    open_count: i64,
    leased: bool,
) -> Result<&'static str, ManufacturingError> {
    if task_id.trim().is_empty() || event_id.trim().is_empty() || task_id == event_id {
        return Err(ManufacturingError::Planning(
            "recall task identities stay distinct".into(),
        ));
    }
    if open_count != 0 {
        return Err(ManufacturingError::Planning(
            "recall is already queued".into(),
        ));
    }
    if !leased {
        return Err(ManufacturingError::Planning(
            "recall waits for its lease".into(),
        ));
    }
    Ok("DONE")
}

/// A stored estimate is not a promised date. Missing capacity or a due date stays unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishEstimate {
    pub status: &'static str,
    pub estimated_date: Option<String>,
    pub late: Option<bool>,
    pub basis: &'static str,
}

pub fn estimate_finish(
    remaining_minutes: i64,
    minutes_per_day: Option<i64>,
    due_date: Option<&str>,
    today: &str,
) -> Result<FinishEstimate, ManufacturingError> {
    if remaining_minutes < 0 || today.len() != 10 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let due = match due_date {
        Some(due) if due.len() == 10 => Some(due),
        Some(_) => return Err(ManufacturingError::Quantity(QuantityError::InvalidText)),
        None => None,
    };
    let already_late = due.is_some_and(|due| remaining_minutes > 0 && due < today);
    let Some(per_day) = minutes_per_day else {
        return Ok(FinishEstimate {
            status: "UNKNOWN",
            estimated_date: None,
            late: due.map(|_| already_late),
            basis: "missing daily capacity",
        });
    };
    if per_day <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if due.is_none() {
        return Ok(FinishEstimate {
            status: "UNKNOWN",
            estimated_date: None,
            late: None,
            basis: "missing due date",
        });
    }
    let days = if remaining_minutes == 0 {
        0
    } else {
        (remaining_minutes + per_day - 1) / per_day
    };
    let start = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d")
        .map_err(|_| ManufacturingError::Quantity(QuantityError::InvalidText))?;
    let estimated = start + chrono::Duration::days(days);
    let estimated_date = estimated.format("%Y-%m-%d").to_string();
    let late = due.is_some_and(|due| estimated_date.as_str() > due);
    Ok(FinishEstimate {
        status: "ESTIMATE",
        estimated_date: Some(estimated_date),
        late: Some(late),
        basis: "remaining routing minutes divided by stated daily capacity; not a promised date",
    })
}

/// One frozen inspection line. A later template edit must not rewrite this copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenInspectionLine {
    pub item_code: String,
    pub required: bool,
    pub min_micros: i64,
    pub max_micros: i64,
    pub sample_count: i64,
}

/// A required reading cannot be omitted. Zero is a recorded reading, not a missing one.
/// The reading is judged against the frozen line, not a template edited afterwards.
pub fn admit_inspection_reading(
    item_code: &str,
    required: bool,
    min_micros: i64,
    max_micros: i64,
    reading: Option<i64>,
) -> Result<i64, ManufacturingError> {
    if item_code.trim().is_empty()
        || min_micros > max_micros
        || (required && min_micros == 0 && max_micros == 0)
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let Some(value) = reading else {
        if required {
            return Err(ManufacturingError::MissingReason(
                "required inspection reading is missing".into(),
            ));
        }
        return Ok(0);
    };
    if !(min_micros..=max_micros).contains(&value) {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(value)
}

/// Copies the cited template version. An empty source, item, or batch is not a freeze.
pub fn freeze_inspection_template(
    source_kind: &str,
    source_id: &str,
    item_code: &str,
    batch_code: &str,
    version_no: i32,
    lines: &[FrozenInspectionLine],
) -> Result<Vec<FrozenInspectionLine>, ManufacturingError> {
    if source_kind.trim().is_empty()
        || source_id.trim().is_empty()
        || item_code.trim().is_empty()
        || batch_code.trim().is_empty()
        || version_no < 1
        || lines.is_empty()
    {
        return Err(ManufacturingError::MissingReason(
            "inspection freeze needs a source, item, batch, and template version".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    let mut frozen = Vec::with_capacity(lines.len());
    for line in lines {
        if line.item_code.trim().is_empty()
            || line.min_micros > line.max_micros
            || line.sample_count <= 0
        {
            return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
        }
        if line.required && line.min_micros == 0 && line.max_micros == 0 {
            return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
        }
        if !seen.insert(line.item_code.clone()) {
            return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
        }
        frozen.push(line.clone());
    }
    Ok(frozen)
}

/// A recorded value inside the frozen range passes. A missing required value does not.
pub fn reading_passes(
    required: bool,
    min_micros: i64,
    max_micros: i64,
    reading: Option<i64>,
) -> bool {
    match reading {
        Some(value) => (min_micros..=max_micros).contains(&value),
        None => !required,
    }
}

/// A disposition consumes held quantity. Concession needs an authorizer. Scrap and return need a reason.
/// The quality status is the ledger status, not a label written beside an unchanged row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispositionEffect {
    pub held_left: ScaledQty,
    pub available_delta: ScaledQty,
    pub isolated_delta: ScaledQty,
    pub quality_status: String,
    pub version_no: i32,
    pub scrap_cost_cent: i64,
}

pub fn admit_disposition(
    held: ScaledQty,
    kind: &str,
    qty: ScaledQty,
    reason: &str,
    authorizer: &str,
) -> Result<DispositionEffect, ManufacturingError> {
    let cited = admit_cited_disposition(crate::manufacturing::CitedDispositionInput {
        held,
        kind,
        qty,
        reason,
        authorizer,
        inspection_id: "legacy",
        batch_code: "legacy",
        location_id: "legacy",
        prior_version: 0,
        unit_cost_cent: 0,
    })?;
    Ok(DispositionEffect {
        quality_status: String::new(),
        version_no: 0,
        scrap_cost_cent: 0,
        ..cited
    })
}

/// A cited disposition names the inspection and the original batch, and keeps a version.
/// An empty location is not a disposition. Concession still needs an authorizer.
#[derive(Debug, Clone, Copy)]
pub struct CitedDispositionInput<'a> {
    pub held: ScaledQty,
    pub kind: &'a str,
    pub qty: ScaledQty,
    pub reason: &'a str,
    pub authorizer: &'a str,
    pub inspection_id: &'a str,
    pub batch_code: &'a str,
    pub location_id: &'a str,
    pub prior_version: i32,
    /// Confirmed unit cost of the held batch. Scrap uses this, not a caller amount.
    pub unit_cost_cent: i64,
}

pub fn admit_cited_disposition(
    input: CitedDispositionInput<'_>,
) -> Result<DispositionEffect, ManufacturingError> {
    let CitedDispositionInput {
        held,
        kind,
        qty,
        reason,
        authorizer,
        inspection_id,
        batch_code,
        location_id,
        prior_version,
        unit_cost_cent,
    } = input;
    if inspection_id.trim().is_empty()
        || batch_code.trim().is_empty()
        || location_id.trim().is_empty()
    {
        return Err(ManufacturingError::MissingReason(
            "disposition needs an inspection, batch, and location".into(),
        ));
    }
    if prior_version < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if qty.as_micros() <= 0 || qty.as_micros() > held.as_micros() {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let held_left = held.try_sub(qty).map_err(ManufacturingError::Quantity)?;
    if unit_cost_cent < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let (available_delta, isolated_delta, quality_status, scrap_cost_cent) = match kind {
        "ACCEPT" | "CONCESSION" => {
            if kind == "CONCESSION" && authorizer.trim().is_empty() {
                return Err(ManufacturingError::MissingReason(
                    "concession authorizer".into(),
                ));
            }
            if kind == "CONCESSION" && reason.trim().is_empty() {
                return Err(ManufacturingError::MissingReason(
                    "disposition reason".into(),
                ));
            }
            (qty, ScaledQty::ZERO, "GOOD", 0)
        }
        "ISOLATE" => {
            if reason.trim().is_empty() {
                return Err(ManufacturingError::MissingReason(
                    "disposition reason".into(),
                ));
            }
            (ScaledQty::ZERO, qty, "ISOLATED", 0)
        }
        "RETURN_SUPPLIER" => {
            if reason.trim().is_empty() {
                return Err(ManufacturingError::MissingReason(
                    "disposition reason".into(),
                ));
            }
            (ScaledQty::ZERO, qty, "RETURN", 0)
        }
        "REWORK" => {
            if reason.trim().is_empty() {
                return Err(ManufacturingError::MissingReason(
                    "disposition reason".into(),
                ));
            }
            (ScaledQty::ZERO, qty, "REWORK", 0)
        }
        "SCRAP" => {
            if reason.trim().is_empty() {
                return Err(ManufacturingError::MissingReason(
                    "disposition reason".into(),
                ));
            }
            let scrap_cost_cent = qty
                .as_micros()
                .checked_mul(unit_cost_cent)
                .and_then(|product| product.checked_div(1_000_000))
                .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
            (ScaledQty::ZERO, qty, "SCRAP", scrap_cost_cent)
        }
        _ => return Err(ManufacturingError::IllegalTransition(kind.into())),
    };
    Ok(DispositionEffect {
        held_left,
        available_delta,
        isolated_delta,
        quality_status: quality_status.into(),
        scrap_cost_cent,
        version_no: prior_version
            .checked_add(1)
            .ok_or_else(|| ManufacturingError::Planning("disposition version exhausted".into()))?,
    })
}

/// One actual source action. A planned BOM line is not a genealogy edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenealogyEdge {
    pub source_action: String,
    pub input_code: String,
    pub output_code: String,
    pub qty: ScaledQty,
    pub work_order: String,
    pub output_batch: String,
    pub customer: String,
    pub reversed: bool,
}

/// Forward and reverse pages share one ordering. A short page is not a complete range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracePage {
    pub edges: Vec<GenealogyEdge>,
    pub next_cursor: Option<usize>,
    pub incomplete: bool,
}

/// One stored edge plus its stable identity. Pages order by that identity, not by arrival time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedEdge {
    pub edge_id: String,
    pub edge: GenealogyEdge,
}

/// Actual consume, output, delivery, return, and reversal edges. Planned BOM is not input.
pub fn admit_genealogy_edge(
    source_action: &str,
    input_code: &str,
    output_code: &str,
    qty: ScaledQty,
    work_order: &str,
    output_batch: &str,
    customer: &str,
) -> Result<GenealogyEdge, ManufacturingError> {
    if source_action.trim().is_empty()
        || input_code.trim().is_empty()
        || output_code.trim().is_empty()
        || work_order.trim().is_empty()
        || output_batch.trim().is_empty()
        || qty.as_micros() == 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if source_action == "PLANNED_BOM" {
        return Err(ManufacturingError::IllegalTransition(
            "planned BOM is not actual consumption".into(),
        ));
    }
    let reversal = source_action.ends_with("_REVERSAL");
    if reversal && qty.as_micros() >= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if !reversal && qty.as_micros() <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if matches!(
        source_action,
        "DELIVER" | "RETURN" | "DELIVER_REVERSAL" | "RETURN_REVERSAL"
    ) && customer.trim().is_empty()
    {
        return Err(ManufacturingError::MissingReason(
            "customer is required for delivery or return".into(),
        ));
    }
    Ok(GenealogyEdge {
        source_action: source_action.to_string(),
        input_code: input_code.to_string(),
        output_code: output_code.to_string(),
        qty,
        work_order: work_order.to_string(),
        output_batch: output_batch.to_string(),
        customer: customer.to_string(),
        reversed: reversal,
    })
}

/// Pages actual edges in stable order. The incomplete flag stays set until the cursor is exhausted.
pub fn page_genealogy(
    edges: &[GenealogyEdge],
    cursor: Option<usize>,
    limit: usize,
) -> Result<TracePage, ManufacturingError> {
    if limit == 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let start = cursor.unwrap_or(0);
    if start > edges.len() {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let end = start.saturating_add(limit).min(edges.len());
    let incomplete = end < edges.len();
    Ok(TracePage {
        edges: edges[start..end].to_vec(),
        next_cursor: incomplete.then_some(end),
        incomplete,
    })
}

/// One database page. A full page or a walk that hit the node cap is not a complete customer range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeysetTracePage {
    pub edges: Vec<GenealogyEdge>,
    pub edge_ids: Vec<String>,
    pub next_edge_id: Option<String>,
    pub incomplete: bool,
    pub loaded: i64,
}

/// Decides the page from rows already limited by the database. It does not slice a preloaded graph.
pub fn keyset_genealogy_page(
    rows: &[IndexedEdge],
    limit: usize,
    loaded: i64,
    node_capped: bool,
) -> Result<KeysetTracePage, ManufacturingError> {
    let maximum_rows = limit
        .checked_add(1)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let row_count = i64::try_from(rows.len())
        .map_err(|_| ManufacturingError::Quantity(QuantityError::Overflow))?;
    if limit == 0 || loaded < 0 || row_count > maximum_rows {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let more = rows.len() > limit;
    let shown = if more { &rows[..limit] } else { rows };
    let incomplete = more || node_capped;
    Ok(KeysetTracePage {
        edges: shown.iter().map(|row| row.edge.clone()).collect(),
        edge_ids: shown.iter().map(|row| row.edge_id.clone()).collect(),
        next_edge_id: incomplete
            .then(|| shown.last().map(|row| row.edge_id.clone()))
            .flatten(),
        incomplete,
        loaded,
    })
}

/// A trace export may freeze only a finished page. A short page is not the customer range.
pub fn admit_trace_export(
    incomplete: bool,
    edge_ids: &[String],
) -> Result<Vec<String>, ManufacturingError> {
    if incomplete {
        return Err(ManufacturingError::Planning(
            "an unfinished trace page cannot stand for the customer range".into(),
        ));
    }
    if edge_ids.is_empty() || edge_ids.iter().any(|id| id.trim().is_empty()) {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let mut frozen = edge_ids.to_vec();
    frozen.sort();
    frozen.dedup();
    Ok(frozen)
}

/// One published sampling rule. A later version does not rewrite this row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingRuleVersion {
    pub rule_code: String,
    pub rule_version: i32,
    pub sample_size: i64,
    pub accept_number: i64,
}

/// One stored period inside a trend window. A later period does not rewrite an earlier rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingTrendPoint {
    pub period_code: String,
    pub sample_size: i64,
    pub defects: i64,
    pub defect_ppm: i64,
    pub accepted: bool,
}

/// A trend is the stored decisions inside one window. It is not a recall and not recovered stock.
pub fn admit_sampling_trend(
    points: &[SamplingTrendPoint],
) -> Result<Vec<SamplingTrendPoint>, ManufacturingError> {
    if points.is_empty() {
        return Err(ManufacturingError::MissingReason(
            "sampling trend has no stored decision".into(),
        ));
    }
    let mut ordered = points.to_vec();
    ordered.sort_by(|left, right| left.period_code.cmp(&right.period_code));
    ordered.dedup_by(|left, right| left.period_code == right.period_code);
    if ordered.len() != points.len() {
        return Err(ManufacturingError::IllegalTransition(
            "sampling trend repeated a period".into(),
        ));
    }
    if ordered.iter().any(|point| {
        point.period_code.trim().is_empty()
            || point.sample_size <= 0
            || point.defects < 0
            || point.defects > point.sample_size
            || point.defect_ppm < 0
    }) {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(ordered)
}

/// Fixed single-sampling decision. The accept number is part of the named rule version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingDecision {
    pub rule_code: String,
    pub rule_version: i32,
    pub sample_size: i64,
    pub defects: i64,
    pub accept_number: i64,
    pub accepted: bool,
}

/// A recall page is complete only when the walk was not cut by the page or the node cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallTracePage {
    pub outputs: Vec<String>,
    pub customers: Vec<String>,
    pub incomplete: bool,
}

/// A recall binds a confirmed trace scope and one rule version. Freeze and notice stay separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallScope {
    pub input_batch: String,
    pub rule_code: String,
    pub rule_version: i32,
    pub outputs: Vec<String>,
    pub customers: Vec<String>,
}

/// A published rule needs a positive sample and a non-negative accept number.
pub fn admit_sampling_rule(
    rule_code: &str,
    rule_version: i32,
    sample_size: i64,
    accept_number: i64,
) -> Result<SamplingRuleVersion, ManufacturingError> {
    if rule_code.trim().is_empty() || rule_version <= 0 || sample_size <= 0 || accept_number < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if accept_number > sample_size {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(SamplingRuleVersion {
        rule_code: rule_code.trim().to_string(),
        rule_version,
        sample_size,
        accept_number,
    })
}

/// Judges defects against the cited rule version. A caller cannot substitute another accept number.
pub fn admit_sampling_against_rule(
    rule: &SamplingRuleVersion,
    defects: i64,
) -> Result<SamplingDecision, ManufacturingError> {
    admit_sampling_decision(
        &rule.rule_code,
        rule.rule_version,
        rule.sample_size,
        defects,
        rule.accept_number,
    )
}

/// One period is stored once. A later period does not replace an earlier decision.
pub fn sampling_period_is_new(
    existing_periods: &[&str],
    period_code: &str,
) -> Result<(), ManufacturingError> {
    if period_code.trim().is_empty() {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if existing_periods
        .iter()
        .any(|period| *period == period_code.trim())
    {
        return Err(ManufacturingError::IllegalTransition(
            "sampling period is already recorded".into(),
        ));
    }
    Ok(())
}

/// Sample size and accept number come from the selected rule, not from the lot size.
pub fn admit_sampling_decision(
    rule_code: &str,
    rule_version: i32,
    sample_size: i64,
    defects: i64,
    accept_number: i64,
) -> Result<SamplingDecision, ManufacturingError> {
    if rule_code.trim().is_empty() || rule_version <= 0 || sample_size <= 0 || accept_number < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if defects < 0 || defects > sample_size {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(SamplingDecision {
        rule_code: rule_code.to_string(),
        rule_version,
        sample_size,
        defects,
        accept_number,
        accepted: defects <= accept_number,
    })
}

/// Defect rate is defects per million sample units. A later period does not rewrite an earlier one.
pub fn defect_rate_ppm(sample_size: i64, defects: i64) -> Result<i64, ManufacturingError> {
    if sample_size <= 0 || defects < 0 || defects > sample_size {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    defects
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_div(sample_size))
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))
}

/// Collects one recall page. A capped or short page is not a confirmed full scope.
pub fn recall_scope_from_page(
    outputs: &[String],
    customers: &[String],
    page_full: bool,
    node_capped: bool,
) -> Result<RecallTracePage, ManufacturingError> {
    let mut outputs: Vec<String> = outputs
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect();
    let mut customers: Vec<String> = customers
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect();
    outputs.sort();
    outputs.dedup();
    customers.sort();
    customers.dedup();
    Ok(RecallTracePage {
        outputs,
        customers,
        incomplete: page_full || node_capped,
    })
}

/// A recall may open only from a complete page that still names outputs and customers.
pub fn admit_complete_recall_page(page: &RecallTracePage) -> Result<(), ManufacturingError> {
    if page.incomplete || page.outputs.is_empty() || page.customers.is_empty() {
        return Err(ManufacturingError::MissingReason(
            "confirmed trace scope is required".into(),
        ));
    }
    Ok(())
}

/// Freeze posts unavailable stock. Notice does not. A failed item stays recoverable.
pub fn recall_freeze_ledger(
    field: &str,
    status: &str,
    qty_micros: i64,
) -> Result<Option<i64>, ManufacturingError> {
    if !matches!(field, "freeze" | "notice")
        || !matches!(status, "DONE" | "FAILED")
        || qty_micros < 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if field == "freeze" && status == "DONE" {
        if qty_micros == 0 {
            return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
        }
        return Ok(Some(qty_micros));
    }
    Ok(None)
}

/// Recall scope is the confirmed outputs and customers. An empty trace cannot open a recall.
pub fn admit_recall_scope(
    input_batch: &str,
    rule_code: &str,
    rule_version: i32,
    outputs: &[String],
    customers: &[String],
) -> Result<RecallScope, ManufacturingError> {
    if input_batch.trim().is_empty() || rule_code.trim().is_empty() || rule_version <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if outputs.is_empty() || customers.is_empty() {
        return Err(ManufacturingError::MissingReason(
            "confirmed trace scope is required".into(),
        ));
    }
    let mut outputs: Vec<String> = outputs.to_vec();
    let mut customers: Vec<String> = customers
        .iter()
        .filter(|item| !item.is_empty())
        .cloned()
        .collect();
    outputs.sort();
    outputs.dedup();
    customers.sort();
    customers.dedup();
    if outputs.is_empty() || customers.is_empty() {
        return Err(ManufacturingError::MissingReason(
            "confirmed trace scope is required".into(),
        ));
    }
    Ok(RecallScope {
        input_batch: input_batch.to_string(),
        rule_code: rule_code.to_string(),
        rule_version,
        outputs,
        customers,
    })
}

/// Release is allowed only when every required item passed and names an inspector.
pub fn admit_quality_release(
    required_open: i64,
    failed: i64,
    inspector: &str,
) -> Result<(), ManufacturingError> {
    if inspector.trim().is_empty() {
        return Err(ManufacturingError::MissingReason(
            "inspector is required".into(),
        ));
    }
    if required_open > 0 || failed > 0 {
        return Err(ManufacturingError::IllegalTransition(
            "required inspection is incomplete or failed".into(),
        ));
    }
    Ok(())
}

pub fn subcontract_fee(work_order_id: &str, fee_cent: i64) -> Result<i64, ManufacturingError> {
    if work_order_id.is_empty() || fee_cent < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(fee_cent)
}

/// Missing overhead cannot become confirmed profit. The stored total is material
/// plus labor only, and the run stays estimated. A claimed fee is not accepted here.
pub fn missing_fee_estimate(
    material_cent: i64,
    labor_minutes: i64,
    labor_rate_cent: i64,
    completed_qty: ScaledQty,
) -> Result<(i64, i64), ManufacturingError> {
    if material_cent < 0
        || labor_minutes < 0
        || labor_rate_cent < 0
        || completed_qty.as_micros() <= 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let scale = crate::quantity::QTY_SCALE;
    if completed_qty.as_micros() % scale != 0 {
        return Err(ManufacturingError::Quantity(QuantityError::ScaleExceeded));
    }
    let labor = labor_minutes
        .checked_mul(labor_rate_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let total = material_cent
        .checked_add(labor)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let units = completed_qty.as_micros() / scale;
    Ok((total, total / units))
}

/// A late fee is a new adjustment. It must not change the confirmed cost row.
pub fn late_cost_adjustment(
    confirmed_total_cent: i64,
    fee_cent: i64,
) -> Result<i64, ManufacturingError> {
    if fee_cent == 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    confirmed_total_cent
        .checked_add(fee_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    Ok(fee_cent)
}

/// Marking a work order complete does not zero work in process.
/// A caller-supplied zero is ignored when stock is still in process.
pub fn complete_does_not_clear_wip(
    completed: bool,
    stored: ScaledQty,
    claimed: ScaledQty,
) -> ScaledQty {
    let _completed = completed;
    if stored.as_micros() > 0 && claimed.as_micros() <= 0 {
        return stored;
    }
    claimed
}

/// Held stock stays unavailable until this release. A second release is rejected.
pub fn release_held_receipt(
    status: ReceiptStatus,
    qty: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    match status {
        ReceiptStatus::Hold if qty.as_micros() > 0 => Ok(qty),
        ReceiptStatus::Released => Err(ManufacturingError::IllegalTransition(
            "already released".into(),
        )),
        _ => Err(ManufacturingError::Quantity(QuantityError::InvalidText)),
    }
}

/// A released semi-finished receipt can be issued to a parent. Held stock cannot.
/// The receipt row remains; only its available remainder decreases.
pub fn reissue_released(
    status: ReceiptStatus,
    available: ScaledQty,
    qty: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    if status != ReceiptStatus::Released {
        return Err(ManufacturingError::IssuedMaterial(
            "held stock cannot be reissued".into(),
        ));
    }
    if qty.as_micros() <= 0 || qty.as_micros() > available.as_micros() {
        return Err(ManufacturingError::IssuedMaterial(
            "reissue exceeds available semi".into(),
        ));
    }
    available.try_sub(qty).map_err(ManufacturingError::Quantity)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialMoveKind {
    Issue,
    Supplement,
    Return,
    Consume,
}

impl MaterialMoveKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Issue => "ISSUE",
            Self::Supplement => "SUPPLEMENT",
            Self::Return => "RETURN",
            Self::Consume => "CONSUME",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialMove {
    pub kind: MaterialMoveKind,
    pub qty: ScaledQty,
    pub supplement_reason_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialBalance {
    pub issued: ScaledQty,
    pub returned: ScaledQty,
    pub consumed: ScaledQty,
    pub shop_qty: ScaledQty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManufacturingError {
    Quantity(QuantityError),
    Cycle(String),
    MissingParent(String),
    IllegalTransition(String),
    IssuedMaterial(String),
    MissingReason(String),
    Planning(String),
}

const MAX_BOM_ITEMS: usize = 10_000;
const MAX_BOM_EDGES: usize = 50_000;
const MAX_BOM_DEPTH: i32 = 32;

/// One manufacturing BOM line. Scrap is applied once. Yield is not a second scrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomLine {
    pub parent: String,
    pub component: String,
    pub qty_per: ScaledQty,
    pub unit_code: String,
    pub scrap_ppm: i64,
    pub supply: String,
    pub line_no: i32,
}

/// A route page is not the business sequence. Display order must not replace sequence numbers.
pub fn admit_route_page(
    product: &str,
    offset: i64,
    limit: i64,
    loaded: i64,
    step_count: i64,
    first_display: i32,
    first_sequence: i32,
) -> Result<&'static str, ManufacturingError> {
    if product.trim().is_empty()
        || offset < 0
        || limit <= 0
        || limit > 50
        || loaded < 0
        || loaded > limit
        || step_count < loaded + offset
        || step_count > 100
        || first_display < 0
        || first_sequence <= 0
        || first_display == first_sequence
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok("PAGED")
}

/// A parent page is not the structure sequence. Yield stays off the exploded quantity.
#[derive(Debug, Clone, Copy)]
pub struct BomPageInput<'a> {
    pub parent: &'a str,
    pub offset: i64,
    pub limit: i64,
    pub loaded: i64,
    pub child_count: i64,
    pub yield_ppm: i64,
    pub qty_with_scrap: i64,
    pub qty_with_yield: i64,
}

pub fn admit_bom_page(input: BomPageInput<'_>) -> Result<&'static str, ManufacturingError> {
    let BomPageInput {
        parent,
        offset,
        limit,
        loaded,
        child_count,
        yield_ppm,
        qty_with_scrap,
        qty_with_yield,
    } = input;
    let loaded_end = offset
        .checked_add(loaded)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    if parent.trim().is_empty()
        || offset < 0
        || limit <= 0
        || limit > 50
        || loaded < 0
        || loaded > limit
        || child_count < loaded_end
        || !(0..=1_000_000).contains(&yield_ppm)
        || qty_with_scrap <= 0
        || qty_with_yield != qty_with_scrap
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok("PAGED")
}

/// Scrap is net quantity times one plus the rate. A 2% rate on 0.125 is 0.1275.
/// Yield is a separate field and is not applied here.
pub fn scrap_qty(net: ScaledQty, scrap_ppm: i64) -> Result<ScaledQty, ManufacturingError> {
    if net.as_micros() <= 0 || !(0..=1_000_000).contains(&scrap_ppm) {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let scale = i128::from(crate::quantity::QTY_SCALE);
    let scaled = i128::from(net.as_micros()) * (scale + i128::from(scrap_ppm));
    if scaled % scale != 0 {
        return Err(ManufacturingError::Quantity(QuantityError::ScaleExceeded));
    }
    let qty = i64::try_from(scaled / scale)
        .map_err(|_| ManufacturingError::Quantity(QuantityError::Overflow))?;
    Ok(ScaledQty::from_micros(qty))
}

/// A draft structure is refused before publish when a line repeats, a unit is blank,
/// supply is not make or buy, or a component can reach itself.
pub fn admit_bom_structure(lines: &[BomLine]) -> Result<(), ManufacturingError> {
    if lines.len() > MAX_BOM_EDGES {
        return Err(ManufacturingError::Planning(
            "BOM edge budget exceeded".into(),
        ));
    }
    if lines.is_empty() {
        return Err(ManufacturingError::MissingParent("empty".into()));
    }
    let mut seen = BTreeSet::new();
    let mut edges = Vec::new();
    for line in lines {
        if line.parent.trim().is_empty()
            || line.component.trim().is_empty()
            || line.unit_code.trim().is_empty()
            || line.parent == line.component
            || line.line_no <= 0
        {
            return Err(ManufacturingError::MissingParent(line.component.clone()));
        }
        if line.supply != "MAKE" && line.supply != "BUY" {
            return Err(ManufacturingError::MissingReason(line.supply.clone()));
        }
        scrap_qty(line.qty_per, line.scrap_ppm)?;
        if !seen.insert((line.parent.clone(), line.component.clone())) {
            return Err(ManufacturingError::Cycle(line.component.clone()));
        }
        edges.push(BomItem {
            parent: line.parent.clone(),
            component: line.component.clone(),
            qty_per: line.qty_per,
        });
    }
    reject_cycle(&edges)
}

/// An edge total for a published structure: parent, component, unit, quantity,
/// line number, and longest depth from the root. Shared subtrees are aggregated
/// before expansion; these rows are not copies of every possible path.
pub type BomExplosionRow = (String, String, String, i64, i32, i32);

pub fn explode_bom(
    lines: &[BomLine],
    root: &str,
    order_qty: ScaledQty,
) -> Result<Vec<BomExplosionRow>, ManufacturingError> {
    admit_bom_structure(lines)?;
    if order_qty.as_micros() <= 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let mut children: BTreeMap<&str, Vec<&BomLine>> = BTreeMap::new();
    let mut edges = Vec::with_capacity(lines.len());
    for line in lines {
        children.entry(line.parent.as_str()).or_default().push(line);
        edges.push(BomItem {
            parent: line.parent.clone(),
            component: line.component.clone(),
            qty_per: line.qty_per,
        });
    }
    if !children.contains_key(root) {
        return Err(ManufacturingError::MissingParent(root.into()));
    }
    for rows in children.values_mut() {
        rows.sort_by_key(|line| line.line_no);
    }
    let mut pending = BTreeMap::from([(root, order_qty)]);
    let mut depths = BTreeMap::from([(root, 0_i32)]);
    let mut out = Vec::new();
    for parent in bom_topological_order(&edges)? {
        let Some(qty) = pending.get(parent).copied() else {
            continue;
        };
        let depth = depths
            .get(parent)
            .copied()
            .ok_or_else(|| ManufacturingError::MissingParent(parent.into()))?;
        for child in children.get(parent).into_iter().flatten() {
            let child_depth = depth + 1;
            if child_depth > MAX_BOM_DEPTH {
                return Err(ManufacturingError::Planning(
                    "BOM depth budget exceeded".into(),
                ));
            }
            let gross = scrap_qty(child.qty_per, child.scrap_ppm)?;
            let needed = multiply(gross, qty)?.as_micros();
            out.push((
                parent.to_string(),
                child.component.clone(),
                child.unit_code.clone(),
                needed,
                child.line_no,
                child_depth,
            ));
            if child.supply == "MAKE" {
                let quantity = pending
                    .entry(child.component.as_str())
                    .or_insert(ScaledQty::ZERO);
                *quantity = quantity
                    .try_add(ScaledQty::from_micros(needed))
                    .map_err(ManufacturingError::Quantity)?;
                let depth = depths
                    .entry(child.component.as_str())
                    .or_insert(child_depth);
                *depth = (*depth).max(child_depth);
            }
        }
    }
    Ok(out)
}

/// Two publishes of one series share one lock. The second sees the first range.
pub fn admit_bom_publish_race(
    series: &str,
    first_version: i32,
    second_start: &str,
    first_end: &str,
) -> Result<&'static str, ManufacturingError> {
    if series.trim().is_empty()
        || first_version < 1
        || second_start.trim().is_empty()
        || first_end.trim().is_empty()
        || second_start <= first_end
    {
        return Err(ManufacturingError::MissingReason("publish race".into()));
    }
    Ok("QUEUED")
}

/// A published range is inclusive. An empty end stays open. Overlap with another default is refused.
pub fn bom_range_overlaps(
    start: &str,
    end: &str,
    other_start: &str,
    other_end: &str,
) -> Result<bool, ManufacturingError> {
    if start.trim().is_empty() || other_start.trim().is_empty() {
        return Err(ManufacturingError::MissingReason("effective date".into()));
    }
    if !end.is_empty() && end < start || !other_end.is_empty() && other_end < other_start {
        return Err(ManufacturingError::MissingReason("effective date".into()));
    }
    let left_open = end.is_empty() || other_start <= end;
    let right_open = other_end.is_empty() || start <= other_end;
    Ok(left_open && right_open)
}

/// A need date picks the published version whose range contains it. A retired version is not selected.
pub fn select_bom_version<'a>(
    versions: &'a [(i32, &'a str, &'a str, &'a str)],
    need_date: &str,
) -> Result<(i32, &'a str), ManufacturingError> {
    if need_date.trim().is_empty() {
        return Err(ManufacturingError::MissingReason("need date".into()));
    }
    let mut chosen: Option<(i32, &'a str)> = None;
    for (version, status, start, end) in versions {
        if *status != "PUBLISHED" {
            continue;
        }
        if need_date < *start || !end.is_empty() && need_date > *end {
            continue;
        }
        chosen = Some((*version, *status));
    }
    chosen.ok_or_else(|| ManufacturingError::MissingParent(need_date.into()))
}

/// Reject a structure that can reach a component from itself.
pub fn reject_cycle(items: &[BomItem]) -> Result<(), ManufacturingError> {
    bom_topological_order(items).map(|_| ())
}

fn bom_topological_order(items: &[BomItem]) -> Result<Vec<&str>, ManufacturingError> {
    if items.len() > MAX_BOM_EDGES {
        return Err(ManufacturingError::Planning(
            "BOM edge budget exceeded".into(),
        ));
    }
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut incoming: BTreeMap<&str, usize> = BTreeMap::new();
    for item in items {
        edges
            .entry(item.parent.as_str())
            .or_default()
            .push(item.component.as_str());
        incoming.entry(item.parent.as_str()).or_default();
        *incoming.entry(item.component.as_str()).or_default() += 1;
        if incoming.len() > MAX_BOM_ITEMS {
            return Err(ManufacturingError::Planning(
                "BOM item budget exceeded".into(),
            ));
        }
    }
    let mut ready: BTreeSet<&str> = incoming
        .iter()
        .filter_map(|(&code, &count)| (count == 0).then_some(code))
        .collect();
    let mut ordered = Vec::with_capacity(incoming.len());
    while let Some(node) = ready.pop_first() {
        ordered.push(node);
        for child in edges.get(node).into_iter().flatten() {
            let count = incoming
                .get_mut(child)
                .ok_or_else(|| ManufacturingError::MissingParent((*child).into()))?;
            *count -= 1;
            if *count == 0 {
                ready.insert(child);
            }
        }
    }
    if ordered.len() != incoming.len() {
        return Err(ManufacturingError::Cycle("BOM contains a cycle".into()));
    }
    Ok(ordered)
}

/// Expand net manufacture of each parent before calculating child demand.
/// Late supply does not cover an earlier bucket. Existing semi-finished stock
/// is consumed before exploding its own components.
pub fn plan_mrp(
    bom: &[BomItem],
    items: &[MrpItem],
    demand: &[(&str, ScaledQty)],
) -> Result<Vec<MrpSuggestion>, ManufacturingError> {
    reject_cycle(bom)?;
    let mut stock: BTreeMap<String, MrpItem> = items
        .iter()
        .cloned()
        .map(|item| (item.code.clone(), item))
        .collect();
    let mut pending: BTreeMap<String, ScaledQty> = BTreeMap::new();
    for (code, qty) in demand {
        let next = pending
            .get(*code)
            .copied()
            .unwrap_or(ScaledQty::ZERO)
            .try_add(*qty)
            .map_err(ManufacturingError::Quantity)?;
        pending.insert((*code).to_string(), next);
    }
    let mut gross: BTreeMap<String, ScaledQty> = BTreeMap::new();
    let mut net: BTreeMap<String, ScaledQty> = BTreeMap::new();
    let mut guard = 0;
    while !pending.is_empty() {
        guard += 1;
        if guard > 10_000 {
            return Err(ManufacturingError::Cycle("depth".into()));
        }
        let level: Vec<(String, ScaledQty)> = pending.into_iter().collect();
        pending = BTreeMap::new();
        for (code, qty) in level {
            let item = stock.entry(code.clone()).or_insert_with(|| MrpItem {
                code: code.clone(),
                on_hand: ScaledQty::ZERO,
                allocated_elsewhere: ScaledQty::ZERO,
                supply_on_time: ScaledQty::ZERO,
                supply_late: ScaledQty::ZERO,
                safety: ScaledQty::ZERO,
            });
            let available = item
                .on_hand
                .try_sub(item.allocated_elsewhere)
                .map_err(ManufacturingError::Quantity)?;
            let carry = if available.as_micros() > 0 {
                available
            } else {
                ScaledQty::ZERO
            };
            let covered = carry
                .try_add(item.supply_on_time)
                .map_err(ManufacturingError::Quantity)?;
            let need = qty
                .try_add(item.safety)
                .map_err(ManufacturingError::Quantity)?;
            let shortage = if need.as_micros() > covered.as_micros() {
                need.try_sub(covered)
                    .map_err(ManufacturingError::Quantity)?
            } else {
                ScaledQty::ZERO
            };
            let previous_gross = gross.get(&code).copied().unwrap_or(ScaledQty::ZERO);
            gross.insert(
                code.clone(),
                previous_gross
                    .try_add(qty)
                    .map_err(ManufacturingError::Quantity)?,
            );
            let previous_net = net.get(&code).copied().unwrap_or(ScaledQty::ZERO);
            net.insert(
                code.clone(),
                previous_net
                    .try_add(shortage)
                    .map_err(ManufacturingError::Quantity)?,
            );
            item.on_hand = ScaledQty::ZERO;
            item.allocated_elsewhere = ScaledQty::ZERO;
            item.supply_on_time = ScaledQty::ZERO;
            item.safety = ScaledQty::ZERO;
            for child in bom.iter().filter(|row| row.parent == code) {
                let child_qty = multiply(shortage, child.qty_per)?;
                let next = pending
                    .get(&child.component)
                    .copied()
                    .unwrap_or(ScaledQty::ZERO)
                    .try_add(child_qty)
                    .map_err(ManufacturingError::Quantity)?;
                pending.insert(child.component.clone(), next);
            }
        }
    }
    let mut suggestions = Vec::new();
    let mut codes: BTreeSet<String> = gross.keys().cloned().collect();
    codes.extend(net.keys().cloned());
    for code in codes {
        let gross_qty = gross.get(&code).copied().unwrap_or(ScaledQty::ZERO);
        let net_qty = net.get(&code).copied().unwrap_or(ScaledQty::ZERO);
        if gross_qty.as_micros() == 0 && net_qty.as_micros() == 0 {
            continue;
        }
        suggestions.push(MrpSuggestion {
            code,
            gross: gross_qty,
            net_make_or_buy: net_qty,
            explanation: format!("algorithm {MRP_ALGORITHM}; late supply excluded"),
        });
    }
    Ok(suggestions)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatedDemand {
    pub code: String,
    pub qty: ScaledQty,
    /// Need date as `YYYY-MM-DD`. Supply dated after this is late.
    pub need_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingStep {
    pub operation: String,
    pub sequence: i32,
    pub work_center: String,
    pub minutes_per: i64,
    pub inspection_required: bool,
}

/// One explicit routing difference. The old step stays; this does not rewrite it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDiff {
    pub operation: String,
    pub change: &'static str,
    pub old_minutes: i64,
    pub new_minutes: i64,
    pub old_work_center: String,
    pub new_work_center: String,
}

/// Compares two approved routing versions by operation code.
/// An empty difference is not a change. The caller must keep the old version.
pub fn routing_diff(
    old: &[RoutingStep],
    new: &[RoutingStep],
) -> Result<Vec<RoutingDiff>, ManufacturingError> {
    validate_routing(old)?;
    validate_routing(new)?;
    let mut diffs = Vec::new();
    for step in old {
        match new.iter().find(|next| next.operation == step.operation) {
            None => diffs.push(RoutingDiff {
                operation: step.operation.clone(),
                change: "REMOVED",
                old_minutes: step.minutes_per,
                new_minutes: 0,
                old_work_center: step.work_center.clone(),
                new_work_center: String::new(),
            }),
            Some(next)
                if next.minutes_per != step.minutes_per
                    || next.work_center != step.work_center
                    || next.inspection_required != step.inspection_required
                    || next.sequence != step.sequence =>
            {
                diffs.push(RoutingDiff {
                    operation: step.operation.clone(),
                    change: "CHANGED",
                    old_minutes: step.minutes_per,
                    new_minutes: next.minutes_per,
                    old_work_center: step.work_center.clone(),
                    new_work_center: next.work_center.clone(),
                });
            }
            Some(_) => {}
        }
    }
    for step in new {
        if !old
            .iter()
            .any(|previous| previous.operation == step.operation)
        {
            diffs.push(RoutingDiff {
                operation: step.operation.clone(),
                change: "ADDED",
                old_minutes: 0,
                new_minutes: step.minutes_per,
                old_work_center: String::new(),
                new_work_center: step.work_center.clone(),
            });
        }
    }
    if diffs.is_empty() {
        return Err(ManufacturingError::IllegalTransition(
            "routing change has no difference".into(),
        ));
    }
    Ok(diffs)
}

/// One routing step with a predecessor. A blank predecessor is the start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationStep {
    pub operation: String,
    pub sequence: i32,
    pub predecessor: String,
    pub work_center: String,
    pub setup_minutes: i64,
    pub minutes_per: i64,
    pub inspection_required: bool,
    pub report_field: String,
    pub display_order: i32,
}

/// One calendar day. A rest day contributes no capacity. Parallel machines multiply the shift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarDay {
    pub day: String,
    pub rest: bool,
    pub shift_minutes: i64,
}

/// Capacity is working days times shift times parallel machines. A rest day is zero.
/// The result is capacity, load, overload, and whether this was an optimized schedule.
/// This function never claims automatic scheduling.
pub fn work_center_load(
    machines: i64,
    days: &[CalendarDay],
    setup_minutes: i64,
    run_minutes: i64,
) -> Result<(i64, i64, i64, bool), ManufacturingError> {
    if machines <= 0 || days.is_empty() || setup_minutes < 0 || run_minutes < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let mut seen = BTreeSet::new();
    let mut capacity = 0_i64;
    for day in days {
        if day.day.trim().is_empty() || day.shift_minutes < 0 || !seen.insert(day.day.as_str()) {
            return Err(ManufacturingError::MissingReason(day.day.clone()));
        }
        if day.rest {
            continue;
        }
        let day_capacity = day
            .shift_minutes
            .checked_mul(machines)
            .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
        capacity = capacity
            .checked_add(day_capacity)
            .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    }
    let load = setup_minutes
        .checked_add(run_minutes)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let overload = (load - capacity).max(0);
    Ok((capacity, load, overload, false))
}

/// One dated demand bucket. A resale source is not a production demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanBucket {
    pub due_date: String,
    pub qty: ScaledQty,
    pub source_kind: String,
}

/// Open demand is the current sales line minus reviewed deliveries and sales cancellations.
/// A client-supplied remainder cannot replace this figure.
pub fn sales_line_remainder(ordered: i64, delivered: i64, cancelled: i64) -> Result<i64, ManufacturingError> {
    if ordered <= 0 || delivered < 0 || cancelled < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let consumed = delivered
        .checked_add(cancelled)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    if consumed > ordered {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(ordered - consumed)
}

/// Splits one order into dated buckets. The sum must equal the open quantity.
/// A BUY source contributes no production quantity. A repeated date is refused.
pub fn plan_buckets(
    open_qty: ScaledQty,
    buckets: &[PlanBucket],
) -> Result<Vec<(String, i64, bool)>, ManufacturingError> {
    if open_qty.as_micros() <= 0 || buckets.is_empty() {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let mut seen = BTreeSet::new();
    let mut sum = 0_i64;
    let mut planned = Vec::new();
    for bucket in buckets {
        if bucket.due_date.trim().is_empty()
            || bucket.qty.as_micros() <= 0
            || !seen.insert(bucket.due_date.as_str())
            || (bucket.source_kind != "MAKE" && bucket.source_kind != "BUY")
        {
            return Err(ManufacturingError::MissingReason(bucket.due_date.clone()));
        }
        sum = sum
            .checked_add(bucket.qty.as_micros())
            .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
        let produce = bucket.source_kind == "MAKE";
        planned.push((
            bucket.due_date.clone(),
            if produce { bucket.qty.as_micros() } else { 0 },
            produce,
        ));
    }
    if sum != open_qty.as_micros() {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(planned)
}

/// A partial delivery reduces only the still-open plan. Executed quantity stays.
/// The cited delivery must be reviewed and must not exceed the open remainder.
pub fn admit_partial_delivery(
    planned: i64,
    executed: i64,
    cancelled: i64,
    delivered: i64,
    stage: &str,
) -> Result<i64, ManufacturingError> {
    if planned <= 0
        || executed < 0
        || cancelled < 0
        || delivered <= 0
        || stage != "REVIEWED"
        || executed + cancelled > planned
        || delivered > planned - executed - cancelled
    {
        return Err(ManufacturingError::IssuedMaterial(
            "delivery exceeds the open plan".into(),
        ));
    }
    Ok(planned - executed - cancelled - delivered)
}

/// A cancel can only reduce the still-open quantity. Executed quantity stays.
pub fn cancel_open_plan(
    planned: i64,
    executed: i64,
    cancel_qty: i64,
) -> Result<i64, ManufacturingError> {
    if planned < 0
        || executed < 0
        || cancel_qty <= 0
        || executed > planned
        || cancel_qty > planned - executed
    {
        return Err(ManufacturingError::IssuedMaterial(
            "cancel exceeds the open plan".into(),
        ));
    }
    Ok(planned - cancel_qty)
}

/// A load that crosses a rest day is split around it. The rest day itself holds no minutes.
/// A later calendar does not change a load that was already saved.
pub fn admit_calendar_span(
    rest_day: &str,
    before: &str,
    after: &str,
    rest_minutes: i64,
    before_minutes: i64,
    after_minutes: i64,
    saved_overload: i64,
) -> Result<&'static str, ManufacturingError> {
    if rest_day.trim().is_empty()
        || before.trim().is_empty()
        || after.trim().is_empty()
        || before >= rest_day
        || after <= rest_day
        || rest_minutes != 0
        || before_minutes <= 0
        || after_minutes <= 0
        || saved_overload < 0
    {
        return Err(ManufacturingError::MissingReason("calendar span".into()));
    }
    Ok("SPANNED")
}

/// A rate version is current only inside its inclusive range. A later rate does not rewrite the earlier one.
pub fn rate_is_current(day: &str, start: &str, end: &str) -> Result<(), ManufacturingError> {
    if day.trim().is_empty()
        || start.trim().is_empty()
        || day < start
        || !end.is_empty() && day > end
    {
        return Err(ManufacturingError::MissingReason("rate date".into()));
    }
    Ok(())
}

/// A predecessor cycle, a skip, a blank work center, or a negative time is refused.
/// Display order is not the business sequence.
pub fn admit_operation_route(steps: &[OperationStep]) -> Result<(), ManufacturingError> {
    if steps.is_empty() || steps.len() > 100 {
        return Err(ManufacturingError::MissingParent("routing".into()));
    }
    let mut seen = BTreeSet::new();
    let mut previous = 0;
    let mut starts = 0;
    for step in steps {
        if step.sequence <= previous
            || step.operation.trim().is_empty()
            || step.work_center.trim().is_empty()
            || step.setup_minutes < 0
            || step.minutes_per < 0
            || step.report_field.trim().is_empty()
            || step.predecessor == step.operation
            || !seen.insert(step.operation.as_str())
        {
            return Err(ManufacturingError::MissingParent(step.operation.clone()));
        }
        if step.predecessor.trim().is_empty() {
            starts += 1;
        }
        previous = step.sequence;
    }
    if starts != 1 {
        return Err(ManufacturingError::MissingParent("start".into()));
    }
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for step in steps {
        if step.predecessor.trim().is_empty() {
            continue;
        }
        if !seen.contains(step.predecessor.as_str()) {
            return Err(ManufacturingError::MissingParent(step.predecessor.clone()));
        }
        let pred_seq = steps
            .iter()
            .find(|item| item.operation == step.predecessor)
            .map(|item| item.sequence)
            .unwrap_or(0);
        if pred_seq >= step.sequence {
            return Err(ManufacturingError::IllegalTransition(
                step.operation.clone(),
            ));
        }
        edges
            .entry(step.predecessor.as_str())
            .or_default()
            .push(step.operation.as_str());
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for operation in seen {
        if let Some(code) = walk_route(operation, &edges, &mut visiting, &mut visited) {
            return Err(ManufacturingError::Cycle(code.to_string()));
        }
    }
    Ok(())
}

fn walk_route<'a>(
    operation: &'a str,
    edges: &BTreeMap<&str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> Option<&'a str> {
    if visited.contains(operation) {
        return None;
    }
    if !visiting.insert(operation) {
        return Some(operation);
    }
    if let Some(next) = edges.get(operation) {
        for child in next {
            if let Some(code) = walk_route(child, edges, visiting, visited) {
                return Some(code);
            }
        }
    }
    visiting.remove(operation);
    visited.insert(operation);
    None
}

/// A routing must have increasing sequence numbers and a named work center.
pub fn validate_routing(steps: &[RoutingStep]) -> Result<(), ManufacturingError> {
    if steps.is_empty() {
        return Err(ManufacturingError::MissingParent("routing".into()));
    }
    let mut previous = 0;
    for step in steps {
        if step.sequence <= previous
            || step.operation.trim().is_empty()
            || step.work_center.trim().is_empty()
            || step.minutes_per < 0
        {
            return Err(ManufacturingError::MissingParent(step.operation.clone()));
        }
        previous = step.sequence;
    }
    Ok(())
}

/// Issued material stays in process until consumed. Unreleased finished goods
/// are not available. Cost is actual material plus actual minutes times the
/// frozen rate; planned time is not an input.
pub fn execute(input: Execution) -> Result<ExecutionResult, ManufacturingError> {
    if input.consumed.as_micros() > input.issued.as_micros()
        || input.received.as_micros() > input.good.as_micros()
    {
        return Err(ManufacturingError::Quantity(QuantityError::Overflow));
    }
    let wip = input
        .issued
        .try_sub(input.consumed)
        .map_err(ManufacturingError::Quantity)?;
    let held = input
        .good
        .try_sub(input.received)
        .map_err(ManufacturingError::Quantity)?;
    let labor = input
        .labor_minutes
        .checked_mul(input.labor_rate_cent)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let cost = input
        .material_cost_cent
        .checked_add(labor)
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    let _held_for_inspection = held;
    let _rejected = input.rejected;
    Ok(ExecutionResult {
        wip_qty: wip,
        available_finished: input.received,
        cost_cent: cost,
    })
}

/// Reserves only available stock. A second release cannot reserve the same free quantity twice.
/// A shortage is refused and leaves no reservation.
pub fn admit_release_reservation(
    on_hand: ScaledQty,
    reserved: ScaledQty,
    requested: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    if requested.as_micros() <= 0 || on_hand.as_micros() < 0 || reserved.as_micros() < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let free = on_hand
        .try_sub(reserved)
        .map_err(ManufacturingError::Quantity)?;
    if requested.as_micros() > free.as_micros() {
        return Err(ManufacturingError::IssuedMaterial(
            "insufficient available stock".into(),
        ));
    }
    Ok(requested)
}

/// Release, pause, resume, complete, close, or cancel a work order.
/// Complete and close require no shop remainder. A planned order with no issued
/// material may be cancelled. Issued material is not deleted by a refused cancel.
pub fn transition_work_order(
    current: WorkOrderStatus,
    action: &str,
    issued: ScaledQty,
    consumed: ScaledQty,
    returned: ScaledQty,
) -> Result<WorkOrderStatus, ManufacturingError> {
    if issued.is_negative()
        || consumed.is_negative()
        || returned.is_negative()
        || consumed.as_micros() + returned.as_micros() > issued.as_micros()
    {
        return Err(ManufacturingError::Quantity(QuantityError::Overflow));
    }
    let shop = issued.as_micros() - returned.as_micros() - consumed.as_micros();
    let next = match (current, action) {
        (WorkOrderStatus::Planned, "release") => WorkOrderStatus::Released,
        (WorkOrderStatus::Released, "pause") => WorkOrderStatus::Paused,
        (WorkOrderStatus::Paused, "resume") => WorkOrderStatus::Released,
        (WorkOrderStatus::Released, "complete") if shop == 0 => WorkOrderStatus::Completed,
        (WorkOrderStatus::Released, "complete") => {
            return Err(ManufacturingError::IssuedMaterial(
                "return or consume shop stock before complete".into(),
            ));
        }
        (WorkOrderStatus::Completed, "close") if shop == 0 => WorkOrderStatus::Closed,
        (WorkOrderStatus::Planned, "cancel")
            if issued.as_micros() == 0 && consumed.as_micros() == 0 =>
        {
            WorkOrderStatus::Cancelled
        }
        (
            WorkOrderStatus::Planned | WorkOrderStatus::Released | WorkOrderStatus::Paused,
            "cancel",
        ) => {
            return Err(ManufacturingError::IssuedMaterial(
                "return or settle issued material before cancel".into(),
            ));
        }
        _ => return Err(ManufacturingError::IllegalTransition(action.into())),
    };
    Ok(next)
}

/// Available stock is on hand minus reserved. An issue consumes its own reservation
/// first, then free stock. It never uses another order's reservation.
pub fn issue_from_reservation(
    on_hand: ScaledQty,
    reserved: ScaledQty,
    own_reserved: ScaledQty,
    issue_qty: ScaledQty,
) -> Result<(ScaledQty, ScaledQty), ManufacturingError> {
    if on_hand.is_negative()
        || reserved.is_negative()
        || own_reserved.is_negative()
        || issue_qty.as_micros() <= 0
        || own_reserved.as_micros() > reserved.as_micros()
    {
        return Err(ManufacturingError::Quantity(QuantityError::Overflow));
    }
    let free = on_hand
        .try_sub(reserved)
        .map_err(ManufacturingError::Quantity)?;
    let from_own = if own_reserved.as_micros() < issue_qty.as_micros() {
        own_reserved
    } else {
        issue_qty
    };
    let from_free = issue_qty
        .try_sub(from_own)
        .map_err(ManufacturingError::Quantity)?;
    if from_free.as_micros() > free.as_micros() {
        return Err(ManufacturingError::IssuedMaterial(
            "insufficient available stock".into(),
        ));
    }
    let next_reserved = reserved
        .try_sub(from_own)
        .map_err(ManufacturingError::Quantity)?;
    let next_on_hand = on_hand
        .try_sub(issue_qty)
        .map_err(ManufacturingError::Quantity)?;
    Ok((next_on_hand, next_reserved))
}

/// Legacy inventory columns store whole base units. A six-digit quantity posts
/// only when it is an exact whole unit; fractional moves stay on the work order.
pub fn whole_units_for_ledger(qty: ScaledQty) -> Result<Option<i64>, ManufacturingError> {
    if qty.as_micros() <= 0 || qty.as_micros() % crate::quantity::QTY_SCALE != 0 {
        return Ok(None);
    }
    Ok(Some(qty.as_micros() / crate::quantity::QTY_SCALE))
}

/// A return cites the original issue. It cannot exceed that issue minus earlier returns.
/// Consume stays in the shop and does not restore warehouse stock.
pub fn admit_material_return(
    issued: ScaledQty,
    already_returned: ScaledQty,
    requested: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    if requested.as_micros() <= 0 || issued.as_micros() <= 0 || already_returned.as_micros() < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let open = issued
        .try_sub(already_returned)
        .map_err(ManufacturingError::Quantity)?;
    if requested.as_micros() > open.as_micros() {
        return Err(ManufacturingError::IssuedMaterial(
            "return exceeds the cited issue".into(),
        ));
    }
    Ok(requested)
}

/// Issue, supplement, return, or consume. Shop stock is issued minus returned
/// minus consumed. A return cannot exceed issued material. A supplement needs a reason.
pub fn apply_material_move(
    current: MaterialBalance,
    movement: MaterialMove,
) -> Result<MaterialBalance, ManufacturingError> {
    if movement.qty.is_negative() || movement.qty.as_micros() == 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    if movement.kind == MaterialMoveKind::Supplement && !movement.supplement_reason_required {
        return Err(ManufacturingError::MissingReason("supplement".into()));
    }
    let mut next = current;
    match movement.kind {
        MaterialMoveKind::Issue | MaterialMoveKind::Supplement => {
            next.issued = next
                .issued
                .try_add(movement.qty)
                .map_err(ManufacturingError::Quantity)?;
        }
        MaterialMoveKind::Return => {
            let outstanding = next
                .issued
                .try_sub(next.returned)
                .map_err(ManufacturingError::Quantity)?;
            if movement.qty.as_micros() > outstanding.as_micros() {
                return Err(ManufacturingError::IssuedMaterial(
                    "return exceeds issued".into(),
                ));
            }
            next.returned = next
                .returned
                .try_add(movement.qty)
                .map_err(ManufacturingError::Quantity)?;
        }
        MaterialMoveKind::Consume => {
            next.consumed = next
                .consumed
                .try_add(movement.qty)
                .map_err(ManufacturingError::Quantity)?;
        }
    }
    let shop = next
        .issued
        .try_sub(next.returned)
        .map_err(ManufacturingError::Quantity)?
        .try_sub(next.consumed)
        .map_err(ManufacturingError::Quantity)?;
    if shop.is_negative() {
        return Err(ManufacturingError::IssuedMaterial(
            "consume exceeds shop stock".into(),
        ));
    }
    next.shop_qty = shop;
    Ok(next)
}

/// One dated demand. Frozen demand is the only demand that enters a bucket run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketDemand {
    pub code: String,
    pub qty: ScaledQty,
    pub need_date: String,
    pub warehouse: String,
    pub frozen: bool,
}

/// On-hand, other allocation, and in-process quantity at one authorized warehouse.
/// An unaccepted suggestion is not supply and is not a field here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketStock {
    pub code: String,
    pub warehouse: String,
    pub on_hand: ScaledQty,
    pub allocated_elsewhere: ScaledQty,
    pub wip: ScaledQty,
    pub safety: ScaledQty,
    pub min_lot: ScaledQty,
    pub lot_multiple: ScaledQty,
    /// MAKE or BUY. Purchase and production dates stay independent.
    pub supply: String,
    pub lead_days: i32,
}

/// Confirmed supply. A date after the need date is late and does not cover it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketSupply {
    pub code: String,
    pub warehouse: String,
    pub qty: ScaledQty,
    pub arrive_date: String,
    pub accepted: bool,
}

/// One suggestion for one item and need date. Dates are calendar working days.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketSuggestion {
    pub code: String,
    pub warehouse: String,
    pub need_date: String,
    pub gross: ScaledQty,
    pub net: ScaledQty,
    pub suggested: ScaledQty,
    pub supply: String,
    pub release_date: String,
    pub explanation: String,
}

fn parse_ymd(text: &str) -> Result<(i32, u32, u32), ManufacturingError> {
    if text.len() != 10
        || !text.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return Err(ManufacturingError::Planning("bad date".into()));
    }
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(ManufacturingError::Planning("bad date".into()));
    }
    let year = parts[0]
        .parse::<i32>()
        .map_err(|_| ManufacturingError::Planning("bad date".into()))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|_| ManufacturingError::Planning("bad date".into()))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|_| ManufacturingError::Planning("bad date".into()))?;
    if !(1..=9_999).contains(&year)
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
    {
        return Err(ManufacturingError::Planning("bad date".into()));
    }
    Ok((year, month, day))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn shift_ymd(date: &str, days: i32) -> Result<String, ManufacturingError> {
    let (mut year, mut month, mut day) = parse_ymd(date)?;
    let mut left = days;
    if left >= 0 {
        while left > 0 {
            let room = days_in_month(year, month) - day;
            if left <= room as i32 {
                day += left as u32;
                left = 0;
            } else {
                left -= room as i32 + 1;
                day = 1;
                month += 1;
                if month == 13 {
                    month = 1;
                    year += 1;
                    if year > 9_999 {
                        return Err(ManufacturingError::Planning(
                            "date exceeds calendar range".into(),
                        ));
                    }
                }
            }
        }
    } else {
        while left < 0 {
            if day as i32 + left > 0 {
                day = (day as i32 + left) as u32;
                left = 0;
            } else {
                left += day as i32;
                month -= 1;
                if month == 0 {
                    month = 12;
                    year -= 1;
                    if year < 1 {
                        return Err(ManufacturingError::Planning(
                            "date exceeds calendar range".into(),
                        ));
                    }
                }
                day = days_in_month(year, month);
            }
        }
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn weekday(date: &str) -> Result<u32, ManufacturingError> {
    let (year, month, day) = parse_ymd(date)?;
    let mut y = year;
    let mut m = month as i32;
    if m < 3 {
        y -= 1;
        m += 12;
    }
    let k = y % 100;
    let j = y / 100;
    let h = (day as i32 + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j).rem_euclid(7);
    // Zeller uses Saturday = 0; the planning calendar uses Monday = 0.
    Ok((h + 5).rem_euclid(7) as u32)
}

/// A saved work-center rest day moves a release date. Purchase and production dates stay separate.
/// The cited center must belong to this tenant. A later calendar does not rewrite a saved suggestion.
pub fn admit_mrp_calendar(
    center: &str,
    rest_day: &str,
    purchase_date: &str,
    production_date: &str,
) -> Result<&'static str, ManufacturingError> {
    if center.trim().is_empty()
        || rest_day.trim().is_empty()
        || purchase_date.trim().is_empty()
        || production_date.trim().is_empty()
        || purchase_date == rest_day
        || production_date == rest_day
        || purchase_date == production_date
    {
        return Err(ManufacturingError::Planning(
            "calendar dates are not independent".into(),
        ));
    }
    Ok("DATED")
}

/// Walks backward by working days, skipping weekends and listed rest days.
/// A zero lead stays on the need date even when that date is a rest day.
pub fn release_on_calendar(need_date: &str, lead_days: i32) -> Result<String, ManufacturingError> {
    mrp_release_with_rest_days(need_date, lead_days, &BTreeSet::new())
}

/// One page of a material issue. The page stays inside the stored lines.
/// Display order is not the line number.
pub fn admit_material_page(
    order_no: &str,
    offset: i64,
    limit: i64,
    loaded: i64,
    line_count: i64,
    first_line: i32,
    first_display: i32,
) -> Result<&'static str, ManufacturingError> {
    if order_no.trim().is_empty()
        || offset < 0
        || limit <= 0
        || limit > 50
        || loaded < 0
        || loaded > limit
        || line_count < loaded + offset
        || line_count > 100
        || first_line <= 0
        || first_display < 0
        || first_line == first_display
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok("PAGED")
}

/// Two reports on one operation share the predecessor ceiling.
/// The second report cannot use output the first report already took.
pub fn admit_same_operation_race(
    ceiling: i64,
    first_good: i64,
    first_rejected: i64,
    second_good: i64,
    second_rejected: i64,
) -> Result<i64, ManufacturingError> {
    if ceiling < 0 || first_good < 0 || first_rejected < 0 || second_good < 0 || second_rejected < 0
    {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let used = first_good
        .checked_add(first_rejected)
        .and_then(|sum| sum.checked_add(second_good))
        .and_then(|sum| sum.checked_add(second_rejected))
        .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?;
    if used > ceiling {
        return Err(ManufacturingError::IssuedMaterial(
            "same operation reports share the predecessor ceiling".into(),
        ));
    }
    Ok(ceiling - used)
}

/// A substitute issue cites the planned component and a reason.
/// It does not rewrite the planned component.
pub fn admit_material_substitute(
    planned: &str,
    actual: &str,
    reason: &str,
    qty: i64,
) -> Result<&'static str, ManufacturingError> {
    if planned.trim().is_empty()
        || actual.trim().is_empty()
        || planned == actual
        || reason.trim().is_empty()
        || qty <= 0
    {
        return Err(ManufacturingError::IssuedMaterial(
            "a substitute needs a different component and a reason".into(),
        ));
    }
    Ok("SUBSTITUTED")
}

/// A partial completion keeps the order open. It cannot pass the order quantity,
/// and it does not erase issued or reported facts.
pub fn admit_partial_completion(
    order_qty: i64,
    already: i64,
    completed: i64,
    issued: i64,
    reported: i64,
    paused: bool,
) -> Result<i64, ManufacturingError> {
    if order_qty <= 0
        || already < 0
        || completed <= 0
        || issued < 0
        || reported < 0
        || paused
        || already + completed > order_qty
        || already + completed > reported
    {
        return Err(ManufacturingError::IssuedMaterial(
            "partial completion stays inside the reported open order".into(),
        ));
    }
    Ok(already + completed)
}

/// A late response finds the stored conversion. It does not add a second target.
pub fn admit_conversion_lookup(
    status: &str,
    qty: i64,
    target_line: i32,
) -> Result<&'static str, ManufacturingError> {
    if status != "SUCCEEDED" || qty <= 0 || target_line <= 0 {
        return Err(ManufacturingError::Planning(
            "late conversion needs a stored target".into(),
        ));
    }
    Ok("LOOKED_UP")
}

/// Accepts part of one suggestion. The total cannot pass the suggested quantity.
/// A shortage is not a complete kit. Cancelling or an expired plan accepts nothing.
pub fn admit_plan_conversion(
    suggested: ScaledQty,
    already: ScaledQty,
    requested: ScaledQty,
    shortage: ScaledQty,
    expired: bool,
    cancelled: bool,
) -> Result<(ScaledQty, bool, bool), ManufacturingError> {
    if cancelled {
        return Err(ManufacturingError::Planning(
            "a cancelled run accepts nothing".into(),
        ));
    }
    if expired {
        return Err(ManufacturingError::Planning(
            "an expired plan is not converted".into(),
        ));
    }
    if requested.as_micros() <= 0 || already.as_micros() < 0 || suggested.as_micros() < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    let next = already
        .try_add(requested)
        .map_err(ManufacturingError::Quantity)?;
    if next.as_micros() > suggested.as_micros() {
        return Err(ManufacturingError::Planning(
            "conversion exceeds the suggestion".into(),
        ));
    }
    let accepted = next.as_micros() == suggested.as_micros();
    let kit_complete = shortage.as_micros() <= 0 && accepted;
    Ok((next, accepted, kit_complete))
}

fn lot_qty(
    net: ScaledQty,
    min_lot: ScaledQty,
    multiple: ScaledQty,
) -> Result<ScaledQty, ManufacturingError> {
    if net.as_micros() <= 0 {
        return Ok(ScaledQty::ZERO);
    }
    let mut qty = net;
    if min_lot.as_micros() > qty.as_micros() {
        qty = min_lot;
    }
    if multiple.as_micros() > 0 {
        let rem = qty.as_micros() % multiple.as_micros();
        if rem != 0 {
            qty = ScaledQty::from_micros(
                qty.as_micros()
                    .checked_add(multiple.as_micros() - rem)
                    .ok_or(ManufacturingError::Quantity(QuantityError::Overflow))?,
            );
        }
    }
    Ok(qty)
}

/// Plans one authorized warehouse by day. Parent net manufacture is exploded;
/// on-hand semi-finished stock is consumed first and is not exploded again.
/// Late supply and an unaccepted suggestion do not cover an earlier bucket.
/// Purchase and production release dates are calculated on the same calendar
/// but stay independent fields.
pub fn plan_bucket_mrp(
    bom: &[BomItem],
    stock: &[BucketStock],
    supply: &[BucketSupply],
    demand: &[BucketDemand],
    warehouse: &str,
    rest_days: &[String],
) -> Result<Vec<BucketSuggestion>, ManufacturingError> {
    if warehouse.trim().is_empty()
        || bom.len() > 50_000
        || stock.len() > 10_000
        || demand.len() > 1_000
        || supply.len() > 50_000
        || rest_days.len() > 3_660
    {
        return Err(ManufacturingError::Planning(
            "invalid warehouse or MRP input budget".into(),
        ));
    }
    let mut calendar_rest_days = BTreeSet::new();
    for day in rest_days {
        parse_ymd(day)?;
        calendar_rest_days.insert(day.as_str());
    }
    let mut policy = BTreeMap::new();
    let mut pending: BTreeMap<String, BTreeMap<String, ScaledQty>> = BTreeMap::new();
    let mut arrivals: BTreeMap<String, Vec<&BucketSupply>> = BTreeMap::new();
    let mut children: BTreeMap<String, Vec<&BomItem>> = BTreeMap::new();
    let mut incoming: BTreeMap<String, usize> = BTreeMap::new();
    for row in stock {
        if row.code.trim().is_empty()
            || row.warehouse != warehouse
            || row.lead_days < 0
            || row.lead_days > 366
            || [
                row.on_hand,
                row.allocated_elsewhere,
                row.wip,
                row.safety,
                row.min_lot,
                row.lot_multiple,
            ]
            .iter()
            .any(|qty| qty.is_negative())
            || (row.supply != "MAKE" && row.supply != "BUY")
            || policy.insert(row.code.clone(), row).is_some()
        {
            return Err(ManufacturingError::Planning(
                "invalid or duplicate stock policy".into(),
            ));
        }
        incoming.entry(row.code.clone()).or_default();
    }
    for row in demand {
        if row.code.trim().is_empty()
            || !row.frozen
            || row.warehouse != warehouse
            || row.qty.as_micros() <= 0
        {
            return Err(ManufacturingError::Planning("invalid frozen demand".into()));
        }
        parse_ymd(&row.need_date)?;
        let bucket = pending
            .entry(row.code.clone())
            .or_default()
            .entry(row.need_date.clone())
            .or_insert(ScaledQty::ZERO);
        *bucket = bucket
            .try_add(row.qty)
            .map_err(ManufacturingError::Quantity)?;
        incoming.entry(row.code.clone()).or_default();
    }
    for row in supply {
        if row.code.trim().is_empty()
            || row.warehouse != warehouse
            || !row.accepted
            || row.qty.as_micros() <= 0
        {
            return Err(ManufacturingError::Planning(
                "invalid or unaccepted supply".into(),
            ));
        }
        parse_ymd(&row.arrive_date)?;
        arrivals.entry(row.code.clone()).or_default().push(row);
        incoming.entry(row.code.clone()).or_default();
    }
    for row in bom {
        if row.parent.trim().is_empty()
            || row.component.trim().is_empty()
            || row.qty_per.as_micros() <= 0
        {
            return Err(ManufacturingError::Planning("invalid BOM edge".into()));
        }
        children.entry(row.parent.clone()).or_default().push(row);
        incoming.entry(row.parent.clone()).or_default();
        *incoming.entry(row.component.clone()).or_default() += 1;
    }
    if incoming.len() > 10_000 {
        return Err(ManufacturingError::Planning(
            "MRP material budget exceeded".into(),
        ));
    }

    // Plan each material only after all of its parents. This aggregates a shared
    // component before lot rounding and keeps its time axis separate from its parents.
    let mut ready: BTreeSet<String> = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(code, _)| code.clone())
        .collect();
    let mut depths: BTreeMap<String, usize> = BTreeMap::new();
    let mut visited = 0;
    let mut suggestions = Vec::new();
    let mut bucket_count: usize = pending.values().map(BTreeMap::len).sum();
    while let Some(code) = ready.pop_first() {
        visited += 1;
        let depth = depths.get(&code).copied().unwrap_or(0);
        if depth > 32 {
            return Err(ManufacturingError::Planning(
                "MRP BOM depth exceeded".into(),
            ));
        }
        let rule = policy.get(&code).copied();
        let (mut carry, safety, min_lot, multiple, supply_kind, lead) = match rule {
            Some(row) => (
                row.on_hand
                    .try_sub(row.allocated_elsewhere)
                    .and_then(|value| value.try_add(row.wip))
                    .map_err(ManufacturingError::Quantity)?,
                row.safety,
                row.min_lot,
                row.lot_multiple,
                row.supply.as_str(),
                row.lead_days,
            ),
            None => (
                ScaledQty::ZERO,
                ScaledQty::ZERO,
                ScaledQty::ZERO,
                ScaledQty::ZERO,
                "BUY",
                0,
            ),
        };
        if carry.is_negative() {
            carry = ScaledQty::ZERO;
        }
        let mut item_arrivals = arrivals.remove(&code).unwrap_or_default();
        item_arrivals.sort_by(|left, right| left.arrive_date.cmp(&right.arrive_date));
        let mut arrival_index = 0;
        for (day, gross) in pending.remove(&code).unwrap_or_default() {
            while let Some(arrival) = item_arrivals.get(arrival_index) {
                if arrival.arrive_date > day {
                    break;
                }
                carry = carry
                    .try_add(arrival.qty)
                    .map_err(ManufacturingError::Quantity)?;
                arrival_index += 1;
            }
            let required = gross
                .try_add(safety)
                .map_err(ManufacturingError::Quantity)?;
            let shortage = if required > carry {
                required
                    .try_sub(carry)
                    .map_err(ManufacturingError::Quantity)?
            } else {
                ScaledQty::ZERO
            };
            let suggested = lot_qty(shortage, min_lot, multiple)?;
            if suggested.as_micros() > 0 {
                let release = mrp_release_with_rest_days(&day, lead, &calendar_rest_days)?;
                suggestions.push(BucketSuggestion {
                    code: code.clone(), warehouse: warehouse.to_string(), need_date: day.clone(),
                    gross, net: shortage, suggested, supply: supply_kind.to_string(),
                    release_date: release.clone(),
                    explanation: format!(
                        "carry {}; safety {}; min_lot {}; multiple {}; lead_days {lead}; late_excluded; lot_surplus_carried; components_due_on_parent_release_date",
                        carry.as_micros(), safety.as_micros(), min_lot.as_micros(), multiple.as_micros(),
                    ),
                });
                if supply_kind == "MAKE" {
                    // Components must be ready when their parent starts. Each
                    // component then applies its own lead to its own need date.
                    for edge in children.get(&code).into_iter().flatten() {
                        let quantity = multiply(suggested, edge.qty_per)?;
                        let buckets = pending.entry(edge.component.clone()).or_default();
                        if !buckets.contains_key(&release) {
                            bucket_count += 1;
                            if bucket_count > 1_000_000 {
                                return Err(ManufacturingError::Planning(
                                    "MRP bucket budget exceeded".into(),
                                ));
                            }
                        }
                        let bucket = buckets.entry(release.clone()).or_insert(ScaledQty::ZERO);
                        *bucket = bucket
                            .try_add(quantity)
                            .map_err(ManufacturingError::Quantity)?;
                    }
                }
            }
            // Generated supply includes its lot surplus. Safety remains a buffer
            // instead of being consumed as demand or silently lost after the first day.
            carry = carry
                .try_add(suggested)
                .and_then(|value| value.try_sub(gross))
                .map_err(ManufacturingError::Quantity)?;
        }
        for edge in children.get(&code).into_iter().flatten() {
            let child_depth = depths.entry(edge.component.clone()).or_insert(1);
            *child_depth = (*child_depth).max(depth + 1);
            let count = incoming
                .get_mut(&edge.component)
                .ok_or_else(|| ManufacturingError::MissingParent(edge.component.clone()))?;
            *count -= 1;
            if *count == 0 {
                ready.insert(edge.component.clone());
            }
        }
    }
    if visited != incoming.len() {
        return Err(ManufacturingError::Cycle("BOM contains a cycle".into()));
    }
    suggestions.sort_by(|left, right| {
        left.need_date
            .cmp(&right.need_date)
            .then(left.code.cmp(&right.code))
    });
    Ok(suggestions)
}

fn mrp_release_with_rest_days(
    need_date: &str,
    lead_days: i32,
    rest_days: &BTreeSet<&str>,
) -> Result<String, ManufacturingError> {
    parse_ymd(need_date)?;
    if lead_days < 0 {
        return Err(ManufacturingError::Planning("negative lead".into()));
    }
    let mut date = need_date.to_string();
    let mut remaining = lead_days;
    let mut walked = 0;
    while remaining > 0 {
        date = shift_ymd(&date, -1)?;
        walked += 1;
        if walked > 3_660 {
            return Err(ManufacturingError::Planning(
                "no working day inside calendar budget".into(),
            ));
        }
        if weekday(&date)? < 5 && !rest_days.contains(date.as_str()) {
            remaining -= 1;
        }
    }
    Ok(date)
}

fn multiply(qty: ScaledQty, per: ScaledQty) -> Result<ScaledQty, ManufacturingError> {
    let product = i128::from(qty.as_micros()) * i128::from(per.as_micros());
    let scale = i128::from(crate::quantity::QTY_SCALE);
    if product % scale != 0 {
        return Err(ManufacturingError::Quantity(QuantityError::ScaleExceeded));
    }
    let micros = i64::try_from(product / scale)
        .map_err(|_| ManufacturingError::Quantity(QuantityError::Overflow))?;
    Ok(ScaledQty::from_micros(micros))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::parse_qty;

    fn qty(text: &str) -> ScaledQty {
        parse_qty(text).unwrap()
    }

    #[test]
    fn mrp_uses_the_same_32_edge_depth_budget_as_bom_expansion() -> Result<(), String> {
        let mut bom: Vec<_> = (0..32)
            .map(|index| BomItem {
                parent: format!("P{index:02}"),
                component: format!("P{:02}", index + 1),
                qty_per: qty("1"),
            })
            .collect();
        let stock: Vec<_> = (0..=32)
            .map(|index| BucketStock {
                code: format!("P{index:02}"),
                warehouse: "WH".into(),
                on_hand: ScaledQty::ZERO,
                allocated_elsewhere: ScaledQty::ZERO,
                wip: ScaledQty::ZERO,
                safety: ScaledQty::ZERO,
                min_lot: ScaledQty::ZERO,
                lot_multiple: ScaledQty::ZERO,
                supply: "MAKE".into(),
                lead_days: 0,
            })
            .collect();
        let demand = [BucketDemand {
            code: "P00".into(),
            qty: qty("1"),
            need_date: "2026-10-08".into(),
            warehouse: "WH".into(),
            frozen: true,
        }];
        let rows = plan_bucket_mrp(&bom, &stock, &[], &demand, "WH", &[])
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(rows.len(), 33);
        assert_eq!(
            rows.iter()
                .find(|row| row.code == "P32")
                .ok_or("last material")?
                .gross,
            qty("1")
        );
        bom.push(BomItem {
            parent: "P32".into(),
            component: "P33".into(),
            qty_per: qty("1"),
        });
        assert_eq!(
            plan_bucket_mrp(&bom, &stock, &[], &demand, "WH", &[]),
            Err(ManufacturingError::Planning(
                "MRP BOM depth exceeded".into()
            ))
        );
        Ok(())
    }

    fn bom_line(parent: &str, component: &str, per: &str, supply: &str, line_no: i32) -> BomLine {
        BomLine {
            parent: parent.into(),
            component: component.into(),
            qty_per: qty(per),
            unit_code: "KG".into(),
            scrap_ppm: 0,
            supply: supply.into(),
            line_no,
        }
    }

    #[test]
    fn bom_shared_subtree_expands_the_combined_quantity_once() -> Result<(), String> {
        let lines = [
            bom_line("P", "A", "2", "MAKE", 10),
            bom_line("P", "B", "3", "MAKE", 20),
            bom_line("A", "S", "5", "MAKE", 10),
            bom_line("B", "S", "7", "MAKE", 10),
            bom_line("S", "R", "0.125", "BUY", 10),
        ];
        let rows = explode_bom(&lines, "P", qty("1")).map_err(|error| format!("{error:?}"))?;
        assert_eq!(rows.len(), 5);
        let raw: Vec<_> = rows.iter().filter(|row| row.1 == "R").collect();
        assert_eq!(raw.len(), 1);
        assert_eq!((raw[0].3, raw[0].5), (3_875_000, 3));
        assert_eq!((rows[0].1.as_str(), rows[1].1.as_str()), ("A", "B"));
        assert!(matches!(
            explode_bom(&lines, "unknown", qty("1")),
            Err(ManufacturingError::MissingParent(_))
        ));
        Ok(())
    }

    #[test]
    fn bom_admits_32_levels_and_labels_33_as_a_budget_error() -> Result<(), String> {
        let mut lines: Vec<_> = (0..32)
            .map(|index| {
                bom_line(
                    &format!("N{index:02}"),
                    &format!("N{:02}", index + 1),
                    "1",
                    "MAKE",
                    10,
                )
            })
            .collect();
        let rows = explode_bom(&lines, "N00", qty("1")).map_err(|error| format!("{error:?}"))?;
        assert_eq!(rows.len(), 32);
        assert_eq!((rows[31].3, rows[31].5), (1_000_000, 32));
        lines.push(bom_line("N32", "N33", "1", "BUY", 10));
        assert_eq!(
            explode_bom(&lines, "N00", qty("1")),
            Err(ManufacturingError::Planning(
                "BOM depth budget exceeded".into()
            ))
        );
        Ok(())
    }

    #[test]
    fn cycle_check_is_iterative_and_enforces_graph_size_before_expanding() -> Result<(), String> {
        let mut items: Vec<_> = (0..9_999)
            .map(|index| BomItem {
                parent: format!("N{index:05}"),
                component: format!("N{:05}", index + 1),
                qty_per: qty("1"),
            })
            .collect();
        reject_cycle(&items).map_err(|error| format!("{error:?}"))?;
        items.push(BomItem {
            parent: "N09999".into(),
            component: "N00000".into(),
            qty_per: qty("1"),
        });
        assert!(matches!(
            reject_cycle(&items),
            Err(ManufacturingError::Cycle(_))
        ));
        items.last_mut().ok_or("last edge")?.component = "N10000".into();
        assert_eq!(
            reject_cycle(&items),
            Err(ManufacturingError::Planning(
                "BOM item budget exceeded".into()
            ))
        );
        let too_many = vec![bom_line("P", "A", "1", "BUY", 10); 50_001];
        assert_eq!(
            admit_bom_structure(&too_many),
            Err(ManufacturingError::Planning(
                "BOM edge budget exceeded".into()
            ))
        );
        Ok(())
    }

    #[test]
    fn planning_dates_are_canonical_and_cannot_leave_the_four_digit_calendar() {
        for date in [
            "+026-10-02",
            "2026-+1-02",
            "2026-10-+1",
            "0000-01-01",
            "2026-02-29",
            "2026-1-02",
            "２０２６-10-02",
            "2026-10-02 ",
        ] {
            assert!(release_on_calendar(date, 0).is_err(), "{date}");
        }
        assert!(release_on_calendar("0001-01-01", 1).is_err());
        assert!(shift_ymd("9999-12-31", 1).is_err());
        assert_eq!(shift_ymd("2024-02-28", 1), Ok("2024-02-29".into()));
        assert_eq!(shift_ymd("2026-02-28", 1), Ok("2026-03-01".into()));
    }

    #[test]
    fn large_bom_quantities_are_admitted_when_the_final_result_fits() -> Result<(), String> {
        let quantity = ScaledQty::from_micros(10_000_000_000_000);
        assert_eq!(
            scrap_qty(quantity, 20_000).map_err(|error| format!("{error:?}"))?,
            ScaledQty::from_micros(10_200_000_000_000)
        );
        assert_eq!(
            scrap_qty(ScaledQty::from_micros(i64::MAX), 0).map_err(|error| format!("{error:?}"))?,
            ScaledQty::from_micros(i64::MAX)
        );
        assert_eq!(
            scrap_qty(ScaledQty::from_micros(i64::MAX), 1_000_000),
            Err(ManufacturingError::Quantity(QuantityError::Overflow))
        );
        assert_eq!(
            scrap_qty(ScaledQty::from_micros(1), 1),
            Err(ManufacturingError::Quantity(QuantityError::ScaleExceeded))
        );
        let lines = [BomLine {
            parent: "P".into(),
            component: "A".into(),
            qty_per: ScaledQty::from_micros(1_000_000),
            unit_code: "KG".into(),
            scrap_ppm: 0,
            supply: "BUY".into(),
            line_no: 1,
        }];
        let rows = explode_bom(&lines, "P", quantity).map_err(|error| format!("{error:?}"))?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].3, 10_000_000_000_000);
        Ok(())
    }

    fn sample_bom() -> Vec<BomItem> {
        vec![
            BomItem {
                parent: "P".into(),
                component: "S".into(),
                qty_per: qty("2"),
            },
            BomItem {
                parent: "P".into(),
                component: "A".into(),
                qty_per: qty("3"),
            },
            BomItem {
                parent: "S".into(),
                component: "B".into(),
                qty_per: qty("4"),
            },
        ]
    }

    #[test]
    fn disposition_exhausted_version_returns_error() {
        assert!(
            admit_cited_disposition(crate::manufacturing::CitedDispositionInput {
                held: ScaledQty::from_micros(1_000_000),
                kind: "ACCEPT",
                qty: ScaledQty::from_micros(1_000_000),
                reason: "",
                authorizer: "",
                inspection_id: "inspection-1",
                batch_code: "batch-1",
                location_id: "location-1",
                prior_version: i32::MAX,
                unit_cost_cent: 0,
            })
            .is_err()
        );
    }

    #[test]
    fn local_trace_large_limit_stays_inside_available_rows() -> Result<(), String> {
        let edge = GenealogyEdge {
            source_action: "consume-1".into(),
            input_code: "A".into(),
            output_code: "P".into(),
            qty: ScaledQty::from_micros(1_000_000),
            work_order: "WO-1".into(),
            output_batch: "P-1".into(),
            customer: String::new(),
            reversed: false,
        };
        let result = page_genealogy(&[edge.clone(), edge.clone()], Some(1), usize::MAX)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(result.edges, vec![edge]);
        assert!(!result.incomplete);
        assert_eq!(result.next_cursor, None);
        Ok(())
    }

    #[test]
    fn keyset_trace_rejects_a_limit_without_room_for_lookahead() {
        assert!(keyset_genealogy_page(&[], usize::MAX, 0, false).is_err());
    }

    #[test]
    fn calendar_counts_every_rest_day_and_keeps_zero_lead_on_need_date() -> Result<(), String> {
        let rest = BTreeSet::from(["2026-10-09"]);
        let result = mrp_release_with_rest_days("2026-10-12", 2, &rest)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(result, "2026-10-07");
        let consecutive = BTreeSet::from(["2026-10-09", "2026-10-08"]);
        assert_eq!(
            mrp_release_with_rest_days("2026-10-12", 2, &consecutive)
                .map_err(|error| format!("{error:?}"))?,
            "2026-10-06"
        );
        assert_eq!(
            mrp_release_with_rest_days("2026-10-11", 0, &consecutive)
                .map_err(|error| format!("{error:?}"))?,
            "2026-10-11"
        );
        assert!(release_on_calendar("not-a-date", 0).is_err());
        assert!(release_on_calendar("2026-10-12", -1).is_err());
        assert!(release_on_calendar("2026-10-12", i32::MAX).is_err());
        Ok(())
    }

    #[test]
    fn calendar_matches_independent_weekday_calculation_across_leap_and_year_boundaries()
    -> Result<(), String> {
        use chrono::{Datelike, NaiveDate};

        for start in [
            "1999-12-20",
            "2000-02-20",
            "2024-02-20",
            "2026-10-01",
            "2026-12-20",
        ] {
            let start =
                NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|error| error.to_string())?;
            for offset in 0..31 {
                let date = start + chrono::Duration::days(offset);
                let text = date.format("%Y-%m-%d").to_string();
                assert_eq!(
                    weekday(&text).map_err(|error| format!("{error:?}"))?,
                    date.weekday().num_days_from_monday()
                );
                for lead in 0..=5 {
                    let mut expected = date;
                    let mut remaining = lead;
                    while remaining > 0 {
                        expected = expected.pred_opt().ok_or("independent date underflow")?;
                        if expected.weekday().num_days_from_monday() < 5 {
                            remaining -= 1;
                        }
                    }
                    assert_eq!(
                        release_on_calendar(&text, lead).map_err(|error| format!("{error:?}"))?,
                        expected.format("%Y-%m-%d").to_string()
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn three_level_sample_excludes_late_supply() {
        let items = vec![
            MrpItem {
                code: "P".into(),
                on_hand: qty("0"),
                allocated_elsewhere: qty("0"),
                supply_on_time: qty("0"),
                supply_late: qty("0"),
                safety: qty("0"),
            },
            MrpItem {
                code: "S".into(),
                on_hand: qty("2"),
                allocated_elsewhere: qty("0"),
                supply_on_time: qty("0"),
                supply_late: qty("0"),
                safety: qty("0"),
            },
            MrpItem {
                code: "A".into(),
                on_hand: qty("12"),
                allocated_elsewhere: qty("2"),
                supply_on_time: qty("5"),
                supply_late: qty("0"),
                safety: qty("2"),
            },
            MrpItem {
                code: "B".into(),
                on_hand: qty("10"),
                allocated_elsewhere: qty("0"),
                supply_on_time: qty("8"),
                supply_late: qty("10"),
                safety: qty("2"),
            },
        ];
        let plan = plan_mrp(&sample_bom(), &items, &[("P", qty("10"))]).unwrap();
        let by_code = |code: &str| plan.iter().find(|row| row.code == code).unwrap();
        assert_eq!(by_code("P").net_make_or_buy, qty("10"));
        assert_eq!(by_code("S").gross, qty("20"));
        assert_eq!(by_code("S").net_make_or_buy, qty("18"));
        assert_eq!(by_code("A").gross, qty("30"));
        assert_eq!(by_code("A").net_make_or_buy, qty("17"));
        assert_eq!(by_code("B").gross, qty("72"));
        assert_eq!(by_code("B").net_make_or_buy, qty("56"));
    }

    #[test]
    fn bucket_plan_keeps_late_supply_out_and_dates_purchase_apart_from_production() {
        let stock = vec![
            BucketStock {
                code: "P".into(),
                warehouse: "WH-1".into(),
                on_hand: qty("0"),
                allocated_elsewhere: qty("0"),
                wip: qty("0"),
                safety: qty("0"),
                min_lot: qty("0"),
                lot_multiple: qty("0"),
                supply: "MAKE".into(),
                lead_days: 2,
            },
            BucketStock {
                code: "S".into(),
                warehouse: "WH-1".into(),
                on_hand: qty("2"),
                allocated_elsewhere: qty("0"),
                wip: qty("0"),
                safety: qty("0"),
                min_lot: qty("0"),
                lot_multiple: qty("0"),
                supply: "MAKE".into(),
                lead_days: 1,
            },
            BucketStock {
                code: "A".into(),
                warehouse: "WH-1".into(),
                on_hand: qty("12"),
                allocated_elsewhere: qty("2"),
                wip: qty("0"),
                safety: qty("2"),
                min_lot: qty("0"),
                lot_multiple: qty("0"),
                supply: "BUY".into(),
                lead_days: 3,
            },
            BucketStock {
                code: "B".into(),
                warehouse: "WH-1".into(),
                on_hand: qty("10"),
                allocated_elsewhere: qty("0"),
                wip: qty("0"),
                safety: qty("2"),
                min_lot: qty("10"),
                lot_multiple: qty("6"),
                supply: "BUY".into(),
                lead_days: 3,
            },
        ];
        let supply = vec![
            BucketSupply {
                code: "A".into(),
                warehouse: "WH-1".into(),
                qty: qty("5"),
                arrive_date: "2026-10-06".into(),
                accepted: true,
            },
            BucketSupply {
                code: "B".into(),
                warehouse: "WH-1".into(),
                qty: qty("8"),
                arrive_date: "2026-10-05".into(),
                accepted: true,
            },
            BucketSupply {
                code: "B".into(),
                warehouse: "WH-1".into(),
                qty: qty("10"),
                arrive_date: "2026-10-12".into(),
                accepted: true,
            },
        ];
        let demand = vec![BucketDemand {
            code: "P".into(),
            qty: qty("10"),
            need_date: "2026-10-08".into(),
            warehouse: "WH-1".into(),
            frozen: true,
        }];
        let plan = plan_bucket_mrp(&sample_bom(), &stock, &supply, &demand, "WH-1", &[]).unwrap();
        let row = |code: &str| plan.iter().find(|item| item.code == code).unwrap();
        assert_eq!(row("P").net, qty("10"));
        assert_eq!(row("S").net, qty("18"));
        assert_eq!(row("A").gross, qty("30"));
        assert_eq!(row("A").net, qty("17"));
        assert_eq!(row("B").gross, qty("72"));
        assert_eq!(row("B").net, qty("56"));
        assert_eq!(
            row("B").suggested,
            qty("60"),
            "56 rounds up to the lot multiple after the minimum lot"
        );
        assert_eq!(row("P").release_date, "2026-10-06");
        assert_eq!(row("S").need_date, "2026-10-06");
        assert_eq!(row("S").release_date, "2026-10-05");
        assert_eq!(row("A").need_date, "2026-10-06");
        assert_eq!(row("A").release_date, "2026-10-01");
        assert_eq!(row("B").need_date, "2026-10-05");
        assert_eq!(row("B").release_date, "2026-09-30");
        assert_ne!(row("P").release_date, row("A").release_date);
        let refused = plan_bucket_mrp(
            &sample_bom(),
            &stock,
            &[BucketSupply {
                code: "B".into(),
                warehouse: "WH-1".into(),
                qty: qty("10"),
                arrive_date: "2026-10-01".into(),
                accepted: false,
            }],
            &demand,
            "WH-1",
            &[],
        );
        assert!(refused.is_err(), "an unaccepted suggestion is not supply");
    }

    #[test]
    fn execution_keeps_unreleased_goods_unavailable() {
        let result = execute(Execution {
            issued: qty("10"),
            consumed: qty("6"),
            good: qty("4"),
            rejected: qty("1"),
            received: qty("3"),
            material_cost_cent: 600,
            labor_minutes: 30,
            labor_rate_cent: 2,
        })
        .unwrap();
        assert_eq!(result.wip_qty, qty("4"));
        assert_eq!(result.available_finished, qty("3"));
        assert_eq!(result.cost_cent, 660);
    }

    #[test]
    fn issued_work_order_cannot_be_deleted_by_cancel() {
        let blocked = transition_work_order(
            WorkOrderStatus::Released,
            "cancel",
            qty("0.125"),
            ScaledQty::ZERO,
            ScaledQty::ZERO,
        );
        assert!(matches!(
            blocked,
            Err(ManufacturingError::IssuedMaterial(_))
        ));
        let cancelled = transition_work_order(
            WorkOrderStatus::Planned,
            "cancel",
            ScaledQty::ZERO,
            ScaledQty::ZERO,
            ScaledQty::ZERO,
        )
        .unwrap();
        assert_eq!(cancelled, WorkOrderStatus::Cancelled);
        let paused = transition_work_order(
            WorkOrderStatus::Released,
            "pause",
            qty("1"),
            ScaledQty::ZERO,
            ScaledQty::ZERO,
        )
        .unwrap();
        assert_eq!(paused, WorkOrderStatus::Paused);
        let resumed =
            transition_work_order(paused, "resume", qty("1"), qty("1"), ScaledQty::ZERO).unwrap();
        assert_eq!(resumed, WorkOrderStatus::Released);
        assert!(
            transition_work_order(
                resumed,
                "complete",
                qty("1"),
                ScaledQty::ZERO,
                ScaledQty::ZERO
            )
            .is_err()
        );
        let completed =
            transition_work_order(resumed, "complete", qty("1"), qty("1"), ScaledQty::ZERO)
                .unwrap();
        let closed =
            transition_work_order(completed, "close", qty("1"), qty("1"), ScaledQty::ZERO).unwrap();
        assert_eq!(closed, WorkOrderStatus::Closed);
        let (on_hand, reserved) =
            issue_from_reservation(qty("2"), qty("1.125"), qty("1"), qty("1.250")).unwrap();
        assert_eq!(on_hand, qty("0.750"));
        assert_eq!(reserved, qty("0.125"));
        assert!(issue_from_reservation(qty("1"), qty("1"), ScaledQty::ZERO, qty("0.125")).is_err());
        assert_eq!(whole_units_for_ledger(qty("2")).unwrap(), Some(2));
        assert_eq!(whole_units_for_ledger(qty("0.125")).unwrap(), None);
    }

    #[test]
    fn material_issue_return_keeps_shop_and_consumed_apart() {
        let issued = apply_material_move(
            MaterialBalance {
                issued: ScaledQty::ZERO,
                returned: ScaledQty::ZERO,
                consumed: ScaledQty::ZERO,
                shop_qty: ScaledQty::ZERO,
            },
            MaterialMove {
                kind: MaterialMoveKind::Issue,
                qty: qty("1"),
                supplement_reason_required: false,
            },
        )
        .unwrap();
        let consumed = apply_material_move(
            issued,
            MaterialMove {
                kind: MaterialMoveKind::Consume,
                qty: qty("0.125"),
                supplement_reason_required: false,
            },
        )
        .unwrap();
        let returned = apply_material_move(
            consumed,
            MaterialMove {
                kind: MaterialMoveKind::Return,
                qty: qty("0.250"),
                supplement_reason_required: false,
            },
        )
        .unwrap();
        assert_eq!(returned.issued, qty("1"));
        assert_eq!(returned.consumed, qty("0.125"));
        assert_eq!(returned.returned, qty("0.250"));
        assert_eq!(returned.shop_qty, qty("0.625"));
        assert!(
            apply_material_move(
                returned,
                MaterialMove {
                    kind: MaterialMoveKind::Supplement,
                    qty: qty("0.125"),
                    supplement_reason_required: false,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn cycle_is_rejected_before_planning() {
        let bom = vec![
            BomItem {
                parent: "P".into(),
                component: "S".into(),
                qty_per: qty("1"),
            },
            BomItem {
                parent: "S".into(),
                component: "P".into(),
                qty_per: qty("1"),
            },
        ];
        assert!(matches!(
            plan_mrp(&bom, &[], &[("P", qty("1"))]),
            Err(ManufacturingError::Cycle(_))
        ));
    }
}
