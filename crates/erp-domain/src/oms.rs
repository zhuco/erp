use erp_types::{
    OmsOrderStatus, OrderExceptionStatus, OrderFulfillmentStatus, OrderMainStatus,
    OrderReviewStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmsOrderAction {
    ReviewPass,
    ReviewReject,
    ReviewFlag,
    Allocate,
    Pick,
    Pack,
    Ship,
    Complete,
    Cancel,
}

pub fn transition_status(
    current: OmsOrderStatus,
    action: OmsOrderAction,
) -> Result<OmsOrderStatus, String> {
    match (current, action) {
        (OmsOrderStatus::New, OmsOrderAction::ReviewPass) => Ok(OmsOrderStatus::PendingShipment),
        (OmsOrderStatus::ReviewPending, OmsOrderAction::ReviewPass) => {
            Ok(OmsOrderStatus::PendingShipment)
        }
        (OmsOrderStatus::PendingShipment, OmsOrderAction::ReviewPass) => {
            Ok(OmsOrderStatus::PendingShipment)
        }
        (OmsOrderStatus::New, OmsOrderAction::ReviewReject) => Ok(OmsOrderStatus::ReviewPending),
        (OmsOrderStatus::ReviewPending, OmsOrderAction::ReviewReject) => {
            Ok(OmsOrderStatus::ReviewPending)
        }
        (OmsOrderStatus::New, OmsOrderAction::ReviewFlag) => Ok(OmsOrderStatus::ReviewPending),
        (OmsOrderStatus::ReviewPending, OmsOrderAction::ReviewFlag) => {
            Ok(OmsOrderStatus::ReviewPending)
        }
        (OmsOrderStatus::PendingShipment, OmsOrderAction::Allocate) => {
            Ok(OmsOrderStatus::Allocated)
        }
        (OmsOrderStatus::Allocated, OmsOrderAction::Pick) => Ok(OmsOrderStatus::Picking),
        (OmsOrderStatus::Picking, OmsOrderAction::Pack) => Ok(OmsOrderStatus::Packed),
        (OmsOrderStatus::Packed, OmsOrderAction::Ship) => Ok(OmsOrderStatus::Shipped),
        (OmsOrderStatus::Shipped, OmsOrderAction::Complete) => Ok(OmsOrderStatus::Done),
        (
            OmsOrderStatus::New | OmsOrderStatus::ReviewPending | OmsOrderStatus::PendingShipment,
            OmsOrderAction::Cancel,
        ) => Ok(OmsOrderStatus::Cancelled),
        _ => Err(format!("invalid transition: {current:?} -> {action:?}")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    Pass,
    Reject,
    Flag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone)]
pub struct ReviewOutcome {
    pub oms_status: OmsOrderStatus,
    pub review_status: OrderReviewStatus,
    pub exception_flags: Vec<String>,
}

pub fn apply_review_decision(
    current_status: OmsOrderStatus,
    action: ReviewAction,
    existing_flags: &[String],
    reason: Option<&str>,
    mode: ReviewMode,
) -> Result<ReviewOutcome, String> {
    let mut exception_flags = existing_flags.to_vec();
    let (next_status, review_status, default_flag, transition_action) = match action {
        ReviewAction::Pass => (
            transition_status(current_status, OmsOrderAction::ReviewPass)?,
            match mode {
                ReviewMode::Auto => OrderReviewStatus::AutoApproved,
                ReviewMode::Manual => OrderReviewStatus::Approved,
            },
            None,
            None,
        ),
        ReviewAction::Reject => (
            transition_status(current_status, OmsOrderAction::ReviewReject)?,
            OrderReviewStatus::Rejected,
            Some("REVIEW_REJECTED"),
            Some(OmsOrderAction::ReviewReject),
        ),
        ReviewAction::Flag => (
            transition_status(current_status, OmsOrderAction::ReviewFlag)?,
            OrderReviewStatus::ManualReview,
            Some("REVIEW_FLAGGED"),
            Some(OmsOrderAction::ReviewFlag),
        ),
    };

    if let Some(flag) = default_flag {
        push_flag(&mut exception_flags, flag);
    }
    if let Some(reason) = reason {
        push_flag(&mut exception_flags, reason);
    }
    if transition_action.is_some() {
        return Ok(ReviewOutcome {
            oms_status: next_status,
            review_status,
            exception_flags,
        });
    }
    Ok(ReviewOutcome {
        oms_status: next_status,
        review_status,
        exception_flags,
    })
}

pub fn derive_exception_status(flags: &[String]) -> OrderExceptionStatus {
    if flags.iter().any(|flag| flag == "BLACKLISTED_CUSTOMER") {
        return OrderExceptionStatus::CustomerBlacklist;
    }
    if flags.iter().any(|flag| flag == "ADDRESS_INVALID") {
        return OrderExceptionStatus::AddressError;
    }
    if flags.iter().any(|flag| flag == "INVENTORY_SHORTAGE") {
        return OrderExceptionStatus::OutOfStock;
    }
    if flags.iter().any(|flag| flag == "DUPLICATE_ORDER") {
        return OrderExceptionStatus::Risk;
    }
    if flags.iter().any(|flag| flag == "REVIEW_FLAGGED") {
        return OrderExceptionStatus::ManualHold;
    }
    if flags.iter().any(|flag| flag == "REVIEW_REJECTED") {
        return OrderExceptionStatus::ManualHold;
    }
    if flags.is_empty() {
        OrderExceptionStatus::None
    } else {
        OrderExceptionStatus::ManualHold
    }
}

pub fn derive_order_status(
    oms_status: OmsOrderStatus,
    review_status: OrderReviewStatus,
    fulfillment_status: OrderFulfillmentStatus,
    exception_status: OrderExceptionStatus,
) -> OrderMainStatus {
    if oms_status == OmsOrderStatus::Cancelled {
        return OrderMainStatus::Cancelled;
    }
    if oms_status == OmsOrderStatus::Done {
        return OrderMainStatus::Completed;
    }
    if fulfillment_status == OrderFulfillmentStatus::Signed {
        return OrderMainStatus::Signed;
    }
    if fulfillment_status == OrderFulfillmentStatus::Shipped {
        return OrderMainStatus::Shipped;
    }
    if matches!(
        fulfillment_status,
        OrderFulfillmentStatus::Picking
            | OrderFulfillmentStatus::Packing
            | OrderFulfillmentStatus::ReadyToShip
    ) {
        return OrderMainStatus::Picking;
    }
    if review_status == OrderReviewStatus::Rejected {
        return OrderMainStatus::ReviewRejected;
    }
    if !matches!(
        exception_status,
        OrderExceptionStatus::None | OrderExceptionStatus::Released | OrderExceptionStatus::Closed
    ) {
        return OrderMainStatus::ExceptionHold;
    }
    if review_status.is_pending() {
        return OrderMainStatus::PendingReview;
    }
    if oms_status == OmsOrderStatus::PendingShipment {
        return OrderMainStatus::PendingAllocate;
    }
    if oms_status == OmsOrderStatus::Allocated {
        return OrderMainStatus::PendingOutbound;
    }
    OrderMainStatus::PendingAllocate
}

pub fn ensure_order_review_reopen_allowed(
    oms_status: OmsOrderStatus,
    order_status: OrderMainStatus,
    fulfillment_status: OrderFulfillmentStatus,
) -> Result<(), &'static str> {
    if matches!(
        oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) || matches!(
        order_status,
        OrderMainStatus::Shipped
            | OrderMainStatus::Signed
            | OrderMainStatus::Completed
            | OrderMainStatus::Cancelled
    ) || matches!(
        fulfillment_status,
        OrderFulfillmentStatus::Shipped | OrderFulfillmentStatus::Signed
    ) {
        return Err("order terminal status not review reopenable");
    }

    Ok(())
}

fn push_flag(flags: &mut Vec<String>, value: &str) {
    if !flags.iter().any(|existing| existing == value) {
        flags.push(value.to_string());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReviewCondition {
    pub field_path: String,
    pub op: String,
    pub value: Value,
}

#[derive(Debug, Clone, Default)]
pub struct OrderReviewContext {
    fields: HashMap<String, Value>,
}

impl OrderReviewContext {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn insert_field(&mut self, key: impl Into<String>, value: Value) {
        self.fields.insert(key.into(), value);
    }

    pub fn value_for(&self, field_path: &str) -> Option<&Value> {
        self.fields.get(field_path)
    }
}

pub fn evaluate_conditions(
    context: &OrderReviewContext,
    conditions: &[OrderReviewCondition],
) -> bool {
    conditions
        .iter()
        .all(|cond| evaluate_condition(context, cond))
}

fn evaluate_condition(context: &OrderReviewContext, condition: &OrderReviewCondition) -> bool {
    let Some(left) = context.value_for(&condition.field_path) else {
        return false;
    };
    match condition.op.as_str() {
        ">" => compare_number(left, &condition.value, |l, r| l > r),
        ">=" => compare_number(left, &condition.value, |l, r| l >= r),
        "<" => compare_number(left, &condition.value, |l, r| l < r),
        "<=" => compare_number(left, &condition.value, |l, r| l <= r),
        "=" | "==" => compare_eq(left, &condition.value),
        "!=" => !compare_eq(left, &condition.value),
        "in" => compare_in(left, &condition.value),
        "contains" => compare_contains(left, &condition.value),
        _ => false,
    }
}

fn compare_number<F>(left: &Value, right: &Value, op: F) -> bool
where
    F: Fn(i64, i64) -> bool,
{
    let Some(l) = left.as_i64() else { return false };
    let Some(r) = right.as_i64() else {
        return false;
    };
    op(l, r)
}

fn compare_eq(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Number(l), Value::Number(r)) => l.as_i64() == r.as_i64(),
        (Value::Bool(l), Value::Bool(r)) => l == r,
        _ => false,
    }
}

fn compare_in(left: &Value, right: &Value) -> bool {
    match right {
        Value::Array(arr) => arr.iter().any(|item| compare_eq(left, item)),
        Value::String(_) | Value::Number(_) | Value::Bool(_) => compare_eq(left, right),
        _ => false,
    }
}

fn compare_contains(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(l), Value::String(r)) => l.contains(r),
        (Value::Array(arr), Value::String(r)) => arr
            .iter()
            .any(|item| item.as_str().map(|val| val == r).unwrap_or(false)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transition_review_pass() {
        let next = transition_status(OmsOrderStatus::New, OmsOrderAction::ReviewPass)
            .expect("transition ok");
        assert_eq!(next, OmsOrderStatus::PendingShipment);
    }

    #[test]
    fn transition_review_reject_stays_pending() {
        let next = transition_status(OmsOrderStatus::ReviewPending, OmsOrderAction::ReviewReject)
            .expect("transition ok");
        assert_eq!(next, OmsOrderStatus::ReviewPending);
    }

    #[test]
    fn review_reopen_guard_rejects_terminal_statuses() {
        for order_status in [
            OrderMainStatus::Shipped,
            OrderMainStatus::Signed,
            OrderMainStatus::Completed,
            OrderMainStatus::Cancelled,
        ] {
            let error = ensure_order_review_reopen_allowed(
                OmsOrderStatus::PendingShipment,
                order_status,
                OrderFulfillmentStatus::NotAllocated,
            )
            .expect_err("terminal order status should be blocked");
            assert_eq!(error, "order terminal status not review reopenable");
        }

        for oms_status in [
            OmsOrderStatus::Shipped,
            OmsOrderStatus::Done,
            OmsOrderStatus::Cancelled,
        ] {
            let error = ensure_order_review_reopen_allowed(
                oms_status,
                OrderMainStatus::PendingOutbound,
                OrderFulfillmentStatus::NotAllocated,
            )
            .expect_err("terminal oms status should be blocked");
            assert_eq!(error, "order terminal status not review reopenable");
        }

        for fulfillment_status in [
            OrderFulfillmentStatus::Shipped,
            OrderFulfillmentStatus::Signed,
        ] {
            let error = ensure_order_review_reopen_allowed(
                OmsOrderStatus::PendingShipment,
                OrderMainStatus::PendingOutbound,
                fulfillment_status,
            )
            .expect_err("terminal fulfillment status should be blocked");
            assert_eq!(error, "order terminal status not review reopenable");
        }
    }

    #[test]
    fn review_reopen_guard_allows_pre_shipment_statuses() {
        ensure_order_review_reopen_allowed(
            OmsOrderStatus::PendingShipment,
            OrderMainStatus::PendingAllocate,
            OrderFulfillmentStatus::NotAllocated,
        )
        .expect("pending allocation can reopen review");

        ensure_order_review_reopen_allowed(
            OmsOrderStatus::Packed,
            OrderMainStatus::Picking,
            OrderFulfillmentStatus::ReadyToShip,
        )
        .expect("pre-shipment fulfillment can reopen review");
    }

    #[test]
    fn evaluate_condition_numeric() {
        let mut ctx = OrderReviewContext::new();
        ctx.insert_field("total_cent", json!(52000));
        let condition = OrderReviewCondition {
            field_path: "total_cent".to_string(),
            op: ">".to_string(),
            value: json!(50000),
        };
        assert!(evaluate_conditions(&ctx, &[condition]));
    }

    #[test]
    fn evaluate_condition_in_list() {
        let mut ctx = OrderReviewContext::new();
        ctx.insert_field("address_province", json!("西藏"));
        let condition = OrderReviewCondition {
            field_path: "address_province".to_string(),
            op: "in".to_string(),
            value: json!(["西藏", "新疆"]),
        };
        assert!(evaluate_conditions(&ctx, &[condition]));
    }
}
