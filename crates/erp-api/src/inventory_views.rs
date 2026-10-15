use super::*;

pub(crate) fn inventory_record_response(record: InventoryRecord) -> InventoryRecordResponse {
    let available = record.available();
    InventoryRecordResponse {
        warehouse_id: record.warehouse_id,
        sku_id: record.sku_id,
        sku_code: record.sku_code,
        barcode: record.barcode,
        cost_cent: record.cost_cent,
        on_hand: record.on_hand,
        reserved: record.reserved,
        frozen: record.frozen,
        in_transit: record.in_transit,
        available,
        updated_at: record.updated_at,
        quantities: Some(record.quantities),
        quantity_state: "precise",
    }
}

pub(crate) fn inventory_summary_response(
    record: InventorySummaryRecord,
) -> InventorySummaryResponse {
    let available = record.available();
    InventorySummaryResponse {
        sku_id: record.sku_id,
        sku_code: record.sku_code,
        barcode: record.barcode,
        on_hand: record.on_hand,
        reserved: record.reserved,
        frozen: record.frozen,
        in_transit: record.in_transit,
        available,
        updated_at: record.updated_at,
        quantities: record.quantities,
    }
}

pub(crate) fn inventory_cost_response(record: InventoryCostRecord) -> InventoryCostResponse {
    let available = record.available();
    InventoryCostResponse {
        sku_id: record.sku_id,
        sku_code: record.sku_code,
        barcode: record.barcode,
        on_hand: record.on_hand,
        reserved: record.reserved,
        frozen: record.frozen,
        in_transit: record.in_transit,
        available,
        unit_cost_cent: record.unit_cost_cent,
        total_cost_cent: record.total_cost_cent,
        estimated_value: record.estimated_value,
        valuation_source: "sku_master_estimate",
        updated_at: record.updated_at,
        quantities: record.quantities,
    }
}

pub(crate) fn inventory_ledger_response(record: InventoryLedgerRecord) -> InventoryLedgerResponse {
    InventoryLedgerResponse {
        ledger_id: record.ledger_id,
        warehouse_id: record.warehouse_id,
        sku_id: record.sku_id,
        sku_code: record.sku_code,
        delta_on_hand: record.delta_on_hand,
        delta_reserved: record.delta_reserved,
        delta_in_transit: record.delta_in_transit,
        delta_frozen: record.delta_frozen,
        reason_type: record.reason_type,
        ref_type: record.ref_type,
        ref_id: record.ref_id,
        occurred_at: record.occurred_at,
        quantities: record.quantities,
    }
}

pub(crate) fn inventory_balance_response(balance: InventoryBalance) -> InventoryRecordResponse {
    InventoryRecordResponse {
        warehouse_id: balance.warehouse_id,
        sku_id: balance.sku_id,
        sku_code: None,
        barcode: None,
        cost_cent: None,
        on_hand: balance.on_hand,
        reserved: balance.reserved,
        frozen: balance.frozen,
        in_transit: balance.in_transit,
        available: balance.available(),
        updated_at: balance.updated_at,
        quantities: Some(balance.quantities),
        quantity_state: "precise",
    }
}

pub(crate) fn inventory_atp_allocation_response(
    allocation: AtpAllocation,
) -> InventoryAtpAllocationResponse {
    InventoryAtpAllocationResponse {
        batch_id: allocation.batch_id,
        batch_no: allocation.batch_no,
        expire_at: allocation.expire_at,
        allocated_qty: allocation.allocated_qty,
        quantity: allocation.quantity,
    }
}

pub(crate) fn inventory_atp_response(quote: AtpQuote) -> InventoryAtpResponse {
    InventoryAtpResponse {
        warehouse_id: quote.warehouse_id,
        sku_id: quote.sku_id,
        on_hand: quote.on_hand,
        reserved: quote.reserved,
        frozen: quote.frozen,
        available: quote.available,
        request_qty: quote.request_qty,
        atp_qty: quote.atp_qty,
        shortage_qty: quote.shortage_qty,
        quantities: quote.quantities,
        allocations: quote
            .allocations
            .into_iter()
            .map(inventory_atp_allocation_response)
            .collect(),
    }
}

pub(crate) fn product_packaging_spec_response(
    spec: ProductPackagingSpec,
) -> ProductPackagingSpecResponse {
    ProductPackagingSpecResponse {
        spec_id: spec.spec_id,
        sku_id: spec.sku_id,
        level_code: spec.level_code,
        unit_name: spec.unit_name,
        units_per_level: spec.units_per_level,
        length_mm: spec.length_mm,
        width_mm: spec.width_mm,
        height_mm: spec.height_mm,
        weight_gram: spec.weight_gram,
        is_default: spec.is_default,
        created_at: spec.created_at,
        updated_at: spec.updated_at,
    }
}

pub(crate) fn inventory_reconciliation_response(
    record: InventoryReconciliationRecord,
) -> InventoryReconciliationResponse {
    InventoryReconciliationResponse {
        reconciliation_id: record.reconciliation_id,
        warehouse_id: record.warehouse_id,
        sku_id: record.sku_id,
        expected_on_hand: record.expected_on_hand,
        actual_on_hand: record.actual_on_hand,
        diff_qty: record.diff_qty,
        status: record.status,
        source_ref: record.source_ref,
        created_at: record.created_at,
    }
}

pub(crate) fn warehouse_area_response(area: WarehouseArea) -> WarehouseAreaResponse {
    WarehouseAreaResponse {
        area_id: area.area_id,
        warehouse_id: area.warehouse_id,
        code: area.code,
        name: area.name,
        area_type: area.area_type,
        status: area.status,
        priority: area.priority,
        created_at: area.created_at,
        updated_at: area.updated_at,
    }
}

pub(crate) fn warehouse_location_response(
    location: WarehouseLocation,
) -> WarehouseLocationResponse {
    WarehouseLocationResponse {
        location_id: location.location_id,
        warehouse_id: location.warehouse_id,
        area_id: location.area_id,
        code: location.code,
        name: location.name,
        barcode: location.barcode,
        volume: location.volume,
        max_weight: location.max_weight,
        is_picking: location.is_picking,
        is_disabled: location.is_disabled,
        created_at: location.created_at,
        updated_at: location.updated_at,
    }
}

pub(crate) fn location_balance_response(record: LocationBalanceRecord) -> LocationBalanceResponse {
    LocationBalanceResponse {
        warehouse_id: record.warehouse_id,
        location_id: record.location_id,
        location_code: record.location_code,
        sku_id: record.sku_id,
        sku_code: record.sku_code,
        on_hand: record.on_hand,
        updated_at: record.updated_at,
    }
}

pub(crate) fn container_response(container: Container) -> ContainerResponse {
    ContainerResponse {
        container_id: container.container_id,
        code: container.code,
        container_type: container.container_type,
        capacity: container.capacity,
        created_at: container.created_at,
        updated_at: container.updated_at,
    }
}

pub(crate) fn sender_address_response(sender: SenderAddress) -> SenderAddressResponse {
    SenderAddressResponse {
        sender_address_id: sender.sender_address_id,
        name: sender.name,
        phone: sender.phone,
        province: sender.province,
        city: sender.city,
        district: sender.district,
        address: sender.address,
        is_default: sender.is_default,
        created_at: sender.created_at,
        updated_at: sender.updated_at,
    }
}

pub(crate) fn workflow_config_response(
    warehouse_id: uuid::Uuid,
    config: WarehouseWorkflowConfigPayload,
) -> WarehouseWorkflowConfigResponse {
    WarehouseWorkflowConfigResponse {
        warehouse_id,
        enable_wave: config.enable_wave,
        require_inspection: config.require_inspection,
        require_weighing: config.require_weighing,
        require_pack: config.require_pack,
    }
}

pub(crate) fn stock_count_summary_response(count: StockCount) -> StockCountSummaryResponse {
    StockCountSummaryResponse {
        count_id: count.count_id,
        warehouse_id: count.warehouse_id,
        status: count.status,
        strategy: count.strategy,
        dimension: count.dimension,
        filters: stock_count_filters_response(&count.filters_json),
        created_at: count.created_at,
        updated_at: count.updated_at,
    }
}

pub(crate) fn stock_count_filters_response(filters_json: &Value) -> StockCountFiltersResponse {
    StockCountFiltersResponse {
        recent_days: filters_json
            .get("recent_days")
            .and_then(|value| value.as_i64()),
        brand: filters_json
            .get("brand")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        category: filters_json
            .get("category")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        scope_type: filters_json
            .get("scope_type")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        scope_ids: filters_json.get("scope_ids").and_then(|value| {
            value.as_array().map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(|v| v.to_string()))
                    .collect::<Vec<_>>()
            })
        }),
        count_mode: filters_json
            .get("count_mode")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        count_type: filters_json
            .get("count_type")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        freeze_stock: filters_json
            .get("freeze_stock")
            .and_then(|value| value.as_bool()),
        created_by: filters_json
            .get("created_by")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
    }
}

pub(crate) fn stock_count_item_response(item: StockCountItem) -> StockCountItemResponse {
    StockCountItemResponse {
        count_item_id: item.count_item_id,
        sku_id: item.sku_id,
        location_id: item.location_id,
        system_qty: item.system_qty,
        counted_qty: item.counted_qty,
        diff_qty: item.diff_qty,
        diff_reason: item.diff_reason,
    }
}

pub(crate) fn stock_count_detail_response(
    detail: StockCountDetail,
    audit_required: bool,
) -> StockCountDetailResponse {
    StockCountDetailResponse {
        count: stock_count_summary_response(detail.count),
        items: detail
            .items
            .into_iter()
            .map(stock_count_item_response)
            .collect(),
        audit_required,
    }
}

pub(crate) async fn ensure_default_stock_count_strategies(
    state: &AppState,
    tenant_id: TenantId,
) -> anyhow::Result<()> {
    let repo = StockCountStrategiesRepo::new(state.db.pool());
    let existing = repo
        .list_strategies(
            tenant_id,
            StockCountStrategyListFilters {
                enabled: None,
                offset: 0,
                limit: 1,
            },
        )
        .await?;
    if existing.total > 0 {
        return Ok(());
    }

    let defaults = vec![
        (
            "每周全仓盘点",
            "WEEKLY_FULL",
            serde_json::json!({ "weekday": 1 }),
        ),
        (
            "每日动销SKU盘点",
            "DAILY_ACTIVE_SKU",
            serde_json::json!({ "recent_days": 7 }),
        ),
        (
            "缺货/负库存盘点",
            "NEGATIVE_STOCK",
            serde_json::json!({ "include_zero": true }),
        ),
    ];

    for (name, strategy_type, config_json) in defaults {
        let _ = repo
            .create_strategy(StockCountStrategyCreate {
                tenant_id,
                name: name.to_string(),
                enabled: true,
                strategy_type: strategy_type.to_string(),
                config_json,
            })
            .await;
    }

    Ok(())
}

pub(crate) fn stock_transfer_summary_response(
    transfer: StockTransfer,
) -> StockTransferSummaryResponse {
    StockTransferSummaryResponse {
        transfer_id: transfer.transfer_id,
        from_warehouse_id: transfer.from_warehouse_id,
        to_warehouse_id: transfer.to_warehouse_id,
        status: transfer.status,
        created_at: transfer.created_at,
        updated_at: transfer.updated_at,
    }
}

pub(crate) fn stock_transfer_item_response(item: StockTransferItem) -> StockTransferItemResponse {
    StockTransferItemResponse {
        transfer_item_id: item.transfer_item_id,
        sku_id: item.sku_id,
        qty: item.qty,
    }
}

pub(crate) fn stock_transfer_detail_response(
    detail: StockTransferDetail,
) -> StockTransferDetailResponse {
    StockTransferDetailResponse {
        transfer: stock_transfer_summary_response(detail.transfer),
        items: detail
            .items
            .into_iter()
            .map(stock_transfer_item_response)
            .collect(),
    }
}

pub(crate) fn inventory_freeze_response(freeze: InventoryFreeze) -> InventoryFreezeResponse {
    InventoryFreezeResponse {
        freeze_id: freeze.freeze_id,
        warehouse_id: freeze.warehouse_id,
        sku_id: freeze.sku_id,
        qty: freeze.qty,
        reason: freeze.reason,
        status: freeze.status,
        created_at: freeze.created_at,
        updated_at: freeze.updated_at,
    }
}

pub(crate) fn stock_adjust_summary_response(doc: StockAdjustDoc) -> StockAdjustSummaryResponse {
    StockAdjustSummaryResponse {
        adjust_id: doc.adjust_id,
        warehouse_id: doc.warehouse_id,
        reason: doc.reason,
        status: doc.status,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }
}

pub(crate) fn stock_adjust_item_response(item: StockAdjustItem) -> StockAdjustItemResponse {
    StockAdjustItemResponse {
        adjust_item_id: item.adjust_item_id,
        sku_id: item.sku_id,
        qty: item.qty,
    }
}

pub(crate) fn stock_adjust_detail_response(detail: StockAdjustDetail) -> StockAdjustDetailResponse {
    StockAdjustDetailResponse {
        doc: stock_adjust_summary_response(detail.doc),
        items: detail
            .items
            .into_iter()
            .map(stock_adjust_item_response)
            .collect(),
    }
}

pub(crate) fn stock_move_summary_response(doc: StockMoveDoc) -> StockMoveSummaryResponse {
    StockMoveSummaryResponse {
        move_id: doc.move_id,
        warehouse_id: doc.warehouse_id,
        from_location_id: doc.from_location_id,
        to_location_id: doc.to_location_id,
        status: doc.status,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }
}

pub(crate) fn stock_move_item_response(item: StockMoveItem) -> StockMoveItemResponse {
    StockMoveItemResponse {
        move_item_id: item.move_item_id,
        sku_id: item.sku_id,
        qty: item.qty,
    }
}

pub(crate) fn stock_move_detail_response(detail: StockMoveDetail) -> StockMoveDetailResponse {
    StockMoveDetailResponse {
        doc: stock_move_summary_response(detail.doc),
        items: detail
            .items
            .into_iter()
            .map(stock_move_item_response)
            .collect(),
    }
}

pub(crate) fn stock_sync_job_response(job: StockSyncJob) -> StockSyncJobResponse {
    StockSyncJobResponse {
        job_id: job.job_id,
        shop_id: uuid::Uuid::from(job.shop_id),
        sku_id: job.sku_id,
        warehouse_id: job.warehouse_id,
        target_qty: job.target_qty,
        status: job.status,
        attempts: job.attempts,
        last_error: job.last_error,
        success_count: job.success_count,
        failure_count: job.failure_count,
        last_duration_ms: job.last_duration_ms,
        last_success_at: job.last_success_at,
        next_retry_at: job.next_retry_at,
        created_at: job.created_at,
        updated_at: job.updated_at,
    }
}

pub(crate) fn inventory_error_response(err: anyhow::Error) -> (StatusCode, Json<ErrorResponse>) {
    if let Some(batch) =
        err.downcast_ref::<erp_repo::inventory_batch_effects::InventoryBatchError>()
    {
        use erp_repo::inventory_batch_effects::InventoryBatchError;
        return match batch {
            InventoryBatchError::NotFound => error_response(
                StatusCode::NOT_FOUND,
                "inventory_batch_not_found",
                "batch not found",
            ),
            InventoryBatchError::IdentityRequired => error_response(
                StatusCode::CONFLICT,
                "inventory_batch_identity_required",
                "an incoming batch-managed SKU requires a batch UUID",
            ),
            InventoryBatchError::Unavailable => error_response(
                StatusCode::CONFLICT,
                "inventory_batch_unavailable",
                "batch is expired or has insufficient free exact stock",
            ),
        };
    }
    if let Some(cost) = err.downcast_ref::<erp_repo::fulfillment::FulfillmentCostError>() {
        use erp_repo::fulfillment::FulfillmentCostError;
        return match cost {
            FulfillmentCostError::Unavailable => error_response(
                StatusCode::CONFLICT,
                "fulfillment_cost_unavailable",
                "shipment SKU cost must be known before shipping",
            ),
            FulfillmentCostError::Invalid => error_response(
                StatusCode::CONFLICT,
                "fulfillment_cost_invalid",
                "shipment SKU cost must not be negative",
            ),
            FulfillmentCostError::Overflow => error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "fulfillment_cost_overflow",
                "shipment total cost exceeds the supported range",
            ),
            FulfillmentCostError::Conflict => error_response(
                StatusCode::CONFLICT,
                "fulfillment_cost_conflict",
                "shipment already has incompatible cost facts",
            ),
        };
    }
    if err
        .downcast_ref::<erp_repo::inventory::InventoryBatchLimit>()
        .is_some()
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "inventory_batch_limit",
            "interactive batch consumption supports at most 1000 batches per warehouse and SKU",
        );
    }
    if let Some(access) = err.downcast_ref::<erp_repo::fulfillment::FulfillmentAccessError>() {
        return match access {
            erp_repo::fulfillment::FulfillmentAccessError::Forbidden => error_response(
                StatusCode::FORBIDDEN,
                "fulfillment_access_denied",
                "current OMS actor or warehouse access denied",
            ),
            erp_repo::fulfillment::FulfillmentAccessError::ScopeLimit => error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "fulfillment_scope_limit",
                "OMS warehouse scope exceeds 10000 grants",
            ),
            erp_repo::fulfillment::FulfillmentAccessError::RolePermissionDenied => error_response(
                StatusCode::FORBIDDEN,
                "fulfillment_role_denied",
                "current role does not hold OMS order, shipping, and inventory write",
            ),
            erp_repo::fulfillment::FulfillmentAccessError::SessionInactive => error_response(
                StatusCode::UNAUTHORIZED,
                "fulfillment_session_inactive",
                "session is not active for this OMS command",
            ),
            erp_repo::fulfillment::FulfillmentAccessError::CredentialsStale => error_response(
                StatusCode::UNAUTHORIZED,
                "fulfillment_credentials_stale",
                "session credentials are stale for this OMS command",
            ),
        };
    }
    if let Some(read) = err.downcast_ref::<erp_repo::inventory_reads::InventoryReadError>() {
        return match read {
            erp_repo::inventory_reads::InventoryReadError::ReconciliationRequired(_) => {
                error_response(
                    StatusCode::CONFLICT,
                    "inventory_quantity_reconciliation_required",
                    "inventory history requires reconciliation",
                )
            }
            erp_repo::inventory_reads::InventoryReadError::ValueOverflow => error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "inventory_read_value_overflow",
                "inventory read exceeds the supported range",
            ),
        };
    }
    if err
        .downcast_ref::<erp_repo::atp_inventory::AtpQuoteError>()
        .is_some()
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "inventory_atp_batch_limit",
            "interactive ATP supports at most 1000 batches per warehouse and SKU",
        );
    }
    if let Some(precise) =
        err.downcast_ref::<erp_repo::inventory_precision::InventoryPrecisionError>()
    {
        use erp_repo::inventory_precision::InventoryPrecisionError;
        return match precise {
            InventoryPrecisionError::InsufficientAvailable { .. } => error_response(
                StatusCode::CONFLICT,
                "inventory_insufficient",
                "insufficient exact available inventory",
            ),
            InventoryPrecisionError::InvalidChange(_) => error_response(
                StatusCode::CONFLICT,
                "inventory_quantity_conflict",
                "inventory change exceeds the current quantity",
            ),
            InventoryPrecisionError::ReconciliationRequired(_) => error_response(
                StatusCode::CONFLICT,
                "inventory_reconciliation_required",
                "inventory history requires reconciliation",
            ),
            InventoryPrecisionError::OutsideEnterprise => error_response(
                StatusCode::FORBIDDEN,
                "inventory_scope_denied",
                "inventory warehouse or SKU is outside the enterprise",
            ),
        };
    }
    if err
        .downcast_ref::<erp_domain::quantity::QuantityError>()
        .is_some()
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "inventory_quantity_invalid",
            "inventory quantity is outside the supported precision or range",
        );
    }
    if let Some(inv) = err.downcast_ref::<InventoryError>() {
        return match inv {
            InventoryError::InsufficientAvailable { .. } => error_response(
                StatusCode::CONFLICT,
                "inventory_insufficient",
                "insufficient inventory",
            ),
            InventoryError::InsufficientFrozen { .. } => error_response(
                StatusCode::CONFLICT,
                "inventory_frozen_insufficient",
                "insufficient frozen inventory",
            ),
            InventoryError::ReservationNotFound => error_response(
                StatusCode::NOT_FOUND,
                "reservation_not_found",
                "reservation not found",
            ),
            InventoryError::ReservationNotActive => error_response(
                StatusCode::CONFLICT,
                "reservation_not_active",
                "reservation not active",
            ),
            InventoryError::ReservationMismatch => error_response(
                StatusCode::CONFLICT,
                "reservation_mismatch",
                "reservation mismatch",
            ),
            InventoryError::ReservationSourceUnverified => error_response(
                StatusCode::CONFLICT,
                "reservation_source_reconciliation_required",
                "reservation has no verified original root/component snapshot",
            ),
            InventoryError::ReservationComponentsLimit => error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "reservation_components_limit",
                "reservation must have 1 to 1000 valid components",
            ),
        };
    }

    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        &err.to_string(),
    )
}
