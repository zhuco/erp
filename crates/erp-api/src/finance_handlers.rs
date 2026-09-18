use super::*;

pub(crate) async fn import_statements(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(csv_query): Query<StatementImportCsvQuery>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json");
    let is_csv = content_type.contains("text/csv") || content_type.contains("application/csv");

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if is_csv {
        let statement_type = match csv_query.statement_type {
            Some(v) => v,
            None => return bad_request("missing statement_type").into_response(),
        };
        match statement_type {
            StatementImportType::Platform => {
                let platform = match csv_query.platform.as_deref() {
                    Some(raw) => match Platform::from_str(raw) {
                        Ok(v) => v,
                        Err(_) => return bad_request("bad platform").into_response(),
                    },
                    None => return bad_request("missing platform").into_response(),
                };
                let shop_id = match csv_query.shop_id.as_deref() {
                    Some(raw) => match ShopId::from_str(raw) {
                        Ok(v) => v,
                        Err(_) => return bad_request("bad shop_id").into_response(),
                    },
                    None => return bad_request("missing shop_id").into_response(),
                };
                let period_start = match csv_query.period_start {
                    Some(v) => v,
                    None => return bad_request("missing period_start").into_response(),
                };
                let period_end = match csv_query.period_end {
                    Some(v) => v,
                    None => return bad_request("missing period_end").into_response(),
                };
                let lines = match parse_platform_lines_csv(&body) {
                    Ok(v) => v,
                    Err(err) => return bad_request(&err.to_string()).into_response(),
                };
                let repo = PlatformStatementsRepo::new(state.db.pool());
                let result = match repo
                    .upsert_statement_with_lines(
                        PlatformStatementCreate {
                            tenant_id,
                            platform,
                            shop_id,
                            period_start,
                            period_end,
                            status: PLATFORM_STATEMENT_STATUS_IMPORTED.to_string(),
                        },
                        &lines,
                    )
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                    }
                };
                let _ = audit_repo
                    .insert(AuditLogCreate {
                        tenant_id,
                        user_id: Some(ctx.user_id),
                        action: "FINANCE.PLATFORM_STATEMENT_IMPORTED".to_string(),
                        entity_type: "platform_statement".to_string(),
                        entity_id: Some(result.statement.statement_id.to_string()),
                        before_json: None,
                        after_json: Some(serde_json::json!({
                            "inserted_lines": result.inserted_lines,
                            "platform": result.statement.platform.to_string(),
                            "shop_id": result.statement.shop_id.to_string(),
                        })),
                        ip: None,
                        ua: None,
                    })
                    .await;

                return (
                    StatusCode::OK,
                    Json(StatementImportResponse {
                        statement_type: statement_type.as_str().to_string(),
                        statement_id: Some(result.statement.statement_id),
                        inserted_lines: result.inserted_lines,
                    }),
                )
                    .into_response();
            }
            StatementImportType::Shipping => {
                let lines = match parse_shipping_lines_csv(&body) {
                    Ok(v) => v,
                    Err(err) => return bad_request(&err.to_string()).into_response(),
                };
                let repo = ShippingStatementsRepo::new(state.db.pool());
                let inserted = match repo.insert_lines(tenant_id, &lines).await {
                    Ok(v) => v,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                    }
                };
                let _ = audit_repo
                    .insert(AuditLogCreate {
                        tenant_id,
                        user_id: Some(ctx.user_id),
                        action: "FINANCE.SHIPPING_STATEMENT_IMPORTED".to_string(),
                        entity_type: "shipping_statement_lines".to_string(),
                        entity_id: None,
                        before_json: None,
                        after_json: Some(serde_json::json!({
                            "inserted_lines": inserted,
                        })),
                        ip: None,
                        ua: None,
                    })
                    .await;

                return (
                    StatusCode::OK,
                    Json(StatementImportResponse {
                        statement_type: statement_type.as_str().to_string(),
                        statement_id: None,
                        inserted_lines: inserted,
                    }),
                )
                    .into_response();
            }
            StatementImportType::Payment => {
                let channel = match csv_query.channel.as_deref().and_then(non_empty) {
                    Some(value) => value,
                    None => return bad_request("missing channel").into_response(),
                };
                let lines = match parse_payment_lines_csv(&body) {
                    Ok(v) => v,
                    Err(err) => return bad_request(&err.to_string()).into_response(),
                };
                let repo = PaymentStatementsRepo::new(state.db.pool());
                let result = match repo
                    .upsert_statement_with_lines(
                        PaymentStatementCreate {
                            tenant_id,
                            channel,
                            period_start: csv_query.period_start,
                            period_end: csv_query.period_end,
                            status: PAYMENT_STATEMENT_STATUS_IMPORTED.to_string(),
                        },
                        &lines,
                    )
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                    }
                };
                let _ = audit_repo
                    .insert(AuditLogCreate {
                        tenant_id,
                        user_id: Some(ctx.user_id),
                        action: "FINANCE.PAYMENT_STATEMENT_IMPORTED".to_string(),
                        entity_type: "payment_statement".to_string(),
                        entity_id: Some(result.statement.statement_id.to_string()),
                        before_json: None,
                        after_json: Some(serde_json::json!({
                            "inserted_lines": result.inserted_lines,
                            "channel": result.statement.channel,
                        })),
                        ip: None,
                        ua: None,
                    })
                    .await;

                return (
                    StatusCode::OK,
                    Json(StatementImportResponse {
                        statement_type: statement_type.as_str().to_string(),
                        statement_id: Some(result.statement.statement_id),
                        inserted_lines: result.inserted_lines,
                    }),
                )
                    .into_response();
            }
        }
    }

    let req: StatementImportRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(err) => return bad_request(&format!("invalid json: {err}")).into_response(),
    };

    match req.statement_type {
        StatementImportType::Platform => {
            let platform = match req.platform.as_deref() {
                Some(raw) => match Platform::from_str(raw) {
                    Ok(v) => v,
                    Err(_) => return bad_request("bad platform").into_response(),
                },
                None => return bad_request("missing platform").into_response(),
            };
            let shop_id = match req.shop_id.as_deref() {
                Some(raw) => match ShopId::from_str(raw) {
                    Ok(v) => v,
                    Err(_) => return bad_request("bad shop_id").into_response(),
                },
                None => return bad_request("missing shop_id").into_response(),
            };
            let period_start = match req.period_start {
                Some(v) => v,
                None => return bad_request("missing period_start").into_response(),
            };
            let period_end = match req.period_end {
                Some(v) => v,
                None => return bad_request("missing period_end").into_response(),
            };
            let lines = req
                .lines
                .unwrap_or_default()
                .into_iter()
                .map(|line| PlatformStatementLineInput {
                    platform_order_id: line.platform_order_id,
                    gross_cent: line.gross_cent.unwrap_or(0),
                    fee_cent: line.fee_cent.unwrap_or(0),
                    refund_cent: line.refund_cent.unwrap_or(0),
                    net_cent: line.net_cent.unwrap_or(0),
                    occurred_at: line.occurred_at,
                    raw_json: line.raw_json.unwrap_or_else(|| serde_json::json!({})),
                })
                .collect::<Vec<_>>();
            let repo = PlatformStatementsRepo::new(state.db.pool());
            let result = match repo
                .upsert_statement_with_lines(
                    PlatformStatementCreate {
                        tenant_id,
                        platform,
                        shop_id,
                        period_start,
                        period_end,
                        status: PLATFORM_STATEMENT_STATUS_IMPORTED.to_string(),
                    },
                    &lines,
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                }
            };
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "FINANCE.PLATFORM_STATEMENT_IMPORTED".to_string(),
                    entity_type: "platform_statement".to_string(),
                    entity_id: Some(result.statement.statement_id.to_string()),
                    before_json: None,
                    after_json: Some(serde_json::json!({
                        "inserted_lines": result.inserted_lines,
                        "platform": result.statement.platform.to_string(),
                        "shop_id": result.statement.shop_id.to_string(),
                    })),
                    ip: None,
                    ua: None,
                })
                .await;

            (
                StatusCode::OK,
                Json(StatementImportResponse {
                    statement_type: req.statement_type.as_str().to_string(),
                    statement_id: Some(result.statement.statement_id),
                    inserted_lines: result.inserted_lines,
                }),
            )
                .into_response()
        }
        StatementImportType::Shipping => {
            let lines = req
                .shipping_lines
                .unwrap_or_default()
                .into_iter()
                .map(|line| ShippingStatementLineInput {
                    tracking_no: line.tracking_no,
                    fee_cent: line.fee_cent.unwrap_or(0),
                    weight: line.weight,
                    occurred_at: line.occurred_at,
                    raw_json: line.raw_json.unwrap_or_else(|| serde_json::json!({})),
                })
                .collect::<Vec<_>>();
            let repo = ShippingStatementsRepo::new(state.db.pool());
            let inserted = match repo.insert_lines(tenant_id, &lines).await {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                }
            };
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "FINANCE.SHIPPING_STATEMENT_IMPORTED".to_string(),
                    entity_type: "shipping_statement_lines".to_string(),
                    entity_id: None,
                    before_json: None,
                    after_json: Some(serde_json::json!({
                        "inserted_lines": inserted,
                    })),
                    ip: None,
                    ua: None,
                })
                .await;

            (
                StatusCode::OK,
                Json(StatementImportResponse {
                    statement_type: req.statement_type.as_str().to_string(),
                    statement_id: None,
                    inserted_lines: inserted,
                }),
            )
                .into_response()
        }
        StatementImportType::Payment => {
            let channel = match req.channel.as_deref().and_then(non_empty) {
                Some(value) => value,
                None => return bad_request("missing channel").into_response(),
            };
            let lines = req
                .payment_lines
                .unwrap_or_default()
                .into_iter()
                .map(|line| PaymentStatementLineInput {
                    platform_order_id: line.platform_order_id,
                    paid_cent: line.paid_cent.unwrap_or(0),
                    fee_cent: line.fee_cent.unwrap_or(0),
                    net_cent: line.net_cent.unwrap_or(0),
                    occurred_at: line.occurred_at,
                    raw_json: line.raw_json.unwrap_or_else(|| serde_json::json!({})),
                })
                .collect::<Vec<_>>();
            let repo = PaymentStatementsRepo::new(state.db.pool());
            let result = match repo
                .upsert_statement_with_lines(
                    PaymentStatementCreate {
                        tenant_id,
                        channel,
                        period_start: req.period_start,
                        period_end: req.period_end,
                        status: PAYMENT_STATEMENT_STATUS_IMPORTED.to_string(),
                    },
                    &lines,
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                }
            };
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "FINANCE.PAYMENT_STATEMENT_IMPORTED".to_string(),
                    entity_type: "payment_statement".to_string(),
                    entity_id: Some(result.statement.statement_id.to_string()),
                    before_json: None,
                    after_json: Some(serde_json::json!({
                        "inserted_lines": result.inserted_lines,
                        "channel": result.statement.channel,
                    })),
                    ip: None,
                    ua: None,
                })
                .await;

            (
                StatusCode::OK,
                Json(StatementImportResponse {
                    statement_type: req.statement_type.as_str().to_string(),
                    statement_id: Some(result.statement.statement_id),
                    inserted_lines: result.inserted_lines,
                }),
            )
                .into_response()
        }
    }
}

pub(crate) async fn run_reconcile(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReconcileRunRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let job_type = req.job_type.trim().to_uppercase();
    if job_type != RECONCILE_JOB_TYPE_PLATFORM && job_type != RECONCILE_JOB_TYPE_SHIPPING {
        return bad_request("unsupported job_type").into_response();
    }

    let reconcile_repo = ReconcileJobsRepo::new(state.db.pool());
    let job = match reconcile_repo
        .create_job(ReconcileJobCreate {
            tenant_id,
            job_type: job_type.clone(),
            status: RECONCILE_JOB_STATUS_RUNNING.to_string(),
            started_at: Some(Utc::now()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let result = if job_type == RECONCILE_JOB_TYPE_PLATFORM {
        let statement_id = match req.statement_id.as_deref() {
            Some(raw) => match uuid::Uuid::parse_str(raw) {
                Ok(v) => v,
                Err(_) => return bad_request("bad statement_id").into_response(),
            },
            None => return bad_request("missing statement_id").into_response(),
        };
        let statement_repo = PlatformStatementsRepo::new(state.db.pool());
        match statement_repo.get_statement(tenant_id, statement_id).await {
            Ok(Some(_)) => {}
            Ok(None) => return (StatusCode::NOT_FOUND, "statement not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        reconcile_platform_statement(&state, tenant_id, job.job_id, statement_id).await
    } else {
        if req.occurred_from.is_none() && req.occurred_to.is_none() {
            return bad_request("missing occurred_from/occurred_to").into_response();
        }
        reconcile_shipping_statement(
            &state,
            tenant_id,
            job.job_id,
            req.occurred_from,
            req.occurred_to,
        )
        .await
    };

    match result {
        Ok((diff_count, order_ids)) => {
            let outbox_repo = OutboxRepo::new(state.db.pool());
            for order_id in order_ids {
                let payload = match serde_json::to_value(ProfitComputeRequestedPayload {
                    tenant_id,
                    order_id: OrderId(order_id),
                }) {
                    Ok(v) => v,
                    Err(err) => {
                        let _ = reconcile_repo
                            .update_job(
                                tenant_id,
                                job.job_id,
                                ReconcileJobUpdate {
                                    status: RECONCILE_JOB_STATUS_FAILED.to_string(),
                                    finished_at: Some(Utc::now()),
                                    last_error: Some(err.to_string()),
                                },
                            )
                            .await;
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}"))
                            .into_response();
                    }
                };
                if let Err(err) = outbox_repo
                    .enqueue(tenant_id, PROFIT_COMPUTE_REQUESTED_EVENT, &payload)
                    .await
                {
                    let _ = reconcile_repo
                        .update_job(
                            tenant_id,
                            job.job_id,
                            ReconcileJobUpdate {
                                status: RECONCILE_JOB_STATUS_FAILED.to_string(),
                                finished_at: Some(Utc::now()),
                                last_error: Some(err.to_string()),
                            },
                        )
                        .await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
                }
            }

            let finished_at = Utc::now();
            let _ = reconcile_repo
                .update_job(
                    tenant_id,
                    job.job_id,
                    ReconcileJobUpdate {
                        status: RECONCILE_JOB_STATUS_DONE.to_string(),
                        finished_at: Some(finished_at),
                        last_error: None,
                    },
                )
                .await;

            let audit_repo = AuditLogsRepo::new(state.db.pool());
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "FINANCE.RECONCILE_RUN".to_string(),
                    entity_type: "reconcile_job".to_string(),
                    entity_id: Some(job.job_id.to_string()),
                    before_json: None,
                    after_json: Some(serde_json::json!({
                        "job_type": job_type,
                        "diff_count": diff_count,
                    })),
                    ip: None,
                    ua: None,
                })
                .await;

            (
                StatusCode::OK,
                Json(ReconcileRunResponse {
                    job_id: job.job_id,
                    status: RECONCILE_JOB_STATUS_DONE.to_string(),
                    diff_count,
                }),
            )
                .into_response()
        }
        Err(err) => {
            let _ = reconcile_repo
                .update_job(
                    tenant_id,
                    job.job_id,
                    ReconcileJobUpdate {
                        status: RECONCILE_JOB_STATUS_FAILED.to_string(),
                        finished_at: Some(Utc::now()),
                        last_error: Some(err.to_string()),
                    },
                )
                .await;
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response()
        }
    }
}

pub(crate) async fn run_payment_reconcile(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PaymentReconcileRunRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let statement_id = match uuid::Uuid::parse_str(&req.statement_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad statement_id").into_response(),
    };
    let statement_repo = PaymentStatementsRepo::new(state.db.pool());
    match statement_repo.get_statement(tenant_id, statement_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("statement not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }

    let reconcile_repo = PaymentReconcileJobsRepo::new(state.db.pool());
    let job = match reconcile_repo
        .create_job(PaymentReconcileJobCreate {
            tenant_id,
            statement_id: Some(statement_id),
            status: RECONCILE_JOB_STATUS_RUNNING.to_string(),
            started_at: Some(Utc::now()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    match reconcile_payment_statement(&state, tenant_id, job.job_id, statement_id).await {
        Ok(diff_count) => {
            let _ = reconcile_repo
                .update_job(
                    tenant_id,
                    job.job_id,
                    PaymentReconcileJobUpdate {
                        status: RECONCILE_JOB_STATUS_DONE.to_string(),
                        diff_count,
                        finished_at: Some(Utc::now()),
                        last_error: None,
                    },
                )
                .await;

            let audit_repo = AuditLogsRepo::new(state.db.pool());
            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id,
                    user_id: Some(ctx.user_id),
                    action: "FINANCE.PAYMENT_RECONCILE_RUN".to_string(),
                    entity_type: "payment_reconcile_job".to_string(),
                    entity_id: Some(job.job_id.to_string()),
                    before_json: None,
                    after_json: Some(serde_json::json!({
                        "statement_id": statement_id,
                        "diff_count": diff_count,
                    })),
                    ip: None,
                    ua: None,
                })
                .await;

            (
                StatusCode::OK,
                Json(PaymentReconcileRunResponse {
                    job_id: job.job_id,
                    status: RECONCILE_JOB_STATUS_DONE.to_string(),
                    diff_count,
                }),
            )
                .into_response()
        }
        Err(err) => {
            let _ = reconcile_repo
                .update_job(
                    tenant_id,
                    job.job_id,
                    PaymentReconcileJobUpdate {
                        status: RECONCILE_JOB_STATUS_FAILED.to_string(),
                        diff_count: 0,
                        finished_at: Some(Utc::now()),
                        last_error: Some(err.to_string()),
                    },
                )
                .await;
            let _ = statement_repo
                .update_statement_status(tenant_id, statement_id, PAYMENT_STATEMENT_STATUS_FAILED)
                .await;
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response()
        }
    }
}

pub(crate) async fn reconcile_platform_statement(
    state: &AppState,
    tenant_id: TenantId,
    job_id: uuid::Uuid,
    statement_id: uuid::Uuid,
) -> anyhow::Result<(i64, HashSet<uuid::Uuid>)> {
    let reconcile_repo = ReconcileJobsRepo::new(state.db.pool());
    let statement_repo = PlatformStatementsRepo::new(state.db.pool());
    statement_repo
        .update_statement_status(
            tenant_id,
            statement_id,
            PLATFORM_STATEMENT_STATUS_RECONCILING,
        )
        .await?;

    #[derive(sqlx::FromRow)]
    struct PlatformReconcileRow {
        platform_order_id: String,
        net_cent: i64,
        raw_json: Value,
        order_id: Option<uuid::Uuid>,
        total_amount_cent: Option<i64>,
        discount_amount_cent: Option<i64>,
    }

    let rows: Vec<PlatformReconcileRow> = sqlx::query_as(
        r#"
        select
            l.platform_order_id,
            l.net_cent,
            l.raw_json,
            o.order_id,
            o.total_amount_cent,
            o.discount_amount_cent
        from platform_statement_lines l
        join platform_statements s
          on s.tenant_id = l.tenant_id and s.statement_id = l.statement_id
        left join order_key_index k
          on k.tenant_id = l.tenant_id
         and k.platform = s.platform
         and k.platform_order_id = l.platform_order_id
        left join orders o
          on o.tenant_id = l.tenant_id
         and o.order_id = k.order_id
         and o.order_month = k.order_month
        where l.tenant_id = $1 and l.statement_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(statement_id)
    .fetch_all(state.db.pool())
    .await?;

    let mut items = Vec::new();
    let mut order_ids = HashSet::new();
    for row in rows {
        if let Some(order_id) = row.order_id {
            order_ids.insert(order_id);
        }
        let expected = row
            .total_amount_cent
            .map(|amount| amount - row.discount_amount_cent.unwrap_or(0));
        if let Some(expected_cent) = expected {
            let order_id = row
                .order_id
                .ok_or_else(|| anyhow::anyhow!("missing order_id"))?;
            let diff_cent = row.net_cent - expected_cent;
            if diff_cent != 0 {
                items.push(ReconcileJobItemCreate {
                    tenant_id,
                    job_id,
                    ref_type: RECONCILE_REF_TYPE_ORDER.to_string(),
                    ref_id: order_id.to_string(),
                    expected_cent,
                    actual_cent: row.net_cent,
                    diff_cent,
                    diff_reason: Some(RECONCILE_REASON_AMOUNT_MISMATCH.to_string()),
                    raw_json: serde_json::json!({
                        "platform_order_id": row.platform_order_id,
                        "statement_line": row.raw_json,
                    }),
                });
            }
        } else {
            items.push(ReconcileJobItemCreate {
                tenant_id,
                job_id,
                ref_type: RECONCILE_REF_TYPE_ORDER.to_string(),
                ref_id: row.platform_order_id.clone(),
                expected_cent: 0,
                actual_cent: row.net_cent,
                diff_cent: row.net_cent,
                diff_reason: Some(RECONCILE_REASON_ORDER_MISSING.to_string()),
                raw_json: serde_json::json!({
                    "platform_order_id": row.platform_order_id,
                    "statement_line": row.raw_json,
                }),
            });
        }
    }

    let diff_count = reconcile_repo.insert_job_items(&items).await?;
    statement_repo
        .update_statement_status(
            tenant_id,
            statement_id,
            PLATFORM_STATEMENT_STATUS_RECONCILED,
        )
        .await?;
    Ok((diff_count, order_ids))
}

pub(crate) async fn reconcile_shipping_statement(
    state: &AppState,
    tenant_id: TenantId,
    job_id: uuid::Uuid,
    occurred_from: Option<DateTime<Utc>>,
    occurred_to: Option<DateTime<Utc>>,
) -> anyhow::Result<(i64, HashSet<uuid::Uuid>)> {
    let reconcile_repo = ReconcileJobsRepo::new(state.db.pool());

    #[derive(sqlx::FromRow)]
    struct ShippingReconcileRow {
        tracking_no: String,
        fee_cent: i64,
        raw_json: Value,
        shipment_id: Option<uuid::Uuid>,
        order_id: Option<uuid::Uuid>,
        postage_amount_cent: Option<i64>,
    }

    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        select
            l.tracking_no,
            l.fee_cent,
            l.raw_json,
            s.shipment_id,
            s.order_id,
            o.postage_amount_cent
        from shipping_statement_lines l
        left join shipments s
          on s.tenant_id = l.tenant_id and s.tracking_no = l.tracking_no
        left join order_index k
          on k.tenant_id = s.tenant_id and k.order_id = s.order_id
        left join orders o
          on o.tenant_id = k.tenant_id
         and o.order_id = k.order_id
         and o.order_month = k.order_month
        where l.tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(tenant_id));
    if let Some(from) = occurred_from {
        qb.push(" and l.occurred_at >= ");
        qb.push_bind(from);
    }
    if let Some(to) = occurred_to {
        qb.push(" and l.occurred_at <= ");
        qb.push_bind(to);
    }

    let rows: Vec<ShippingReconcileRow> = qb.build_query_as().fetch_all(state.db.pool()).await?;

    let mut items = Vec::new();
    let mut order_ids = HashSet::new();
    for row in rows {
        if let Some(order_id) = row.order_id {
            order_ids.insert(order_id);
        }
        if let Some(shipment_id) = row.shipment_id {
            let expected_cent = row.postage_amount_cent.unwrap_or(0);
            let diff_cent = row.fee_cent - expected_cent;
            if diff_cent != 0 {
                items.push(ReconcileJobItemCreate {
                    tenant_id,
                    job_id,
                    ref_type: RECONCILE_REF_TYPE_SHIPMENT.to_string(),
                    ref_id: shipment_id.to_string(),
                    expected_cent,
                    actual_cent: row.fee_cent,
                    diff_cent,
                    diff_reason: Some(RECONCILE_REASON_SHIPPING_MISMATCH.to_string()),
                    raw_json: serde_json::json!({
                        "tracking_no": row.tracking_no,
                        "statement_line": row.raw_json,
                    }),
                });
            }
        } else {
            items.push(ReconcileJobItemCreate {
                tenant_id,
                job_id,
                ref_type: RECONCILE_REF_TYPE_SHIPMENT.to_string(),
                ref_id: row.tracking_no.clone(),
                expected_cent: 0,
                actual_cent: row.fee_cent,
                diff_cent: row.fee_cent,
                diff_reason: Some(RECONCILE_REASON_SHIPMENT_MISSING.to_string()),
                raw_json: serde_json::json!({
                    "tracking_no": row.tracking_no,
                    "statement_line": row.raw_json,
                }),
            });
        }
    }

    let diff_count = reconcile_repo.insert_job_items(&items).await?;
    Ok((diff_count, order_ids))
}

pub(crate) async fn reconcile_payment_statement(
    state: &AppState,
    tenant_id: TenantId,
    job_id: uuid::Uuid,
    statement_id: uuid::Uuid,
) -> anyhow::Result<i64> {
    let statement_repo = PaymentStatementsRepo::new(state.db.pool());
    statement_repo
        .update_statement_status(
            tenant_id,
            statement_id,
            PAYMENT_STATEMENT_STATUS_RECONCILING,
        )
        .await?;

    #[derive(sqlx::FromRow)]
    struct PaymentReconcileRow {
        platform_order_id: String,
        paid_cent: i64,
        raw_json: Value,
        order_id: Option<uuid::Uuid>,
        paid_amount_cent: Option<i64>,
        total_amount_cent: Option<i64>,
        discount_amount_cent: Option<i64>,
        refund_cumulative_cent: i64,
        refund_period_cent: i64,
    }

    let rows: Vec<PaymentReconcileRow> = sqlx::query_as(
        r#"
        select
            l.platform_order_id,
            l.paid_cent,
            l.raw_json,
            o.order_id,
            o.paid_amount_cent,
            o.total_amount_cent,
            o.discount_amount_cent,
            coalesce(refunds.refund_cumulative_cent, 0) as refund_cumulative_cent,
            coalesce(refunds.refund_period_cent, 0) as refund_period_cent
        from payment_statement_lines l
        join payment_statements s
          on s.tenant_id = l.tenant_id and s.statement_id = l.statement_id
        left join order_index k
          on k.tenant_id = l.tenant_id
         and k.platform_order_id = l.platform_order_id
        left join orders o
          on o.tenant_id = k.tenant_id
         and o.order_id = k.order_id
         and o.order_month = k.order_month
        left join lateral (
            select
                coalesce(
                    sum(
                        case
                            when rd.status = 'DONE'
                             and rd.created_at::date <= coalesce(
                                 l.occurred_at::date,
                                 s.period_end,
                                 s.period_start,
                                 current_date
                             )
                            then rd.amount_cent
                            else 0
                        end
                    ),
                    0
                )::bigint as refund_cumulative_cent,
                coalesce(
                    sum(
                        case
                            when rd.status = 'DONE'
                             and rd.created_at::date >= coalesce(
                                 s.period_start,
                                 s.period_end,
                                 current_date
                             )
                             and rd.created_at::date <= coalesce(
                                 s.period_end,
                                 s.period_start,
                                 current_date
                             )
                            then rd.amount_cent
                            else 0
                        end
                    ),
                    0
                )::bigint as refund_period_cent
            from after_sales af
            join refund_docs rd
              on rd.tenant_id = af.tenant_id
             and rd.after_sales_id = af.id
            where af.tenant_id = l.tenant_id
              and af.order_id = o.order_id
        ) refunds on true
        where l.tenant_id = $1 and l.statement_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(statement_id)
    .fetch_all(state.db.pool())
    .await?;

    let reconcile_repo = PaymentReconcileJobsRepo::new(state.db.pool());
    let mut items = Vec::new();
    for row in rows {
        let expected = if row.order_id.is_some() {
            row.paid_amount_cent.or_else(|| {
                row.total_amount_cent
                    .map(|amount| amount - row.discount_amount_cent.unwrap_or(0))
            })
        } else {
            None
        };
        if let Some(expected_cent) = expected {
            let refund_cumulative_cent = row.refund_cumulative_cent.max(0);
            let refund_period_cent = row.refund_period_cent.max(0);
            let mut expected_candidates = vec![
                ("ORDER_PAID", expected_cent),
                (
                    "NET_AFTER_REFUND",
                    expected_cent.saturating_sub(refund_cumulative_cent),
                ),
            ];
            if refund_period_cent > 0 {
                expected_candidates.push(("REFUND_DELTA", refund_period_cent.saturating_mul(-1)));
            }

            let (expected_basis, reconcile_expected_cent) = expected_candidates
                .iter()
                .copied()
                .min_by_key(|(_, candidate)| {
                    (i128::from(row.paid_cent) - i128::from(*candidate)).abs()
                })
                .unwrap_or(("ORDER_PAID", expected_cent));

            let diff_cent = row.paid_cent - reconcile_expected_cent;
            if diff_cent != 0 {
                let candidate_values: Vec<Value> = expected_candidates
                    .iter()
                    .map(|(basis, candidate)| {
                        serde_json::json!({
                            "basis": basis,
                            "expected_cent": candidate,
                        })
                    })
                    .collect();
                items.push(PaymentReconcileItemCreate {
                    tenant_id,
                    job_id,
                    platform_order_id: row.platform_order_id.clone(),
                    order_id: row.order_id.map(OrderId),
                    expected_cent: reconcile_expected_cent,
                    actual_cent: row.paid_cent,
                    diff_cent,
                    diff_reason: Some(RECONCILE_REASON_AMOUNT_MISMATCH.to_string()),
                    raw_json: serde_json::json!({
                        "platform_order_id": row.platform_order_id,
                        "statement_line": row.raw_json,
                        "expected_basis": expected_basis,
                        "expected_candidates": candidate_values,
                        "refund_cumulative_cent": refund_cumulative_cent,
                        "refund_period_cent": refund_period_cent,
                    }),
                });
            }
        } else {
            items.push(PaymentReconcileItemCreate {
                tenant_id,
                job_id,
                platform_order_id: row.platform_order_id.clone(),
                order_id: row.order_id.map(OrderId),
                expected_cent: 0,
                actual_cent: row.paid_cent,
                diff_cent: row.paid_cent,
                diff_reason: Some(RECONCILE_REASON_ORDER_MISSING.to_string()),
                raw_json: serde_json::json!({
                    "platform_order_id": row.platform_order_id,
                    "statement_line": row.raw_json,
                }),
            });
        }
    }

    let diff_count = reconcile_repo.insert_items(&items).await?;
    statement_repo
        .update_statement_status(tenant_id, statement_id, PAYMENT_STATEMENT_STATUS_RECONCILED)
        .await?;
    Ok(diff_count)
}

pub(crate) async fn list_reconcile_jobs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ReconcileJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = ReconcileJobsRepo::new(state.db.pool());
    let result = match repo
        .list_jobs(
            tenant_id,
            ReconcileJobListFilters {
                job_type: query.job_type.and_then(non_empty),
                status: query.status.and_then(non_empty),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|job| ReconcileJobResponse {
            job_id: job.job_id,
            job_type: job.job_type,
            status: job.status,
            started_at: job.started_at,
            finished_at: job.finished_at,
            last_error: job.last_error,
            created_at: job.created_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(ReconcileJobListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_reconcile_job_items(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ReconcileJobItemListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let job_id = match uuid::Uuid::parse_str(&query.job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = ReconcileJobsRepo::new(state.db.pool());
    let result = match repo
        .list_job_items(
            tenant_id,
            ReconcileJobItemListFilters {
                job_id,
                ref_type: query.ref_type.and_then(non_empty),
                diff_only: query.diff_only.unwrap_or(true),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|item| ReconcileJobItemResponse {
            job_id: item.job_id,
            ref_type: item.ref_type,
            ref_id: item.ref_id,
            expected_cent: item.expected_cent,
            actual_cent: item.actual_cent,
            diff_cent: item.diff_cent,
            diff_reason: item.diff_reason,
            raw_json: item.raw_json,
            created_at: item.created_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(ReconcileJobItemListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_payment_reconcile_jobs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PaymentReconcileJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let statement_id = match query.statement_id.and_then(non_empty) {
        Some(value) => match uuid::Uuid::parse_str(&value) {
            Ok(parsed) => Some(parsed),
            Err(_) => return bad_request("bad statement_id").into_response(),
        },
        None => None,
    };
    let repo = PaymentReconcileJobsRepo::new(state.db.pool());
    let result = match repo
        .list_jobs(
            tenant_id,
            PaymentReconcileJobListFilters {
                status: query.status.and_then(non_empty),
                statement_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|job| PaymentReconcileJobResponse {
            job_id: job.job_id,
            statement_id: job.statement_id,
            status: job.status,
            diff_count: job.diff_count,
            started_at: job.started_at,
            finished_at: job.finished_at,
            last_error: job.last_error,
            created_at: job.created_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(PaymentReconcileJobListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_payment_reconcile_items(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PaymentReconcileItemListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let job_id = match uuid::Uuid::parse_str(&query.job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = PaymentReconcileJobsRepo::new(state.db.pool());
    let result = match repo
        .list_items(
            tenant_id,
            PaymentReconcileItemListFilters {
                job_id,
                diff_only: query.diff_only.unwrap_or(false),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|item| PaymentReconcileItemResponse {
            job_id: item.job_id,
            platform_order_id: item.platform_order_id,
            order_id: item.order_id.map(uuid::Uuid::from),
            expected_cent: item.expected_cent,
            actual_cent: item.actual_cent,
            diff_cent: item.diff_cent,
            diff_reason: item.diff_reason,
            raw_json: item.raw_json,
            created_at: item.created_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(PaymentReconcileItemListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_writeoff_records(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WriteoffRecordListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = WriteoffRecordsRepo::new(state.db.pool());
    let result = match repo
        .list_records(
            tenant_id,
            WriteoffRecordListFilters {
                ref_type: query.ref_type.and_then(non_empty),
                ref_id: query.ref_id.and_then(non_empty),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|record| WriteoffRecordResponse {
            writeoff_id: record.writeoff_id,
            ref_type: record.ref_type,
            ref_id: record.ref_id,
            amount_cent: record.amount_cent,
            reason: record.reason,
            created_by: record.created_by,
            created_at: record.created_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(WriteoffRecordListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn apply_invoice(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InvoiceApplyRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&req.order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let repo = InvoicesRepo::new(state.db.pool());
    let invoice = match repo
        .create_invoice(InvoiceCreate {
            tenant_id,
            order_id,
            invoice_title: req.invoice_title,
            tax_no: req.tax_no,
            status: "APPLIED".to_string(),
            result_json: Some(serde_json::json!({})),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "FINANCE.INVOICE_APPLIED".to_string(),
            entity_type: "invoice".to_string(),
            entity_id: Some(invoice.invoice_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&invoice).unwrap_or(Value::Null)),
            ip: None,
            ua: None,
        })
        .await;

    (
        StatusCode::OK,
        Json(InvoiceResponse {
            invoice_id: invoice.invoice_id,
            order_id: invoice.order_id.to_string(),
            invoice_title: invoice.invoice_title,
            tax_no: invoice.tax_no,
            status: invoice.status,
            issued_at: invoice.issued_at,
            result_json: invoice.result_json,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn list_invoices(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InvoiceListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let order_id = match query.order_id.as_deref() {
        Some(raw) => match OrderId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = InvoicesRepo::new(state.db.pool());
    let result = match repo
        .list_invoices(
            tenant_id,
            InvoiceListFilters {
                status: query.status.and_then(non_empty),
                order_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|invoice| InvoiceResponse {
            invoice_id: invoice.invoice_id,
            order_id: invoice.order_id.to_string(),
            invoice_title: invoice.invoice_title,
            tax_no: invoice.tax_no,
            status: invoice.status,
            issued_at: invoice.issued_at,
            result_json: invoice.result_json,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(InvoiceListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_ap_docs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ApDocListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let supplier_id = match query.supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = ApDocsRepo::new(state.db.pool());
    let result = match repo
        .list_docs(
            tenant_id,
            ApDocListFilters {
                supplier_id,
                status: query.status.and_then(non_empty),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|doc| ApDocResponse {
            ap_id: doc.ap_id,
            supplier_id: doc.supplier_id,
            supplier_name: doc.supplier_name,
            ref_type: doc.ref_type,
            ref_id: doc.ref_id,
            amount_cent: doc.amount_cent,
            status: doc.status,
            settlement_status: doc.settlement_status,
            paid_at: doc.paid_at,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(ApDocListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_ap_doc(
    State(state): State<AppState>,
    Path((tenant_id, ap_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let ap_id = match uuid::Uuid::parse_str(&ap_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad ap_id").into_response(),
    };
    let repo = ApDocsRepo::new(state.db.pool());
    let doc = match repo.get_doc(tenant_id, ap_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("ap doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(ApDocResponse {
            ap_id: doc.ap_id,
            supplier_id: doc.supplier_id,
            supplier_name: doc.supplier_name,
            ref_type: doc.ref_type,
            ref_id: doc.ref_id,
            amount_cent: doc.amount_cent,
            status: doc.status,
            settlement_status: doc.settlement_status,
            paid_at: doc.paid_at,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn mark_ap_doc_paid(
    State(state): State<AppState>,
    Path((tenant_id, ap_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }
    let ap_id = match uuid::Uuid::parse_str(&ap_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad ap_id").into_response(),
    };
    let repo = ApDocsRepo::new(state.db.pool());
    let doc = match repo.mark_paid(tenant_id, ap_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("ap doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "FINANCE.AP_DOC_PAID".to_string(),
            entity_type: "ap_doc".to_string(),
            entity_id: Some(doc.ap_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&doc).unwrap_or(Value::Null)),
            ip: None,
            ua: None,
        })
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_AP_DOC_PAID.to_string(),
            ref_type: Some("AP_DOC".to_string()),
            ref_id: Some(doc.ap_id.to_string()),
            metadata: Some(serde_json::to_value(&doc).unwrap_or(Value::Null)),
        })
        .await;

    (
        StatusCode::OK,
        Json(ApDocResponse {
            ap_id: doc.ap_id,
            supplier_id: doc.supplier_id,
            supplier_name: doc.supplier_name,
            ref_type: doc.ref_type,
            ref_id: doc.ref_id,
            amount_cent: doc.amount_cent,
            status: doc.status,
            settlement_status: doc.settlement_status,
            paid_at: doc.paid_at,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn close_ap_doc(
    State(state): State<AppState>,
    Path((tenant_id, ap_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }
    let ap_id = match uuid::Uuid::parse_str(&ap_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad ap_id").into_response(),
    };

    let repo = ApDocsRepo::new(state.db.pool());
    let current = match repo.get_doc(tenant_id, ap_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("ap doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if current.status != "PAID" {
        return bad_request("ap doc must be paid before closing").into_response();
    }

    let doc = match repo.close_doc(tenant_id, ap_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("ap doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "FINANCE.AP_DOC_CLOSED".to_string(),
            entity_type: "ap_doc".to_string(),
            entity_id: Some(doc.ap_id.to_string()),
            before_json: Some(serde_json::to_value(&current).unwrap_or(Value::Null)),
            after_json: Some(serde_json::to_value(&doc).unwrap_or(Value::Null)),
            ip: None,
            ua: None,
        })
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_AP_DOC_CLOSED.to_string(),
            ref_type: Some("AP_DOC".to_string()),
            ref_id: Some(doc.ap_id.to_string()),
            metadata: Some(serde_json::to_value(&doc).unwrap_or(Value::Null)),
        })
        .await;

    (
        StatusCode::OK,
        Json(ApDocResponse {
            ap_id: doc.ap_id,
            supplier_id: doc.supplier_id,
            supplier_name: doc.supplier_name,
            ref_type: doc.ref_type,
            ref_id: doc.ref_id,
            amount_cent: doc.amount_cent,
            status: doc.status,
            settlement_status: doc.settlement_status,
            paid_at: doc.paid_at,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }),
    )
        .into_response()
}

pub(crate) async fn list_profit_snapshots(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProfitListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match ShopId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = ProfitSnapshotsRepo::new(state.db.pool());
    let result = match repo
        .list_snapshots(
            tenant_id,
            ProfitSnapshotListFilters {
                shop_id,
                sku_id: query.sku_id.and_then(non_empty),
                created_from: query.created_from,
                created_to: query.created_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let items = result
        .items
        .into_iter()
        .map(|snapshot| ProfitSnapshotResponse {
            order_id: snapshot.order_id.to_string(),
            shop_id: snapshot.shop_id.to_string(),
            platform_order_id: snapshot.platform_order_id,
            order_created_at: snapshot.order_created_at,
            revenue_cent: snapshot.revenue_cent,
            cost_cent: snapshot.cost_cent,
            platform_fee_cent: snapshot.platform_fee_cent,
            shipping_fee_cent: snapshot.shipping_fee_cent,
            gross_profit_cent: snapshot.gross_profit_cent,
            computed_at: snapshot.computed_at,
        })
        .collect();
    (
        StatusCode::OK,
        Json(ProfitSnapshotListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_profit_snapshot_detail(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let repo = ProfitSnapshotsRepo::new(state.db.pool());
    let summary = match repo.get_snapshot_summary(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("profit snapshot not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let order_month: Option<NaiveDate> = match sqlx::query_scalar(
        r#"
        select order_month
        from order_index
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(order_month) = order_month else {
        return not_found("order not found").into_response();
    };

    #[derive(sqlx::FromRow)]
    struct RevenueRow {
        line_id: String,
        sku_id: Option<String>,
        qty: i64,
        amount_cent: Option<i64>,
        pay_amount_cent: Option<i64>,
    }
    let revenue_rows: Vec<RevenueRow> = match sqlx::query_as(
        r#"
        select line_id, sku_id, qty, amount_cent, pay_amount_cent
        from order_items
        where tenant_id = $1 and order_id = $2 and order_month = $3
        order by line_id
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(order_month)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let revenue_items = revenue_rows
        .into_iter()
        .map(|row| ProfitSourceRevenueItem {
            line_id: row.line_id,
            sku_id: row.sku_id,
            qty: row.qty,
            amount_cent: row.amount_cent,
            pay_amount_cent: row.pay_amount_cent,
        })
        .collect();

    #[derive(sqlx::FromRow)]
    struct CogsRow {
        sku_id: uuid::Uuid,
        qty: i64,
        unit_cost_cent: i64,
        total_cost_cent: i64,
        ref_type: String,
        ref_id: String,
        shipment_id: Option<uuid::Uuid>,
        outbound_doc_id: Option<uuid::Uuid>,
        order_item_id: Option<uuid::Uuid>,
        created_at: DateTime<Utc>,
    }
    let cogs_rows: Vec<CogsRow> = match sqlx::query_as(
        r#"
        select sku_id, qty, unit_cost_cent, total_cost_cent, ref_type, ref_id,
               shipment_id, outbound_doc_id, order_item_id, created_at
        from order_cogs
        where tenant_id = $1 and order_id = $2
        order by created_at asc
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let cogs_items = cogs_rows
        .into_iter()
        .map(|row| ProfitSourceCogsItem {
            sku_id: row.sku_id,
            qty: row.qty,
            unit_cost_cent: row.unit_cost_cent,
            total_cost_cent: row.total_cost_cent,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            shipment_id: row.shipment_id,
            outbound_doc_id: row.outbound_doc_id,
            order_item_id: row.order_item_id,
            created_at: row.created_at,
        })
        .collect();

    #[derive(sqlx::FromRow)]
    struct PlatformFeeRow {
        statement_id: uuid::Uuid,
        platform_order_id: String,
        fee_cent: i64,
        occurred_at: Option<DateTime<Utc>>,
    }
    let platform_fee_rows: Vec<PlatformFeeRow> = match sqlx::query_as(
        r#"
        select statement_id, platform_order_id, fee_cent, occurred_at
        from platform_statement_lines
        where tenant_id = $1 and platform_order_id in (
            select platform_order_id
            from order_key_index
            where tenant_id = $1 and order_id = $2
        )
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let platform_fee_lines = platform_fee_rows
        .into_iter()
        .map(|row| ProfitSourcePlatformFeeLine {
            statement_id: row.statement_id,
            platform_order_id: row.platform_order_id,
            fee_cent: row.fee_cent,
            occurred_at: row.occurred_at,
        })
        .collect();

    #[derive(sqlx::FromRow)]
    struct ShippingFeeRow {
        line_id: uuid::Uuid,
        tracking_no: String,
        fee_cent: i64,
        occurred_at: Option<DateTime<Utc>>,
    }
    let shipping_fee_rows: Vec<ShippingFeeRow> = match sqlx::query_as(
        r#"
        select l.line_id, l.tracking_no, l.fee_cent, l.occurred_at
        from shipping_statement_lines l
        join shipments s
          on s.tenant_id = l.tenant_id and s.tracking_no = l.tracking_no
        where s.tenant_id = $1 and s.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let shipping_fee_lines = shipping_fee_rows
        .into_iter()
        .map(|row| ProfitSourceShippingFeeLine {
            line_id: row.line_id,
            tracking_no: row.tracking_no,
            fee_cent: row.fee_cent,
            occurred_at: row.occurred_at,
        })
        .collect();

    let snapshot = ProfitSnapshotResponse {
        order_id: summary.order_id.to_string(),
        shop_id: summary.shop_id.to_string(),
        platform_order_id: summary.platform_order_id,
        order_created_at: summary.order_created_at,
        revenue_cent: summary.revenue_cent,
        cost_cent: summary.cost_cent,
        platform_fee_cent: summary.platform_fee_cent,
        shipping_fee_cent: summary.shipping_fee_cent,
        gross_profit_cent: summary.gross_profit_cent,
        computed_at: summary.computed_at,
    };

    (
        StatusCode::OK,
        Json(ProfitSnapshotDetailResponse {
            snapshot,
            revenue_items,
            cogs_items,
            platform_fee_lines,
            shipping_fee_lines,
        }),
    )
        .into_response()
}

pub(crate) async fn refresh_profit_snapshot_for_order(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
    user_id: Option<UserId>,
    trigger: &str,
) -> anyhow::Result<()> {
    let repo = ProfitSnapshotsRepo::new(state.db.pool());
    let (revenue_cent, cost_cent, platform_fee_cent, shipping_fee_cent) =
        repo.compute_profit_inputs(tenant_id, order_id).await?;
    let gross_profit_cent = revenue_cent - cost_cent - platform_fee_cent - shipping_fee_cent;
    let computed_at = Utc::now();
    let snapshot = repo
        .upsert_snapshot(ProfitSnapshotUpsert {
            tenant_id,
            order_id,
            revenue_cent,
            cost_cent,
            platform_fee_cent,
            shipping_fee_cent,
            gross_profit_cent,
            computed_at,
        })
        .await?;

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id,
            action: "FINANCE.PROFIT_SNAPSHOT_UPSERT".to_string(),
            entity_type: "profit_snapshot".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "trigger": trigger,
                "snapshot": snapshot,
            })),
            ip: None,
            ua: None,
        })
        .await?;

    Ok(())
}

pub(crate) async fn load_oms_shipment_items(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
) -> Result<Vec<OmsShipmentItemView>, Response> {
    #[derive(sqlx::FromRow)]
    struct Row {
        order_item_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        sku_code: Option<String>,
        qty: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select si.order_item_id,
               si.sku_id,
               s.sku_code,
               si.qty
        from shipment_items si
        left join skus s
          on s.sku_id = si.sku_id
        where si.tenant_id = $1
          and si.shipment_id = $2
        order by si.shipment_item_id asc
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())?;

    Ok(rows
        .into_iter()
        .map(|row| OmsShipmentItemView {
            order_item_id: row.order_item_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            qty: row.qty,
        })
        .collect())
}

pub(crate) fn oms_shipment_summary(
    tenant_id: uuid::Uuid,
    shipment: erp_repo::fulfillment::OmsShipment,
    items: Option<Vec<OmsShipmentItemView>>,
) -> OmsShipmentSummary {
    OmsShipmentSummary {
        shipment_id: shipment.shipment_id,
        shipment_no: shipment.shipment_no,
        shipment_type: shipment.shipment_type,
        origin_shipment_id: shipment.origin_shipment_id,
        tenant_id,
        order_id: shipment.order_id,
        shop_id: shipment.shop_id,
        platform: shipment.platform,
        platform_order_id: shipment.platform_order_id,
        warehouse_id: shipment.warehouse_id,
        carrier_code: shipment.carrier_code,
        tracking_no: shipment.tracking_no,
        status: shipment.status,
        weight_gram: shipment.weight_gram,
        shipped_at: shipment.shipped_at,
        created_at: shipment.created_at,
        updated_at: shipment.updated_at,
        items,
    }
}

pub(crate) async fn resolve_print_mapping(
    state: &AppState,
    tenant_id: TenantId,
    doc_type: &str,
) -> Result<(uuid::Uuid, Option<String>), Response> {
    let repo = PrintMappingsRepo::new(state.db.pool());
    let mapping = match repo.get_mapping(tenant_id, doc_type).await {
        Ok(Some(mapping)) => mapping,
        Ok(None) => return Err(bad_request("print mapping not found").into_response()),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()),
    };

    match state
        .print_templates_repo
        .get_template(tenant_id, mapping.template_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => return Err(bad_request("print template not found").into_response()),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()),
    }

    let local_agent_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::PrintLocalAgent).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };

    let mut device_fingerprint = if local_agent_enabled {
        mapping.device_fingerprint
    } else {
        None
    };

    if local_agent_enabled {
        if let Some(fingerprint) = device_fingerprint.as_deref() {
            let devices_repo = PrintDevicesRepoCore::new(state.db.pool());
            match devices_repo
                .get_device_by_fingerprint(tenant_id, fingerprint)
                .await
            {
                Ok(Some(_)) => {}
                Ok(None) => return Err(bad_request("print device not found").into_response()),
                Err(e) => {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
                }
            }
        } else {
            match state.print_devices_repo.get_default_device(tenant_id).await {
                Ok(Some(device)) => device_fingerprint = Some(device.fingerprint),
                Ok(None) => {}
                Err(e) => {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
                }
            }
        }
    }

    Ok((mapping.template_id, device_fingerprint))
}
