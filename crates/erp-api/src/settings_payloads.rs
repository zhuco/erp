use super::*;

pub(crate) fn default_wms_labor_tasks() -> Vec<WmsLaborTaskResponse> {
    let now = Utc::now();
    (0..12)
        .map(|index| WmsLaborTaskResponse {
            task_id: format!("LAB-{:04}", index + 1),
            warehouse_id: "WH-DEFAULT".to_string(),
            zone_code: format!("ZONE-{}", (index % 4) + 1),
            task_type: match index % 3 {
                0 => "PICK".to_string(),
                1 => "PUTAWAY".to_string(),
                _ => "COUNT".to_string(),
            },
            assignee: None,
            planned_qty: 20 + index * 2,
            finished_qty: 0,
            status: "UNASSIGNED".to_string(),
            updated_at: (now - Duration::minutes(index as i64 * 5)).to_rfc3339(),
        })
        .collect()
}

pub(crate) fn default_wms_slotting_suggestions() -> Vec<WmsSlottingSuggestionResponse> {
    let now = Utc::now();
    (0..8)
        .map(|index| WmsSlottingSuggestionResponse {
            suggestion_id: format!("SLOT-{:04}", index + 1),
            warehouse_id: "WH-DEFAULT".to_string(),
            sku_id: format!("SKU-{:04}", index + 1),
            from_location_id: format!("A-{}-01", index + 1),
            to_location_id: format!("B-{}-03", index + 1),
            reason: if index % 2 == 0 {
                "高频商品前移".to_string()
            } else {
                "减少跨区拣选路径".to_string()
            },
            priority: match index % 3 {
                0 => "HIGH".to_string(),
                1 => "MEDIUM".to_string(),
                _ => "LOW".to_string(),
            },
            status: "PENDING".to_string(),
            updated_at: (now - Duration::hours(index as i64)).to_rfc3339(),
        })
        .collect()
}

pub(crate) fn default_wms_replenishment_tasks() -> Vec<WmsReplenishmentTaskResponse> {
    let now = Utc::now();
    (0..10)
        .map(|index| WmsReplenishmentTaskResponse {
            task_id: format!("REP-{:04}", index + 1),
            warehouse_id: "WH-DEFAULT".to_string(),
            sku_id: format!("SKU-R-{:04}", index + 1),
            from_location_id: format!("BK-{}-01", (index % 5) + 1),
            to_location_id: format!("PK-{}-02", (index % 3) + 1),
            qty: 8 + index,
            assignee: None,
            status: "PENDING".to_string(),
            updated_at: (now - Duration::minutes(index as i64 * 3)).to_rfc3339(),
        })
        .collect()
}

pub(crate) fn default_wms_dock_appointments() -> Vec<WmsDockAppointmentResponse> {
    let now = Utc::now();
    (0..6)
        .map(|index| WmsDockAppointmentResponse {
            appointment_id: format!("DOCK-{:04}", index + 1),
            warehouse_id: "WH-DEFAULT".to_string(),
            dock_no: format!("D-{}", (index % 3) + 1),
            carrier_name: if index % 2 == 0 {
                "顺丰".to_string()
            } else {
                "中通".to_string()
            },
            vehicle_no: format!("沪A{}", 1000 + index),
            eta_at: (now + Duration::hours(index as i64 + 1)).to_rfc3339(),
            status: "SCHEDULED".to_string(),
            updated_at: (now - Duration::minutes(index as i64)).to_rfc3339(),
        })
        .collect()
}

pub(crate) fn parse_wms_labor_tasks_value(value: &Value) -> Option<Vec<WmsLaborTaskResponse>> {
    if let Ok(items) = serde_json::from_value::<Vec<WmsLaborTaskResponse>>(value.clone()) {
        return Some(items);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(items) = serde_json::from_value::<Vec<WmsLaborTaskResponse>>(nested.clone()) {
            return Some(items);
        }
    }
    None
}

pub(crate) fn parse_wms_slotting_suggestions_value(
    value: &Value,
) -> Option<Vec<WmsSlottingSuggestionResponse>> {
    if let Ok(items) = serde_json::from_value::<Vec<WmsSlottingSuggestionResponse>>(value.clone()) {
        return Some(items);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(items) =
            serde_json::from_value::<Vec<WmsSlottingSuggestionResponse>>(nested.clone())
        {
            return Some(items);
        }
    }
    None
}

pub(crate) fn parse_wms_replenishment_tasks_value(
    value: &Value,
) -> Option<Vec<WmsReplenishmentTaskResponse>> {
    if let Ok(items) = serde_json::from_value::<Vec<WmsReplenishmentTaskResponse>>(value.clone()) {
        return Some(items);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(items) =
            serde_json::from_value::<Vec<WmsReplenishmentTaskResponse>>(nested.clone())
        {
            return Some(items);
        }
    }
    None
}

pub(crate) fn parse_wms_dock_appointments_value(
    value: &Value,
) -> Option<Vec<WmsDockAppointmentResponse>> {
    if let Ok(items) = serde_json::from_value::<Vec<WmsDockAppointmentResponse>>(value.clone()) {
        return Some(items);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(items) = serde_json::from_value::<Vec<WmsDockAppointmentResponse>>(nested.clone())
        {
            return Some(items);
        }
    }
    None
}

pub(crate) fn default_system_workflow_catalog() -> SystemWorkflowCatalogPayload {
    let now = Utc::now();
    SystemWorkflowCatalogPayload {
        rules: vec![
            SystemNumberRule {
                rule_id: "RULE-PO-HQ".to_string(),
                doc_type: "采购订单".to_string(),
                org_scope: "总部".to_string(),
                prefix: "PO".to_string(),
                date_format: "YYYYMM".to_string(),
                sequence_length: 4,
                next_number: 128,
                status: "ACTIVE".to_string(),
                updated_at: (now - Duration::days(2)).to_rfc3339(),
            },
            SystemNumberRule {
                rule_id: "RULE-PO-EAST".to_string(),
                doc_type: "采购订单".to_string(),
                org_scope: "华东事业部".to_string(),
                prefix: "PO-E".to_string(),
                date_format: "YYYYMM".to_string(),
                sequence_length: 4,
                next_number: 56,
                status: "ACTIVE".to_string(),
                updated_at: (now - Duration::days(1)).to_rfc3339(),
            },
            SystemNumberRule {
                rule_id: "RULE-RTN".to_string(),
                doc_type: "售后单".to_string(),
                org_scope: "总部".to_string(),
                prefix: "AF".to_string(),
                date_format: "YYYYMMDD".to_string(),
                sequence_length: 3,
                next_number: 9,
                status: "DISABLED".to_string(),
                updated_at: (now - Duration::days(6)).to_rfc3339(),
            },
        ],
        workflows: vec![
            SystemWorkflowDefinition {
                workflow_id: "WF-PO-1".to_string(),
                doc_type: "采购订单".to_string(),
                name: "采购订单审批".to_string(),
                version: "v2.1".to_string(),
                status: "ACTIVE".to_string(),
                steps: vec![
                    "采购经理".to_string(),
                    "财务审核".to_string(),
                    "采购总监".to_string(),
                ],
                governance: Some(SystemWorkflowGovernance {
                    engine: "CAMUNDA8".to_string(),
                    process_key: "purchase_order_approval".to_string(),
                    definition_id: Some("purchase_order_approval:21:default".to_string()),
                    decision_key: Some("purchase_order_gate".to_string()),
                    publish_status: "PUBLISHED".to_string(),
                    approval_mode: "HUMAN_TASK".to_string(),
                    effective_at: Some((now - Duration::days(3)).to_rfc3339()),
                    last_deploy_at: Some((now - Duration::days(3)).to_rfc3339()),
                    last_deploy_result: Some("SUCCESS".to_string()),
                }),
                updated_at: (now - Duration::days(3)).to_rfc3339(),
            },
            SystemWorkflowDefinition {
                workflow_id: "WF-PO-2".to_string(),
                doc_type: "閲囪喘订单".to_string(),
                name: "閲囪喘订单瀹℃壒".to_string(),
                version: "v2.0".to_string(),
                status: "ARCHIVED".to_string(),
                steps: vec!["閲囪喘缁忕悊".to_string(), "璐㈠姟瀹℃牳".to_string()],
                governance: Some(SystemWorkflowGovernance {
                    engine: "CAMUNDA8".to_string(),
                    process_key: "purchase_order_approval".to_string(),
                    definition_id: Some("purchase_order_approval:20:archived".to_string()),
                    decision_key: Some("purchase_order_gate".to_string()),
                    publish_status: "ARCHIVED".to_string(),
                    approval_mode: "HUMAN_TASK".to_string(),
                    effective_at: Some((now - Duration::days(20)).to_rfc3339()),
                    last_deploy_at: Some((now - Duration::days(20)).to_rfc3339()),
                    last_deploy_result: Some("SUCCESS".to_string()),
                }),
                updated_at: (now - Duration::days(20)).to_rfc3339(),
            },
            SystemWorkflowDefinition {
                workflow_id: "WF-TR-1".to_string(),
                doc_type: "调拨单".to_string(),
                name: "调拨单流程".to_string(),
                version: "v1.3".to_string(),
                status: "DRAFT".to_string(),
                steps: vec!["仓储主管".to_string(), "物流经理".to_string()],
                governance: Some(SystemWorkflowGovernance {
                    engine: "CAMUNDA8".to_string(),
                    process_key: "transfer_order_approval".to_string(),
                    definition_id: None,
                    decision_key: Some("transfer_order_gate".to_string()),
                    publish_status: "DRAFT".to_string(),
                    approval_mode: "HUMAN_TASK".to_string(),
                    effective_at: None,
                    last_deploy_at: None,
                    last_deploy_result: Some("PENDING".to_string()),
                }),
                updated_at: (now - Duration::days(1)).to_rfc3339(),
            },
        ],
    }
}

pub(crate) fn parse_system_workflow_catalog_value(
    value: &Value,
) -> Option<SystemWorkflowCatalogPayload> {
    if let Ok(catalog) = serde_json::from_value::<SystemWorkflowCatalogPayload>(value.clone()) {
        return Some(catalog);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(catalog) = serde_json::from_value::<SystemWorkflowCatalogPayload>(nested.clone())
        {
            return Some(catalog);
        }
    }
    None
}

pub(crate) fn default_system_custom_fields_catalog() -> SystemCustomFieldsCatalogPayload {
    let now = Utc::now();
    SystemCustomFieldsCatalogPayload {
        fields: vec![
            SystemCustomField {
                field_id: "CF-ORDER-01".to_string(),
                entity: "订单".to_string(),
                label: "娓犻亾澶囨敞".to_string(),
                field_key: "channel_note".to_string(),
                field_type: "TEXT".to_string(),
                required: false,
                compatible: true,
                status: "ACTIVE".to_string(),
                description: Some("鍚屾娓犻亾订单澶囨敞".to_string()),
                updated_at: (now - Duration::days(2)).to_rfc3339(),
            },
            SystemCustomField {
                field_id: "CF-SKU-02".to_string(),
                entity: "鍟嗗搧".to_string(),
                label: "娓╂帶绛夌骇".to_string(),
                field_key: "temp_level".to_string(),
                field_type: "SELECT".to_string(),
                required: true,
                compatible: true,
                status: "ACTIVE".to_string(),
                description: Some("鐢熼矞鍟嗗搧娓╂帶瑕佹眰".to_string()),
                updated_at: (now - Duration::days(6)).to_rfc3339(),
            },
            SystemCustomField {
                field_id: "CF-PUR-03".to_string(),
                entity: "閲囪喘".to_string(),
                label: "棰勮鍒拌揣鍛ㄦ湡".to_string(),
                field_key: "lead_time".to_string(),
                field_type: "NUMBER".to_string(),
                required: false,
                compatible: false,
                status: "DISABLED".to_string(),
                description: Some("用于供应商响应分析".to_string()),
                updated_at: (now - Duration::days(10)).to_rfc3339(),
            },
        ],
        audits: vec![
            SystemCustomFieldAudit {
                record_id: "AUD-1001".to_string(),
                field_id: "CF-ORDER-01".to_string(),
                action: "创建字段".to_string(),
                operator: "系统管理员".to_string(),
                comment: Some("棣栨鍚敤娓犻亾澶囨敞".to_string()),
                created_at: (now - Duration::days(5)).to_rfc3339(),
            },
            SystemCustomFieldAudit {
                record_id: "AUD-1002".to_string(),
                field_id: "CF-SKU-02".to_string(),
                action: "更新瀛楁".to_string(),
                operator: "鍟嗗搧涓荤".to_string(),
                comment: Some("鏂板娓╂帶绛夌骇閫夐」".to_string()),
                created_at: (now - Duration::days(2)).to_rfc3339(),
            },
        ],
    }
}

pub(crate) fn parse_system_custom_fields_catalog_value(
    value: &Value,
) -> Option<SystemCustomFieldsCatalogPayload> {
    if let Ok(catalog) = serde_json::from_value::<SystemCustomFieldsCatalogPayload>(value.clone()) {
        return Some(catalog);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(catalog) =
            serde_json::from_value::<SystemCustomFieldsCatalogPayload>(nested.clone())
        {
            return Some(catalog);
        }
    }
    None
}

pub(crate) fn default_system_audit_reports_catalog() -> SystemAuditReportsCatalogPayload {
    let now = Utc::now();
    SystemAuditReportsCatalogPayload {
        reports: vec![
            SystemAuditReport {
                report_id: "REP-ORDER-01".to_string(),
                module: "订单".to_string(),
                period: "2025-12".to_string(),
                owner: "瀹¤涓撳憳".to_string(),
                anomalies: 3,
                status: "READY".to_string(),
                generated_at: (now - Duration::days(2)).to_rfc3339(),
            },
            SystemAuditReport {
                report_id: "REP-INV-02".to_string(),
                module: "搴撳瓨".to_string(),
                period: "2025-12".to_string(),
                owner: "系统管理员".to_string(),
                anomalies: 0,
                status: "READY".to_string(),
                generated_at: (now - Duration::days(1)).to_rfc3339(),
            },
            SystemAuditReport {
                report_id: "REP-RET-03".to_string(),
                module: "售后".to_string(),
                period: "2025-12".to_string(),
                owner: "运营主管".to_string(),
                anomalies: 5,
                status: "GENERATING".to_string(),
                generated_at: (now - Duration::hours(6)).to_rfc3339(),
            },
        ],
        rules: vec![
            SystemAuditRule {
                rule_id: "RULE-001".to_string(),
                module: "订单".to_string(),
                name: "异常订单金额".to_string(),
                condition: "订单金额 > 10 万元".to_string(),
                enabled: true,
                updated_at: (now - Duration::days(2)).to_rfc3339(),
            },
            SystemAuditRule {
                rule_id: "RULE-002".to_string(),
                module: "搴撳瓨".to_string(),
                name: "负库存告警".to_string(),
                condition: "搴撳瓨 < 0".to_string(),
                enabled: true,
                updated_at: (now - Duration::days(1)).to_rfc3339(),
            },
        ],
    }
}

pub(crate) fn default_system_integrations_catalog() -> SystemIntegrationsCatalogPayload {
    let now = Utc::now();
    SystemIntegrationsCatalogPayload {
        configs: vec![
            SystemIntegrationConfig {
                integration_id: "INT-EC-01".to_string(),
                name: "天猫旗舰店".to_string(),
                category: "电商平台".to_string(),
                provider: "Alibaba OpenAPI".to_string(),
                status: "CONNECTED".to_string(),
                success_rate: 98.7,
                retry_policy: "失败重试 3 次 / 15 分钟".to_string(),
                alert_channel: "企业微信-集成告警".to_string(),
                last_checked_at: Some((now - Duration::hours(2)).to_rfc3339()),
                updated_at: (now - Duration::days(1)).to_rfc3339(),
                description: Some("用于订单与库存同步".to_string()),
            },
            SystemIntegrationConfig {
                integration_id: "INT-PAY-02".to_string(),
                name: "微信支付".to_string(),
                category: "支付渠道".to_string(),
                provider: "WeChat Pay".to_string(),
                status: "WARNING".to_string(),
                success_rate: 92.4,
                retry_policy: "失败重试 5 次 / 10 分钟".to_string(),
                alert_channel: "钉钉-财务告警".to_string(),
                last_checked_at: Some((now - Duration::hours(6)).to_rfc3339()),
                updated_at: (now - Duration::days(2)).to_rfc3339(),
                description: Some("支付对账接口".to_string()),
            },
        ],
        changes: vec![
            SystemIntegrationChange {
                record_id: "CHG-1001".to_string(),
                integration_id: "INT-EC-01".to_string(),
                action: "更新重试策略".to_string(),
                operator: "系统管理员".to_string(),
                comment: Some("高峰期调整重试次数".to_string()),
                created_at: (now - Duration::days(3)).to_rfc3339(),
            },
            SystemIntegrationChange {
                record_id: "CHG-1002".to_string(),
                integration_id: "INT-PAY-02".to_string(),
                action: "告警渠道调整".to_string(),
                operator: "财务主管".to_string(),
                comment: Some("新增钉钉告警群".to_string()),
                created_at: (now - Duration::days(1)).to_rfc3339(),
            },
        ],
    }
}

pub(crate) fn parse_system_integrations_catalog_value(
    value: &Value,
) -> Option<SystemIntegrationsCatalogPayload> {
    if let Ok(catalog) = serde_json::from_value::<SystemIntegrationsCatalogPayload>(value.clone()) {
        return Some(catalog);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(catalog) =
            serde_json::from_value::<SystemIntegrationsCatalogPayload>(nested.clone())
        {
            return Some(catalog);
        }
    }
    None
}

pub(crate) fn parse_system_audit_reports_catalog_value(
    value: &Value,
) -> Option<SystemAuditReportsCatalogPayload> {
    if let Ok(catalog) = serde_json::from_value::<SystemAuditReportsCatalogPayload>(value.clone()) {
        return Some(catalog);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(catalog) =
            serde_json::from_value::<SystemAuditReportsCatalogPayload>(nested.clone())
        {
            return Some(catalog);
        }
    }
    None
}

pub(crate) async fn fetch_warehouse_setting_value(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    key: &str,
) -> anyhow::Result<Option<Value>> {
    let repo = SettingsRepo::new(pool);
    let settings = repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("WAREHOUSE".to_string()),
                scope_id: Some(warehouse_id),
                key: Some(key.to_string()),
            },
        )
        .await?;
    Ok(settings
        .into_iter()
        .next()
        .map(|setting| setting.value_json))
}

pub(crate) async fn fetch_warehouse_workflow_config(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
) -> anyhow::Result<WarehouseWorkflowConfigPayload> {
    let value =
        fetch_warehouse_setting_value(pool, tenant_id, warehouse_id, WMS_WORKFLOW_CONFIG_KEY)
            .await?;
    if let Some(value) = value {
        if let Some(config) = parse_workflow_config_value(&value) {
            return Ok(config);
        }
    }
    Ok(default_warehouse_workflow_config())
}

pub(crate) async fn fetch_tenant_setting_value(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    key: &str,
) -> anyhow::Result<Option<Value>> {
    let repo = SettingsRepo::new(pool);
    let settings = repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some(key.to_string()),
            },
        )
        .await?;
    Ok(settings
        .into_iter()
        .next()
        .map(|setting| setting.value_json))
}

pub(crate) async fn save_tenant_setting_value(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    key: &str,
    value: Value,
) -> anyhow::Result<()> {
    let repo = SettingsRepo::new(pool);
    repo.upsert_setting(SettingUpsert {
        tenant_id,
        scope_type: "TENANT".to_string(),
        scope_id: None,
        key: key.to_string(),
        value_json: value,
        expected_version: None,
    })
    .await?;
    Ok(())
}

pub(crate) async fn load_wms_labor_tasks(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<Vec<WmsLaborTaskResponse>> {
    let value = fetch_tenant_setting_value(pool, tenant_id, WMS_LABOR_TASKS_KEY).await?;
    if let Some(value) = value {
        if let Some(items) = parse_wms_labor_tasks_value(&value) {
            return Ok(items);
        }
    }
    Ok(default_wms_labor_tasks())
}

pub(crate) async fn save_wms_labor_tasks(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    items: &[WmsLaborTaskResponse],
) -> anyhow::Result<()> {
    let value = serde_json::to_value(items)?;
    save_tenant_setting_value(pool, tenant_id, WMS_LABOR_TASKS_KEY, value).await
}

pub(crate) async fn load_wms_slotting_suggestions(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<Vec<WmsSlottingSuggestionResponse>> {
    let value = fetch_tenant_setting_value(pool, tenant_id, WMS_SLOTTING_SUGGESTIONS_KEY).await?;
    if let Some(value) = value {
        if let Some(items) = parse_wms_slotting_suggestions_value(&value) {
            return Ok(items);
        }
    }
    Ok(default_wms_slotting_suggestions())
}

pub(crate) async fn save_wms_slotting_suggestions(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    items: &[WmsSlottingSuggestionResponse],
) -> anyhow::Result<()> {
    let value = serde_json::to_value(items)?;
    save_tenant_setting_value(pool, tenant_id, WMS_SLOTTING_SUGGESTIONS_KEY, value).await
}

pub(crate) async fn load_wms_replenishment_tasks(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<Vec<WmsReplenishmentTaskResponse>> {
    let value = fetch_tenant_setting_value(pool, tenant_id, WMS_REPLENISHMENT_TASKS_KEY).await?;
    if let Some(value) = value {
        if let Some(items) = parse_wms_replenishment_tasks_value(&value) {
            return Ok(items);
        }
    }
    Ok(default_wms_replenishment_tasks())
}

pub(crate) async fn save_wms_replenishment_tasks(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    items: &[WmsReplenishmentTaskResponse],
) -> anyhow::Result<()> {
    let value = serde_json::to_value(items)?;
    save_tenant_setting_value(pool, tenant_id, WMS_REPLENISHMENT_TASKS_KEY, value).await
}

pub(crate) async fn load_wms_dock_appointments(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<Vec<WmsDockAppointmentResponse>> {
    let value = fetch_tenant_setting_value(pool, tenant_id, WMS_DOCK_APPOINTMENTS_KEY).await?;
    if let Some(value) = value {
        if let Some(items) = parse_wms_dock_appointments_value(&value) {
            return Ok(items);
        }
    }
    Ok(default_wms_dock_appointments())
}

pub(crate) async fn save_wms_dock_appointments(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    items: &[WmsDockAppointmentResponse],
) -> anyhow::Result<()> {
    let value = serde_json::to_value(items)?;
    save_tenant_setting_value(pool, tenant_id, WMS_DOCK_APPOINTMENTS_KEY, value).await
}

pub(crate) async fn load_system_workflow_catalog(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<SystemWorkflowCatalogPayload> {
    let value = fetch_tenant_setting_value(pool, tenant_id, SYSTEM_WORKFLOW_CATALOG_KEY).await?;
    if let Some(value) = value {
        if let Some(catalog) = parse_system_workflow_catalog_value(&value) {
            return Ok(catalog);
        }
    }
    Ok(default_system_workflow_catalog())
}

pub(crate) async fn load_system_custom_fields_catalog(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<SystemCustomFieldsCatalogPayload> {
    let value =
        fetch_tenant_setting_value(pool, tenant_id, SYSTEM_CUSTOM_FIELDS_CATALOG_KEY).await?;
    if let Some(value) = value {
        if let Some(catalog) = parse_system_custom_fields_catalog_value(&value) {
            return Ok(catalog);
        }
    }
    Ok(default_system_custom_fields_catalog())
}

pub(crate) async fn load_system_integrations_catalog(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<SystemIntegrationsCatalogPayload> {
    let value =
        fetch_tenant_setting_value(pool, tenant_id, SYSTEM_INTEGRATIONS_CATALOG_KEY).await?;
    if let Some(value) = value {
        if let Some(catalog) = parse_system_integrations_catalog_value(&value) {
            return Ok(catalog);
        }
    }
    Ok(default_system_integrations_catalog())
}

pub(crate) async fn load_system_audit_reports_catalog(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<SystemAuditReportsCatalogPayload> {
    let value =
        fetch_tenant_setting_value(pool, tenant_id, SYSTEM_AUDIT_REPORTS_CATALOG_KEY).await?;
    if let Some(value) = value {
        if let Some(catalog) = parse_system_audit_reports_catalog_value(&value) {
            return Ok(catalog);
        }
    }
    Ok(default_system_audit_reports_catalog())
}

pub(crate) async fn fetch_stock_count_audit_required(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<bool> {
    let value = fetch_tenant_setting_value(pool, tenant_id, "wms.count_require_audit").await?;
    Ok(value.as_ref().and_then(parse_bool_setting).unwrap_or(false))
}

pub(crate) fn parse_numeric_setting(value: &Value) -> Option<i64> {
    if let Some(result) = value.as_i64() {
        return Some(result);
    }
    if let Some(obj) = value.as_object() {
        if let Some(result) = obj.get("value").and_then(|item| item.as_i64()) {
            return Some(result);
        }
    }
    None
}

pub(crate) fn parse_bool_setting(value: &Value) -> Option<bool> {
    if let Some(result) = value.as_bool() {
        return Some(result);
    }
    if let Some(obj) = value.as_object() {
        if let Some(result) = obj.get("value").and_then(|item| item.as_bool()) {
            return Some(result);
        }
    }
    None
}

pub(crate) fn parse_string_setting(value: &Value) -> Option<String> {
    if let Some(result) = value.as_str() {
        let trimmed = result.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(obj) = value.as_object() {
        if let Some(result) = obj.get("value").and_then(|item| item.as_str()) {
            let trimmed = result.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub(crate) async fn load_wms_inbound_settings(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<WmsInboundSettingsResponse> {
    let require_audit = fetch_tenant_setting_value(pool, tenant_id, "inv.inbound_require_audit")
        .await?
        .as_ref()
        .and_then(parse_bool_setting)
        .unwrap_or(false);
    let require_inspection =
        fetch_tenant_setting_value(pool, tenant_id, "wms.inbound_require_inspection")
            .await?
            .as_ref()
            .and_then(parse_bool_setting)
            .unwrap_or(false);
    let require_putaway_location =
        fetch_tenant_setting_value(pool, tenant_id, "wms.inbound_require_location")
            .await?
            .as_ref()
            .and_then(parse_bool_setting)
            .unwrap_or(false);
    let enable_return_inbound_link =
        fetch_tenant_setting_value(pool, tenant_id, "service.enable_return_inbound_link")
            .await?
            .as_ref()
            .and_then(parse_bool_setting)
            .unwrap_or(false);

    Ok(WmsInboundSettingsResponse {
        require_audit,
        require_inspection,
        require_putaway_location,
        enable_return_inbound_link,
    })
}
