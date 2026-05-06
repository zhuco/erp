use super::*;

pub(super) fn review_record_status(review_status: OrderReviewStatus) -> &'static str {
    match review_status {
        OrderReviewStatus::AutoApproved | OrderReviewStatus::Approved => {
            ORDER_REVIEW_RECORD_APPROVED
        }
        OrderReviewStatus::Rejected => ORDER_REVIEW_RECORD_REJECTED,
        OrderReviewStatus::NotReviewed
        | OrderReviewStatus::ManualReview
        | OrderReviewStatus::RolledBack => ORDER_REVIEW_RECORD_PENDING,
    }
}

pub(super) async fn handle_order_upserted(
    db: &Db,
    payload: OrderUpsertedPayload,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            order_id = %payload.order_id,
            "read-only mode active; skipping order review and after-sale sync"
        );
        return Ok(());
    }

    let sales_config =
        match load_sales_config_for_shop(db.pool(), payload.tenant_id, payload.shop_id).await {
            Ok(v) => v,
            Err(err) => {
                warn!(
                    tenant_id = %payload.tenant_id,
                    shop_id = %payload.shop_id,
                    error = %err,
                    "failed to load sales config; fallback to defaults"
                );
                SalesShopConfig::default()
            }
        };
    if sales_config.auto_to_review {
        handle_order_review(db, &payload).await?;
    } else {
        auto_pass_order_review(db, &payload).await?;
    }
    apply_default_order_tags(db, &payload).await?;
    if is_pdd_only_mode(db, payload.tenant_id).await? && payload.platform != Platform::Pdd {
        warn!(
            tenant_id = %payload.tenant_id,
            platform = %payload.platform,
            order_id = %payload.order_id,
            "pdd-only mode blocked after-sale sync"
        );
        return Ok(());
    }

    let aftersales_enabled = feature_flag_enabled(
        db,
        payload.tenant_id,
        Some(payload.shop_id),
        FeatureFlagKey::AftersalesWrite,
    )
    .await?;
    if !aftersales_enabled {
        warn!(
            tenant_id = %payload.tenant_id,
            shop_id = %payload.shop_id,
            order_id = %payload.order_id,
            "feature disabled: aftersales write"
        );
        return Ok(());
    }

    let cases_repo = AfterSaleCaseRepo::new(db.pool());
    cases_repo
        .sync_cases_for_order(
            payload.tenant_id,
            payload.platform,
            payload.shop_id,
            payload.order_id,
        )
        .await?;

    auto_allocate_order(db, payload.tenant_id, payload.shop_id, payload.order_id).await?;
    Ok(())
}

pub(super) async fn handle_order_reviewed(
    db: &Db,
    payload: OrderReviewedPayload,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            order_id = %payload.order_id,
            "read-only mode active; skipping order allocation"
        );
        return Ok(());
    }
    let reviewed = OrderReviewStatus::from_str(&payload.review_status)
        .map(|status| status.is_approved())
        .unwrap_or(false);
    if !reviewed {
        return Ok(());
    }
    auto_allocate_order(db, payload.tenant_id, payload.shop_id, payload.order_id).await
}

pub(super) async fn apply_default_order_tags(
    db: &Db,
    payload: &OrderUpsertedPayload,
) -> anyhow::Result<()> {
    let tag_enabled =
        feature_flag_enabled(db, payload.tenant_id, None, FeatureFlagKey::TagSystem).await?;
    if !tag_enabled {
        return Ok(());
    }

    let tags_repo = OrderTagDefinitionsRepo::new(db.pool());
    let definitions = tags_repo.list_auto_apply_tags(payload.tenant_id).await?;
    if definitions.is_empty() {
        return Ok(());
    }

    let orders_repo = OrdersRepo::new(db.pool());
    let Some(detail) = orders_repo
        .get_order_detail(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };

    let mut context = build_review_context(&detail);
    context.insert_field(
        "shop_id",
        serde_json::Value::String(payload.shop_id.to_string()),
    );
    context.insert_field(
        "platform_order_id",
        serde_json::Value::String(detail.order.platform_order_id.clone()),
    );
    if let Some(address) = detail.order.receiver_address.as_ref() {
        if let Some(province) = extract_province(address) {
            context.insert_field("address_province", serde_json::Value::String(province));
        }
        if address.trim().is_empty() {
            context.insert_field("address_invalid", serde_json::Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", serde_json::Value::Bool(true));
    }

    let existing_tags: std::collections::HashSet<String> =
        detail.tags.iter().map(|tag| tag.tag.clone()).collect();
    let mut matched_tags: Vec<String> = Vec::new();

    for definition in definitions {
        if existing_tags.contains(&definition.name) {
            continue;
        }
        let conditions: Vec<OrderReviewCondition> =
            match serde_json::from_value(definition.conditions_json.clone()) {
                Ok(value) => value,
                Err(err) => {
                    warn!(
                        tenant_id = %payload.tenant_id,
                        tag = %definition.name,
                        error = %err,
                        "invalid auto-apply conditions"
                    );
                    continue;
                }
            };
        let matched = if conditions.is_empty() {
            true
        } else {
            evaluate_conditions(&context, &conditions)
        };
        if matched {
            matched_tags.push(definition.name);
        }
    }

    if matched_tags.is_empty() {
        return Ok(());
    }

    let created_tags = matched_tags
        .iter()
        .map(|tag| OrderTagCreate {
            tag: tag.to_string(),
            created_by: None,
        })
        .collect::<Vec<_>>();
    let _ = orders_repo
        .add_order_tags(payload.tenant_id, payload.order_id, created_tags)
        .await?;

    let audit_repo = AuditLogsRepo::new(db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id: payload.tenant_id,
            user_id: None,
            action: "OMS.ORDER_TAG_AUTO".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(payload.order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "tags": matched_tags, "source": "AUTO" })),
            ip: None,
            ua: None,
        })
        .await;

    Ok(())
}

pub(super) async fn handle_order_review(
    db: &Db,
    payload: &OrderUpsertedPayload,
) -> anyhow::Result<()> {
    let orders_repo = OrdersRepo::new(db.pool());
    let rules_repo = OrderReviewRulesRepo::new(db.pool());
    let blacklist_repo = CustomerBlacklistRepo::new(db.pool());

    let Some(state) = orders_repo
        .get_order_review_state(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };
    if !state.review_status.is_pending() {
        return Ok(());
    }

    let Some(detail) = orders_repo
        .get_order_detail(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };

    let mut context = build_review_context(&detail);
    if let Some(address) = detail.order.receiver_address.as_ref() {
        if let Some(province) = extract_province(address) {
            context.insert_field("address_province", serde_json::Value::String(province));
        }
        if address.trim().is_empty() {
            context.insert_field("address_invalid", serde_json::Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", serde_json::Value::Bool(true));
    }

    let rules = rules_repo.list_enabled_rules(payload.tenant_id).await?;
    let mut hits: Vec<(uuid::Uuid, bool, String)> = Vec::new();
    let mut decision: Option<ReviewAction> = None;
    let mut extra_flags: Vec<String> = Vec::new();

    for rule in rules {
        let conditions: Vec<OrderReviewCondition> =
            serde_json::from_value(rule.conditions_json.clone()).unwrap_or_default();
        let matched = if conditions.is_empty() {
            false
        } else {
            evaluate_conditions(&context, &conditions)
        };
        let message = if matched {
            if rule.reason.is_empty() {
                "matched rule".to_string()
            } else {
                rule.reason.clone()
            }
        } else {
            String::new()
        };
        hits.push((rule.rule_id, matched, message.clone()));

        if matched {
            let action = match rule.action.to_uppercase().as_str() {
                "REJECT" => ReviewAction::Reject,
                "FLAG" => ReviewAction::Flag,
                _ => ReviewAction::Pass,
            };
            if !rule.reason.trim().is_empty() {
                extra_flags.push(rule.reason.clone());
            }
            match action {
                ReviewAction::Reject => {
                    decision = Some(ReviewAction::Reject);
                    break;
                }
                ReviewAction::Pass => {
                    decision = Some(ReviewAction::Pass);
                    break;
                }
                ReviewAction::Flag => {
                    decision = Some(ReviewAction::Flag);
                }
            }
        }
    }

    if has_mapping_missing(&detail) {
        extra_flags.push("MAPPING_MISSING".to_string());
        if !matches!(decision, Some(ReviewAction::Reject)) {
            decision = Some(ReviewAction::Flag);
        }
    }

    if has_invalid_address(&detail) {
        extra_flags.push("ADDRESS_INVALID".to_string());
        if !matches!(decision, Some(ReviewAction::Reject)) {
            decision = Some(ReviewAction::Flag);
        }
    }

    if let Some(phone) = detail.order.receiver_phone.as_ref() {
        if blacklist_repo
            .find_entry(payload.tenant_id, "phone", phone)
            .await?
            .is_some()
        {
            extra_flags.push("BLACKLISTED_CUSTOMER".to_string());
            if !matches!(decision, Some(ReviewAction::Reject)) {
                decision = Some(ReviewAction::Flag);
            }
        }
    }

    if is_duplicate_order(db.pool(), payload.tenant_id, &detail).await? {
        extra_flags.push("DUPLICATE_ORDER".to_string());
        if !matches!(decision, Some(ReviewAction::Reject)) {
            decision = Some(ReviewAction::Flag);
        }
    }

    let action = decision.unwrap_or(ReviewAction::Pass);
    let outcome = apply_review_decision(
        state.oms_status,
        action,
        &state.exception_flags,
        None,
        ReviewMode::Auto,
    )
    .map_err(|err| anyhow::anyhow!(err))?;
    let mut exception_flags = outcome.exception_flags;
    for flag in extra_flags {
        if !exception_flags.iter().any(|existing| existing == &flag) {
            exception_flags.push(flag);
        }
    }

    let exception_status = derive_exception_status(&exception_flags);
    let order_status = derive_order_status(
        outcome.oms_status,
        outcome.review_status,
        state.fulfillment_status,
        exception_status,
    );
    let updated = orders_repo
        .update_order_review_state(
            payload.tenant_id,
            payload.order_id,
            erp_repo::orders::OrderReviewUpdate {
                oms_status: outcome.oms_status,
                order_status,
                review_status: outcome.review_status,
                fulfillment_status: state.fulfillment_status,
                exception_status,
                exception_flags: exception_flags.clone(),
                expected_version: Some(state.version),
            },
        )
        .await?;

    if let Some(updated) = updated {
        let review_repo = OrderReviewsRepo::new(db.pool());
        let record_status = review_record_status(updated.review_status).to_string();
        let reviewed_at = if record_status == ORDER_REVIEW_RECORD_PENDING {
            None
        } else {
            Some(Utc::now())
        };
        let _ = review_repo
            .upsert_review(OrderReviewUpsert {
                tenant_id: payload.tenant_id,
                order_id: payload.order_id,
                status: record_status,
                reviewed_by: None,
                reviewed_at,
                notes: String::new(),
            })
            .await;

        orders_repo
            .upsert_review_hits(payload.tenant_id, payload.order_id, hits)
            .await?;
        let _ = orders_repo
            .upsert_exception_tickets(
                payload.tenant_id,
                payload.order_id,
                &updated.exception_flags,
            )
            .await;

        let audit_repo = AuditLogsRepo::new(db.pool());
        let action_label = match action {
            ReviewAction::Pass => "OMS.REVIEW_AUTO_PASS",
            ReviewAction::Reject => "OMS.REVIEW_AUTO_REJECT",
            ReviewAction::Flag => "OMS.REVIEW_AUTO_FLAG",
        };
        let _ = audit_repo
            .insert(AuditLogCreate {
                tenant_id: payload.tenant_id,
                user_id: None,
                action: action_label.to_string(),
                entity_type: "ORDER".to_string(),
                entity_id: Some(payload.order_id.to_string()),
                before_json: Some(serde_json::json!({
                    "oms_status": state.oms_status,
                    "review_status": state.review_status,
                    "exception_flags": state.exception_flags,
                })),
                after_json: Some(serde_json::json!({
                    "oms_status": updated.oms_status,
                    "review_status": updated.review_status,
                    "exception_flags": updated.exception_flags,
                })),
                ip: None,
                ua: None,
            })
            .await;

        let operation_action = match action {
            ReviewAction::Pass => OP_ACTION_OMS_REVIEW_AUTO_PASS,
            ReviewAction::Reject => OP_ACTION_OMS_REVIEW_AUTO_REJECT,
            ReviewAction::Flag => OP_ACTION_OMS_REVIEW_AUTO_FLAG,
        };
        let logs_repo = OperationLogsRepo::new(db.pool());
        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id: payload.tenant_id,
                actor_type: "SYSTEM".to_string(),
                actor_user_id: None,
                action: operation_action.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(payload.order_id.to_string()),
                metadata: Some(serde_json::json!({
                    "review_status": updated.review_status.to_string(),
                    "exception_flags": updated.exception_flags,
                })),
            })
            .await;

        let outbox_repo = OutboxRepo::new(db.pool());
        let out_payload = serde_json::json!({
            "tenant_id": payload.tenant_id,
            "order_id": payload.order_id,
            "review_status": outcome.review_status.to_string(),
            "oms_status": outcome.oms_status.to_string(),
            "exception_flags": exception_flags,
        });
        outbox_repo
            .enqueue(payload.tenant_id, "ORDER_REVIEWED", &out_payload)
            .await?;
    }

    Ok(())
}

pub(super) async fn auto_pass_order_review(
    db: &Db,
    payload: &OrderUpsertedPayload,
) -> anyhow::Result<()> {
    let orders_repo = OrdersRepo::new(db.pool());
    let Some(state) = orders_repo
        .get_order_review_state(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };
    if !state.review_status.is_pending() {
        return Ok(());
    }

    let outcome = apply_review_decision(
        state.oms_status,
        ReviewAction::Pass,
        &state.exception_flags,
        None,
        ReviewMode::Auto,
    )
    .map_err(|err| anyhow::anyhow!(err))?;
    let exception_status = derive_exception_status(&outcome.exception_flags);
    let order_status = derive_order_status(
        outcome.oms_status,
        outcome.review_status,
        state.fulfillment_status,
        exception_status,
    );
    let updated = orders_repo
        .update_order_review_state(
            payload.tenant_id,
            payload.order_id,
            erp_repo::orders::OrderReviewUpdate {
                oms_status: outcome.oms_status,
                order_status,
                review_status: outcome.review_status,
                fulfillment_status: state.fulfillment_status,
                exception_status,
                exception_flags: outcome.exception_flags.clone(),
                expected_version: Some(state.version),
            },
        )
        .await?;

    if let Some(updated) = updated {
        let review_repo = OrderReviewsRepo::new(db.pool());
        let record_status = review_record_status(updated.review_status).to_string();
        let reviewed_at = if record_status == ORDER_REVIEW_RECORD_PENDING {
            None
        } else {
            Some(Utc::now())
        };
        let _ = review_repo
            .upsert_review(OrderReviewUpsert {
                tenant_id: payload.tenant_id,
                order_id: payload.order_id,
                status: record_status,
                reviewed_by: None,
                reviewed_at,
                notes: String::new(),
            })
            .await;

        let _ = orders_repo
            .upsert_exception_tickets(
                payload.tenant_id,
                payload.order_id,
                &updated.exception_flags,
            )
            .await;
        let audit_repo = AuditLogsRepo::new(db.pool());
        let _ = audit_repo
            .insert(AuditLogCreate {
                tenant_id: payload.tenant_id,
                user_id: None,
                action: "OMS.REVIEW_AUTO_PASS".to_string(),
                entity_type: "ORDER".to_string(),
                entity_id: Some(payload.order_id.to_string()),
                before_json: Some(serde_json::json!({
                    "oms_status": state.oms_status,
                    "review_status": state.review_status,
                    "exception_flags": state.exception_flags,
                })),
                after_json: Some(serde_json::json!({
                    "oms_status": updated.oms_status,
                    "review_status": updated.review_status,
                    "exception_flags": updated.exception_flags,
                })),
                ip: None,
                ua: None,
            })
            .await;

        let logs_repo = OperationLogsRepo::new(db.pool());
        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id: payload.tenant_id,
                actor_type: "SYSTEM".to_string(),
                actor_user_id: None,
                action: OP_ACTION_OMS_REVIEW_AUTO_PASS.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(payload.order_id.to_string()),
                metadata: Some(serde_json::json!({
                    "review_status": updated.review_status.to_string(),
                    "exception_flags": updated.exception_flags,
                })),
            })
            .await;

        let outbox_repo = OutboxRepo::new(db.pool());
        let out_payload = serde_json::json!({
            "tenant_id": payload.tenant_id,
            "order_id": payload.order_id,
            "shop_id": payload.shop_id,
            "platform": payload.platform,
            "review_status": outcome.review_status.to_string(),
            "oms_status": outcome.oms_status.to_string(),
            "exception_flags": outcome.exception_flags,
        });
        outbox_repo
            .enqueue(payload.tenant_id, "ORDER_REVIEWED", &out_payload)
            .await?;
    }

    Ok(())
}

pub(super) fn build_review_context(
    detail: &erp_repo::orders::OmsOrderDetail,
) -> OrderReviewContext {
    let mut context = OrderReviewContext::new();
    context.insert_field(
        "total_cent",
        serde_json::Value::Number(detail.order.total_amount_cent.into()),
    );
    context.insert_field(
        "sku_count",
        serde_json::Value::Number(
            detail
                .items
                .iter()
                .map(|item| item.quantity as i64)
                .sum::<i64>()
                .into(),
        ),
    );
    context.insert_field(
        "sku_lines",
        serde_json::Value::Number((detail.items.len() as i64).into()),
    );
    if let Some(phone) = detail.order.receiver_phone.as_ref() {
        context.insert_field("receiver_phone", serde_json::Value::String(phone.clone()));
    }
    if let Some(name) = detail.order.receiver_name.as_ref() {
        context.insert_field("receiver_name", serde_json::Value::String(name.clone()));
    }
    context.insert_field(
        "oms_status",
        serde_json::Value::String(detail.order.oms_status.to_string()),
    );
    context.insert_field(
        "review_status",
        serde_json::Value::String(detail.order.review_status.to_string()),
    );
    context.insert_field(
        "platform",
        serde_json::Value::String(detail.order.platform.to_string()),
    );
    context
}

pub(super) fn extract_province(address: &str) -> Option<String> {
    const PROVINCES: [&str; 34] = [
        "北京",
        "天津",
        "河北",
        "山西",
        "内蒙古",
        "辽宁",
        "吉林",
        "黑龙江",
        "上海",
        "江苏",
        "浙江",
        "安徽",
        "福建",
        "江西",
        "山东",
        "河南",
        "湖北",
        "湖南",
        "广东",
        "广西",
        "海南",
        "重庆",
        "四川",
        "贵州",
        "云南",
        "西藏",
        "陕西",
        "甘肃",
        "青海",
        "宁夏",
        "新疆",
        "香港",
        "澳门",
        "台湾",
    ];
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return None;
    }
    for province in PROVINCES {
        if trimmed.contains(province) {
            return Some(province.to_string());
        }
    }
    None
}

pub(super) fn has_mapping_missing(detail: &erp_repo::orders::OmsOrderDetail) -> bool {
    detail.items.iter().any(|item| item.mapping_missing)
}

pub(super) fn has_invalid_address(detail: &erp_repo::orders::OmsOrderDetail) -> bool {
    detail
        .order
        .receiver_address
        .as_deref()
        .map(|value| value.trim().len() < 6)
        .unwrap_or(true)
}

pub(super) async fn is_duplicate_order(
    pool: &PgPool,
    tenant_id: TenantId,
    detail: &erp_repo::orders::OmsOrderDetail,
) -> anyhow::Result<bool> {
    let Some(phone) = detail.order.receiver_phone.as_ref() else {
        return Ok(false);
    };
    let count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from orders
        where tenant_id = $1
          and receiver_phone = $2
          and total_amount_cent = $3
          and order_id <> $4
          and order_created_at >= $5
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(phone)
    .bind(detail.order.total_amount_cent)
    .bind(uuid::Uuid::from(detail.order.id))
    .bind(detail.order.order_created_at - ChronoDuration::days(7))
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

pub(super) async fn auto_allocate_order(
    db: &Db,
    tenant_id: TenantId,
    shop_id: ShopId,
    order_id: OrderId,
) -> anyhow::Result<()> {
    let inventory_enabled =
        feature_flag_enabled(db, tenant_id, Some(shop_id), FeatureFlagKey::InventoryWrite).await?;
    if !inventory_enabled {
        warn!(
            tenant_id = %tenant_id,
            shop_id = %shop_id,
            order_id = %order_id,
            "feature disabled: inventory write"
        );
        return Ok(());
    }

    let orders_repo = OrdersRepo::new(db.pool());
    if let Some(order) = orders_repo
        .get_order_shipping_info(tenant_id, order_id)
        .await?
    {
        if order.status != erp_types::OrderStatus::PendingShipment
            && order.status != erp_types::OrderStatus::Allocated
        {
            return Ok(());
        }
    }

    let fulfillment_repo = FulfillmentRepo::new(db.pool());
    let result = fulfillment_repo
        .allocate_order(tenant_id, order_id, None)
        .await?;
    if !result.shortages.is_empty() {
        warn!(
            tenant_id = %tenant_id,
            order_id = %order_id,
            shortage_count = result.shortages.len(),
            "allocation shortages detected"
        );
    }
    Ok(())
}
