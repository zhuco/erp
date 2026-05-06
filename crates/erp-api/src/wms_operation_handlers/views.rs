use super::*;

pub(crate) fn wms_outbound_doc_response(doc: OutboundDoc) -> WmsOutboundDocResponse {
    WmsOutboundDocResponse {
        outbound_doc_id: doc.outbound_doc_id,
        warehouse_id: doc.warehouse_id,
        doc_type: doc.doc_type,
        shipment_id: doc.shipment_id,
        order_id: doc.order_id,
        platform: doc.platform,
        platform_order_id: doc.platform_order_id,
        shop_id: doc.shop_id,
        receiver_name: doc.receiver_name,
        receiver_phone: doc.receiver_phone,
        receiver_address: doc.receiver_address,
        carrier_code: doc.carrier_code,
        tracking_no: doc.tracking_no,
        exception_flags: doc.exception_flags,
        tags: doc.tags,
        doc_no: doc.doc_no,
        status: doc.status,
        outbound_status: doc.outbound_status,
        pick_status: doc.pick_status,
        verify_status: doc.verify_status,
        pack_status: doc.pack_status,
        ship_status: doc.ship_status,
        picked_at: doc.picked_at,
        packed_at: doc.packed_at,
        shipped_at: doc.shipped_at,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }
}

pub(crate) fn wms_pick_task_response(task: OutboundPickTask) -> WmsPickTaskResponse {
    WmsPickTaskResponse {
        pick_task_id: task.pick_task_id,
        outbound_doc_id: task.outbound_doc_id,
        sku_id: task.sku_id,
        from_location_id: task.from_location_id,
        container_id: task.container_id,
        qty: task.qty,
        picked_qty: task.picked_qty,
        status: task.status,
        created_at: task.created_at,
        updated_at: task.updated_at,
    }
}

pub(crate) fn wms_pack_record_response(record: PackRecord) -> WmsPackRecordResponse {
    WmsPackRecordResponse {
        pack_record_id: record.pack_record_id,
        shipment_id: record.shipment_id,
        package_no: record.package_no,
        weight_gram: record.weight_gram,
        operator_id: record.operator_id,
        packed_at: record.packed_at,
        pack_status: record.pack_status,
        void_reason: record.void_reason,
        voided_at: record.voided_at,
    }
}

pub(crate) fn wms_outbound_detail_response(detail: OutboundDocDetail) -> WmsOutboundDetailResponse {
    WmsOutboundDetailResponse {
        doc: wms_outbound_doc_response(detail.doc),
        items: detail
            .items
            .into_iter()
            .map(|item| WmsOutboundItemResponse {
                sku_id: item.sku_id,
                planned_qty: item.planned_qty,
                picked_qty: item.picked_qty,
                packed_qty: item.packed_qty,
                batch_enabled: item.batch_enabled,
                serial_enabled: item.serial_enabled,
            })
            .collect(),
        pick_tasks: detail
            .pick_tasks
            .into_iter()
            .map(wms_pick_task_response)
            .collect(),
        pack_records: detail
            .pack_records
            .into_iter()
            .map(wms_pack_record_response)
            .collect(),
    }
}

pub(crate) fn wms_wave_response(wave: Wave) -> WmsWaveResponse {
    WmsWaveResponse {
        wave_id: wave.wave_id,
        warehouse_id: wave.warehouse_id,
        status: wave.status,
        created_at: wave.created_at,
        updated_at: wave.updated_at,
    }
}

pub(crate) fn wms_wave_rule_response(rule: WaveRule) -> WmsWaveRuleResponse {
    WmsWaveRuleResponse {
        rule_id: rule.rule_id,
        warehouse_id: rule.warehouse_id,
        carrier_code: rule.carrier_code,
        max_orders: rule.max_orders,
        max_qty: rule.max_qty,
        cutoff_time: rule
            .cutoff_time
            .map(|time| time.format("%H:%M:%S").to_string()),
        strategy: rule.strategy,
        enabled: rule.enabled,
        created_at: rule.created_at,
        updated_at: rule.updated_at,
    }
}
