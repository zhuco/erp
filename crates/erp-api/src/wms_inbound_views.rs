use super::*;

pub(crate) fn inbound_doc_summary_response(
    summary: InboundDocSummary,
) -> InboundDocSummaryResponse {
    InboundDocSummaryResponse {
        doc_id: summary.doc_id,
        warehouse_id: summary.warehouse_id,
        doc_type: summary.doc_type,
        doc_no: summary.doc_no,
        status: summary.status,
        inbound_status: summary.inbound_status,
        receipt_status: summary.receipt_status,
        inbound_qc_status: summary.inbound_qc_status,
        putaway_status: summary.putaway_status,
        ref_type: summary.ref_type,
        ref_id: summary.ref_id,
        supplier_id: summary.supplier_id,
        supplier_name: summary.supplier_name,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
    }
}

pub(crate) fn inbound_doc_detail_response(detail: InboundDocDetail) -> InboundDocDetailResponse {
    InboundDocDetailResponse {
        doc: InboundDocResponse {
            doc_id: detail.doc.doc_id,
            warehouse_id: detail.doc.warehouse_id,
            doc_type: detail.doc.doc_type,
            doc_no: detail.doc.doc_no,
            status: detail.doc.status,
            inbound_status: detail.doc.inbound_status,
            receipt_status: detail.doc.receipt_status,
            inbound_qc_status: detail.doc.inbound_qc_status,
            putaway_status: detail.doc.putaway_status,
            ref_type: detail.doc.ref_type,
            ref_id: detail.doc.ref_id,
            created_at: detail.doc.created_at,
            updated_at: detail.doc.updated_at,
        },
        items: detail
            .items
            .into_iter()
            .map(|item| InboundDocItemResponse {
                sku_id: item.sku_id,
                sku_code: item.sku_code,
                barcode: item.barcode,
                planned_qty: item.planned_qty,
                received_qty: item.received_qty,
                putaway_qty: item.putaway_qty,
            })
            .collect(),
    }
}

pub(crate) fn inbound_reject_response(reject: InboundReject) -> InboundRejectResponse {
    InboundRejectResponse {
        reject_id: reject.reject_id,
        doc_id: reject.doc_id,
        doc_no: reject.doc_no,
        doc_type: reject.doc_type,
        warehouse_id: reject.warehouse_id,
        sku_id: reject.sku_id,
        sku_code: reject.sku_code,
        barcode: reject.barcode,
        qty: reject.qty,
        reason: reject.reason,
        operator_id: reject.operator_id.map(uuid::Uuid::from),
        created_at: reject.created_at,
    }
}
