use super::*;

pub(super) async fn run_wave_jobs_once(db: &Db) -> anyhow::Result<()> {
    let tenant_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select tenant_id
        from tenants
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    let warehouses_repo = WarehousesRepo::new(db.pool());
    let wms_repo = WmsRepo::new(db.pool());

    for tenant_uuid in tenant_ids {
        let tenant_id = TenantId(tenant_uuid);
        let warehouses = match warehouses_repo
            .list_warehouses(
                tenant_id,
                WarehouseListFilters {
                    keyword: None,
                    status: Some("ACTIVE".to_string()),
                    offset: 0,
                    limit: 200,
                },
            )
            .await
        {
            Ok(v) => v.items,
            Err(err) => {
                warn!(tenant_id = %tenant_id, error = %err, "wave auto run warehouses failed");
                continue;
            }
        };

        let require_audit =
            match fetch_tenant_setting_value(db.pool(), tenant_id, "inv.outbound_require_audit")
                .await
            {
                Ok(value) => value.as_ref().and_then(parse_bool_setting).unwrap_or(false),
                Err(err) => {
                    warn!(tenant_id = %tenant_id, error = %err, "wave auto run setting failed");
                    false
                }
            };
        let status = if require_audit { "AUDITED" } else { "DRAFT" };

        for warehouse in warehouses {
            let workflow_enabled = match fetch_warehouse_workflow_enabled(
                db.pool(),
                tenant_id,
                warehouse.warehouse_id,
            )
            .await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!(
                        tenant_id = %tenant_id,
                        warehouse_id = %warehouse.warehouse_id,
                        error = %err,
                        "wave workflow config load failed"
                    );
                    continue;
                }
            };
            if !workflow_enabled {
                continue;
            }

            let rules = match wms_repo
                .list_wave_rules(
                    tenant_id,
                    erp_repo::wms::WaveRuleListFilters {
                        warehouse_id: Some(warehouse.warehouse_id),
                        carrier_code: None,
                        enabled: Some(true),
                        offset: 0,
                        limit: 50,
                    },
                )
                .await
            {
                Ok(v) => v.items,
                Err(err) => {
                    warn!(
                        tenant_id = %tenant_id,
                        warehouse_id = %warehouse.warehouse_id,
                        error = %err,
                        "wave rule load failed"
                    );
                    continue;
                }
            };
            if rules.is_empty() {
                continue;
            }

            for rule in rules {
                let cutoff_at = resolve_wave_cutoff_at(rule.cutoff_time);
                let max_docs = rule.max_orders.unwrap_or(50).clamp(1, 500) as i64;
                let candidates = match wms_repo
                    .list_wave_candidates(
                        tenant_id,
                        warehouse.warehouse_id,
                        status,
                        cutoff_at,
                        rule.carrier_code.as_deref(),
                        max_docs,
                    )
                    .await
                {
                    Ok(v) => v,
                    Err(err) => {
                        warn!(
                            tenant_id = %tenant_id,
                            warehouse_id = %warehouse.warehouse_id,
                            error = %err,
                            "wave candidate load failed"
                        );
                        continue;
                    }
                };
                let selected_ids = select_wave_candidates(candidates, max_docs, rule.max_qty);
                if selected_ids.is_empty() {
                    continue;
                }

                let input = erp_repo::wms::WaveCreateInput {
                    tenant_id,
                    warehouse_id: warehouse.warehouse_id,
                    cutoff_at,
                    max_docs: selected_ids.len().max(1) as i64,
                    auto_pick: false,
                    outbound_doc_ids: Some(selected_ids),
                };
                let result = if require_audit {
                    wms_repo.create_wave_with_status(input, "AUDITED").await
                } else {
                    wms_repo.create_wave(input).await
                };
                if let Err(err) = result {
                    let message = err.to_string();
                    if message.contains("no outbound docs available") {
                        continue;
                    }
                    warn!(
                        tenant_id = %tenant_id,
                        warehouse_id = %warehouse.warehouse_id,
                        error = %err,
                        "wave auto run failed"
                    );
                }
            }
        }
    }

    Ok(())
}

pub(super) fn resolve_wave_cutoff_at(cutoff_time: Option<NaiveTime>) -> chrono::DateTime<Utc> {
    let now = Utc::now();
    let Some(time) = cutoff_time else {
        return now;
    };
    let today = now.date_naive();
    let naive_cutoff = chrono::NaiveDateTime::new(today, time);
    let mut cutoff = chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive_cutoff, Utc);
    if cutoff > now {
        cutoff -= ChronoDuration::days(1);
    }
    cutoff
}

pub(super) fn select_wave_candidates(
    candidates: Vec<erp_repo::wms::WaveCandidate>,
    max_docs: i64,
    max_qty: Option<i64>,
) -> Vec<uuid::Uuid> {
    let max_docs = max_docs.max(1) as usize;
    let mut selected = Vec::new();
    let mut total_qty: i64 = 0;
    for candidate in candidates {
        if selected.len() >= max_docs {
            break;
        }
        if let Some(limit) = max_qty {
            if !selected.is_empty() && total_qty + candidate.total_qty > limit {
                break;
            }
            if selected.is_empty() && candidate.total_qty > limit {
                selected.push(candidate.outbound_doc_id);
                break;
            }
            total_qty += candidate.total_qty;
        }
        selected.push(candidate.outbound_doc_id);
    }
    selected
}

pub(super) fn parse_bool_setting(value: &serde_json::Value) -> Option<bool> {
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

pub(super) fn parse_workflow_config_value(
    value: &serde_json::Value,
) -> Option<WarehouseWorkflowConfigLite> {
    if let Ok(config) = serde_json::from_value::<WarehouseWorkflowConfigLite>(value.clone()) {
        return Some(config);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(config) = serde_json::from_value::<WarehouseWorkflowConfigLite>(nested.clone()) {
            return Some(config);
        }
    }
    None
}

pub(super) async fn fetch_tenant_setting_value(
    pool: &PgPool,
    tenant_id: TenantId,
    key: &str,
) -> anyhow::Result<Option<serde_json::Value>> {
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

pub(super) async fn fetch_warehouse_setting_value(
    pool: &PgPool,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    key: &str,
) -> anyhow::Result<Option<serde_json::Value>> {
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

pub(super) async fn fetch_warehouse_workflow_enabled(
    pool: &PgPool,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
) -> anyhow::Result<bool> {
    let value =
        fetch_warehouse_setting_value(pool, tenant_id, warehouse_id, WMS_WORKFLOW_CONFIG_KEY)
            .await?;
    if let Some(value) = value {
        if let Some(config) = parse_workflow_config_value(&value) {
            return Ok(config.enable_wave.unwrap_or(false));
        }
    }
    Ok(false)
}
