//! Current manufacturing/warehouse authority guards the real exact batch quote.
use super::*;
use erp_repo::shared_contract::{MaterialBatchQuery, SharedContractRepo};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BatchQuery {
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    #[serde(default)]
    query: String,
    offset: i64,
    limit: i64,
}
pub(super) async fn list(
    State(state): State<AppState>,
    Path((path, order)): Path<(String, uuid::Uuid)>,
    Query(query): Query<BatchQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> Response {
    let tenant = match require_tenant(&path, &ctx) {
        Ok(t) => t,
        Err(r) => return r,
    };
    let actor = match work_order_catalog::actor(&ctx, epoch) {
        Ok(a) => a,
        Err(r) => return r,
    };
    match SharedContractRepo::new(state.db.pool())
        .material_batch_options(
            tenant,
            MaterialBatchQuery {
                work_order_id: order,
                warehouse_id: query.warehouse_id,
                sku_id: query.sku_id,
                text: &query.query,
                offset: query.offset,
                limit: query.limit,
            },
            actor,
        )
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(error)
            if error
                .downcast_ref::<erp_repo::inventory_batch_effects::InventoryBatchLimit>()
                .is_some() =>
        {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"code":"inventory_batch_limit"})),
            )
                .into_response()
        }
        Err(error) => work_order_catalog::failure(error),
    }
}
