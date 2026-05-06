use super::*;

pub(crate) fn purchase_order_summary_response(
    order: PurchaseOrder,
) -> PurchaseOrderSummaryResponse {
    PurchaseOrderSummaryResponse {
        po_id: order.po_id,
        supplier_id: order.supplier_id,
        supplier_name: order.supplier_name,
        warehouse_id: order.warehouse_id,
        expected_at: order.expected_at,
        remark: order.remark,
        status: order.status,
        purchase_order_status: order.purchase_order_status,
        total_cent: order.total_cent,
        created_at: order.created_at,
        updated_at: order.updated_at,
    }
}

pub(crate) fn purchase_order_item_response(item: PurchaseOrderItem) -> PurchaseOrderItemResponse {
    PurchaseOrderItemResponse {
        sku_id: item.sku_id,
        sku_code: item.sku_code,
        qty: item.qty,
        price_cent: item.price_cent,
        received_qty: item.received_qty,
    }
}

pub(crate) fn purchase_receipt_summary_response(
    summary: PurchaseReceiptSummary,
) -> PurchaseReceiptSummaryResponse {
    PurchaseReceiptSummaryResponse {
        receipt_id: summary.receipt_id,
        po_id: summary.po_id,
        supplier_id: summary.supplier_id,
        supplier_name: summary.supplier_name,
        warehouse_id: summary.warehouse_id,
        status: summary.status,
        receiving_status: summary.receiving_status,
        confirmed_at: summary.confirmed_at,
        created_at: summary.created_at,
    }
}

pub(crate) fn purchase_order_detail_response(
    detail: PurchaseOrderDetail,
) -> PurchaseOrderDetailResponse {
    PurchaseOrderDetailResponse {
        order: purchase_order_summary_response(detail.order),
        items: detail
            .items
            .into_iter()
            .map(purchase_order_item_response)
            .collect(),
        receipts: detail
            .receipts
            .into_iter()
            .map(purchase_receipt_summary_response)
            .collect(),
    }
}

pub(crate) fn purchase_receipt_response(receipt: PurchaseReceipt) -> PurchaseReceiptResponse {
    PurchaseReceiptResponse {
        receipt_id: receipt.receipt_id,
        po_id: receipt.po_id,
        supplier_id: receipt.supplier_id,
        supplier_name: receipt.supplier_name,
        warehouse_id: receipt.warehouse_id,
        status: receipt.status,
        receiving_status: receipt.receiving_status,
        confirmed_at: receipt.confirmed_at,
        created_at: receipt.created_at,
    }
}

pub(crate) fn purchase_receipt_detail_response(
    detail: PurchaseReceiptDetail,
) -> PurchaseReceiptDetailResponse {
    PurchaseReceiptDetailResponse {
        receipt: purchase_receipt_response(detail.receipt),
        items: detail
            .items
            .into_iter()
            .map(purchase_receipt_item_response)
            .collect(),
    }
}

pub(crate) fn purchase_receipt_item_response(
    item: PurchaseReceiptItem,
) -> PurchaseReceiptItemResponse {
    PurchaseReceiptItemResponse {
        sku_id: item.sku_id,
        qty: item.qty,
    }
}

pub(crate) fn purchase_suggestion_response(
    suggestion: PurchaseSuggestion,
) -> PurchaseSuggestionResponse {
    PurchaseSuggestionResponse {
        sku_id: suggestion.sku_id,
        sku_code: suggestion.sku_code,
        suggested_qty: suggestion.suggested_qty,
        reason: suggestion.reason,
        computed_at: suggestion.computed_at,
    }
}

pub(crate) fn purchase_return_summary_response(
    purchase_return: PurchaseReturn,
) -> PurchaseReturnSummaryResponse {
    PurchaseReturnSummaryResponse {
        return_id: purchase_return.return_id,
        po_id: purchase_return.po_id,
        supplier_id: purchase_return.supplier_id,
        supplier_name: purchase_return.supplier_name,
        warehouse_id: purchase_return.warehouse_id,
        status: purchase_return.status,
        confirmed_at: purchase_return.confirmed_at,
        created_at: purchase_return.created_at,
    }
}

pub(crate) fn purchase_return_detail_response(
    detail: PurchaseReturnDetail,
) -> PurchaseReturnDetailResponse {
    PurchaseReturnDetailResponse {
        purchase_return: purchase_return_summary_response(detail.purchase_return),
        items: detail
            .items
            .into_iter()
            .map(|item| PurchaseReturnItemResponse {
                sku_id: item.sku_id,
                qty: item.qty,
            })
            .collect(),
    }
}

pub(crate) fn procurement_request_response(
    request: ProcurementRequest,
) -> ProcurementRequestResponse {
    ProcurementRequestResponse {
        request_id: request.request_id,
        department: request.department,
        requester: request.requester,
        status: request.status,
        budget_cent: request.budget_cent,
        expected_at: request.expected_at,
        remark: request.remark,
        created_at: request.created_at,
        updated_at: request.updated_at,
    }
}

pub(crate) fn procurement_request_item_response(
    item: ProcurementRequestItem,
) -> ProcurementRequestItemResponse {
    ProcurementRequestItemResponse {
        sku_id: item.sku_id,
        sku_code: item.sku_code,
        qty: item.qty,
        price_cent: item.price_cent,
    }
}

pub(crate) fn procurement_request_detail_response(
    detail: ProcurementRequestDetail,
) -> ProcurementRequestDetailResponse {
    ProcurementRequestDetailResponse {
        request: procurement_request_response(detail.request),
        items: detail
            .items
            .into_iter()
            .map(procurement_request_item_response)
            .collect(),
    }
}

pub(crate) fn procurement_plan_response(plan: ProcurementPlan) -> ProcurementPlanResponse {
    ProcurementPlanResponse {
        plan_id: plan.plan_id,
        request_id: plan.request_id,
        supplier_id: plan.supplier_id,
        supplier_name: plan.supplier_name,
        status: plan.status,
        total_cent: plan.total_cent,
        planned_at: plan.planned_at,
        expected_at: plan.expected_at,
        remark: plan.remark,
        created_at: plan.created_at,
        updated_at: plan.updated_at,
    }
}

pub(crate) fn procurement_plan_item_response(
    item: ProcurementPlanItem,
) -> ProcurementPlanItemResponse {
    ProcurementPlanItemResponse {
        sku_id: item.sku_id,
        sku_code: item.sku_code,
        qty: item.qty,
        price_cent: item.price_cent,
    }
}

pub(crate) fn procurement_plan_detail_response(
    detail: ProcurementPlanDetail,
) -> ProcurementPlanDetailResponse {
    ProcurementPlanDetailResponse {
        plan: procurement_plan_response(detail.plan),
        items: detail
            .items
            .into_iter()
            .map(procurement_plan_item_response)
            .collect(),
    }
}

pub(crate) fn procurement_sourcing_response(
    sourcing: ProcurementSourcing,
) -> ProcurementSourcingResponse {
    ProcurementSourcingResponse {
        sourcing_id: sourcing.sourcing_id,
        title: sourcing.title,
        category: sourcing.category,
        supplier_id: sourcing.supplier_id,
        supplier_name: sourcing.supplier_name,
        status: sourcing.status,
        target_price_cent: sourcing.target_price_cent,
        remark: sourcing.remark,
        created_at: sourcing.created_at,
        updated_at: sourcing.updated_at,
    }
}

pub(crate) fn procurement_inspection_response(
    inspection: ProcurementInspection,
) -> ProcurementInspectionResponse {
    ProcurementInspectionResponse {
        inspection_id: inspection.inspection_id,
        receipt_id: inspection.receipt_id,
        supplier_id: inspection.supplier_id,
        supplier_name: inspection.supplier_name,
        warehouse_id: inspection.warehouse_id,
        status: inspection.status,
        inspected_at: inspection.inspected_at,
        remark: inspection.remark,
        created_at: inspection.created_at,
        updated_at: inspection.updated_at,
    }
}

pub(crate) fn procurement_inspection_item_response(
    item: ProcurementInspectionItem,
) -> ProcurementInspectionItemResponse {
    ProcurementInspectionItemResponse {
        sku_id: item.sku_id,
        sku_code: item.sku_code,
        qty: item.qty,
        passed_qty: item.passed_qty,
        failed_qty: item.failed_qty,
        result: item.result,
    }
}

pub(crate) fn procurement_inspection_detail_response(
    detail: ProcurementInspectionDetail,
) -> ProcurementInspectionDetailResponse {
    ProcurementInspectionDetailResponse {
        inspection: procurement_inspection_response(detail.inspection),
        items: detail
            .items
            .into_iter()
            .map(procurement_inspection_item_response)
            .collect(),
    }
}

pub(crate) fn procurement_reconciliation_response(
    reconciliation: ProcurementReconciliation,
) -> ProcurementReconciliationResponse {
    ProcurementReconciliationResponse {
        reconciliation_id: reconciliation.reconciliation_id,
        supplier_id: reconciliation.supplier_id,
        supplier_name: reconciliation.supplier_name,
        period_start: reconciliation.period_start,
        period_end: reconciliation.period_end,
        amount_cent: reconciliation.amount_cent,
        status: reconciliation.status,
        remark: reconciliation.remark,
        created_at: reconciliation.created_at,
        updated_at: reconciliation.updated_at,
    }
}

pub(crate) fn supplier_performance_response(
    performance: SupplierPerformance,
) -> SupplierPerformanceResponse {
    SupplierPerformanceResponse {
        performance_id: performance.performance_id,
        supplier_id: performance.supplier_id,
        supplier_name: performance.supplier_name,
        period_start: performance.period_start,
        period_end: performance.period_end,
        delivery_score: performance.delivery_score,
        quality_score: performance.quality_score,
        price_score: performance.price_score,
        total_score: performance.total_score,
        status: performance.status,
        remark: performance.remark,
        created_at: performance.created_at,
        updated_at: performance.updated_at,
    }
}

pub(crate) fn normalize_purchase_order_status(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "DRAFT" | "PENDING_APPROVAL" | "PLACED" | "PARTIALLY_RECEIVED" | "RECEIVED" | "CLOSED" => {
            Some(value)
        }
        "SENT" => Some("PENDING_APPROVAL".to_string()),
        "CONFIRMED" => Some("PLACED".to_string()),
        "RECEIVING" => Some("PARTIALLY_RECEIVED".to_string()),
        "DONE" => Some("RECEIVED".to_string()),
        "CANCELLED" => Some("CLOSED".to_string()),
        _ => None,
    }
}

pub(crate) fn normalize_purchase_receipt_status(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "NOT_RECEIVED" | "PART_RECEIVED" | "RECEIVED" => Some(value),
        "DRAFT" => Some("NOT_RECEIVED".to_string()),
        "CONFIRMED" => Some("RECEIVED".to_string()),
        _ => None,
    }
}

pub(crate) fn normalize_purchase_return_status(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    match value.as_str() {
        "DRAFT" | "DONE" => Some(value),
        _ => None,
    }
}
