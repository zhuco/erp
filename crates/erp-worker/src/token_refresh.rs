use super::*;

pub(super) async fn handle_token_refresh_requested(
    db: &Db,
    adapters: &AdapterRegistry,
    payload: erp_domain::TokenRefreshRequestedPayload,
) -> anyhow::Result<()> {
    let repo = PlatformTokensRepo::new(db.pool());
    let Some(token) = repo.get_token(payload.tenant_id, payload.shop_id).await? else {
        return Err(anyhow::anyhow!("platform token not found"));
    };

    let candidate = erp_repo::platform_tokens::TokenRefreshCandidate {
        token,
        platform: payload.platform,
    };
    refresh_token_for_candidate(db, adapters, &candidate).await
}

pub(super) async fn refresh_tokens_once(
    db: &Db,
    adapters: &AdapterRegistry,
    threshold: ChronoDuration,
    batch_limit: i64,
) -> anyhow::Result<()> {
    let repo = PlatformTokensRepo::new(db.pool());
    let candidates = repo
        .list_expiring_tokens(Utc::now() + threshold, batch_limit)
        .await?;

    for candidate in candidates {
        if let Err(err) = refresh_token_for_candidate(db, adapters, &candidate).await {
            warn!(
                error = %err,
                tenant_id = %candidate.token.tenant_id,
                shop_id = %candidate.token.shop_id,
                "token refresh failed"
            );
        }
    }

    Ok(())
}

pub(super) async fn refresh_token_for_candidate(
    db: &Db,
    adapters: &AdapterRegistry,
    candidate: &erp_repo::platform_tokens::TokenRefreshCandidate,
) -> anyhow::Result<()> {
    let repo = PlatformTokensRepo::new(db.pool());
    let shops_repo = ShopsRepo::new(db.pool());
    let outbox_repo = OutboxRepo::new(db.pool());

    let shop = shops_repo
        .get_shop_auth(candidate.token.shop_id)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "shop not available for refresh {}: {err}",
                candidate.token.shop_id
            )
        })?;
    let adapter = adapters.platform_adapter(candidate.platform);
    let token_payload = PlatformTokenPayload {
        access_token: candidate.token.access_token.clone(),
        refresh_token: candidate.token.refresh_token.clone(),
        expires_at: candidate.token.expires_at,
        scopes: candidate.token.scopes.clone(),
    };

    match adapter.refresh_token(&shop, &token_payload).await {
        Ok(result) => {
            let now = Utc::now();
            let access_token = result.access_token.or(candidate.token.access_token.clone());
            let refresh_token = result
                .refresh_token
                .or(candidate.token.refresh_token.clone());
            let expires_at = result.expires_at.or(candidate.token.expires_at);
            let scopes = result.scopes.or(candidate.token.scopes.clone());

            let _ = repo
                .upsert_token(erp_repo::platform_tokens::PlatformTokenUpsert {
                    tenant_id: candidate.token.tenant_id,
                    shop_id: candidate.token.shop_id,
                    token_status: None,
                    access_token,
                    refresh_token,
                    expires_at,
                    scopes,
                    last_refreshed_at: Some(now),
                    refresh_fail_count: Some(0),
                    last_error: Some("".to_string()),
                })
                .await?;

            let status = if expires_at.map(|t| t > now).unwrap_or(false) {
                "AUTHORIZED"
            } else {
                "EXPIRED"
            };
            let _ = shops_repo
                .update_shop(
                    candidate.token.tenant_id,
                    candidate.token.shop_id,
                    erp_repo::shops::ShopUpdate {
                        name: None,
                        status: Some(status.to_string()),
                        credentials: None,
                        auth_meta_json: None,
                    },
                )
                .await?;
        }
        Err(err) => {
            let _ = repo
                .mark_refresh_failed(
                    candidate.token.tenant_id,
                    candidate.token.shop_id,
                    err.to_string(),
                )
                .await?;
            let _ = shops_repo
                .update_shop(
                    candidate.token.tenant_id,
                    candidate.token.shop_id,
                    erp_repo::shops::ShopUpdate {
                        name: None,
                        status: Some("EXPIRED".to_string()),
                        credentials: None,
                        auth_meta_json: None,
                    },
                )
                .await?;

            let payload = serde_json::to_value(erp_domain::TokenRefreshFailedPayload {
                tenant_id: candidate.token.tenant_id,
                shop_id: candidate.token.shop_id,
                platform: candidate.platform,
                last_error: err.to_string(),
            })?;
            outbox_repo
                .enqueue(candidate.token.tenant_id, "TOKEN_REFRESH_FAILED", &payload)
                .await?;
        }
    }

    Ok(())
}
