//! Negative recovery contracts, not browser/native business acceptance.
use super::material_model::*;
use crate::{http::HttpClient, remote_actions::ActionScope};
use erp_grid_core::RowId;
use serde_json::{Value, json};
use uuid::Uuid;
#[test]
fn material_ui_batch_quote_rejects_wrong_dimensions_expired_ready_and_unknown_quantity()
-> Result<(), String> {
    let (draft, _) = fixture()?;
    let line = &draft.lines[0];
    let order = draft.order.as_ref().ok_or("order")?.work_order_id;
    let raw = json!({"work_order_id":order,"warehouse_id":line.warehouse_id,"sku_id":line.sku_id,"batch_enabled":true,"observed_at":"2026-10-10T00:00:00Z","warehouse_available_micros":"750000","needs_reconciliation":false,"total":1,"offset":0,"limit":500,
        "data":[{"batch_id":Uuid::new_v4(),"batch_no":"REAL-LOT","expire_at":"2026-10-11T00:00:00Z","on_hand_micros":"500000","reserved_micros":"100000","available_micros":"400000","state":"READY"}]});
    let page: BatchPage = serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
    page.validate(order, line, 0)?;
    for key in ["work_order_id", "warehouse_id", "sku_id"] {
        let mut bad = raw.clone();
        bad[key] = json!(Uuid::new_v4());
        let page: BatchPage = serde_json::from_value(bad).map_err(|e| e.to_string())?;
        assert!(page.validate(order, line, 0).is_err());
    }
    for (key, value) in [
        ("on_hand_micros", Value::Null),
        ("available_micros", json!("900000")),
        ("expire_at", json!("2026-10-09T00:00:00Z")),
    ] {
        let mut bad = raw.clone();
        bad["data"][0][key] = value;
        let page: BatchPage = serde_json::from_value(bad).map_err(|e| e.to_string())?;
        assert!(page.validate(order, line, 0).is_err());
    }
    Ok(())
}

pub(super) fn fixture() -> Result<(Draft, ActionScope), String> {
    let client = HttpClient::new("https://owned-material.qa.invalid").map_err(|e| e.to_string())?;
    let scope = ActionScope::new(&client, RowId(1), RowId(2))?;
    let order = WorkOrder {
        work_order_id: Uuid::new_v4(),
        work_order_no: "QA-WO".into(),
        product_code: "QA-P".into(),
        status: "RELEASED".into(),
        bom_id: Some(Uuid::new_v4()),
        order_qty_micros: "9007199254740993".into(),
        issued_micros: "0".into(),
        consumed_micros: "0".into(),
    };
    let c = Component {
        component_code: "QA-M".into(),
        sku_id: Some(Uuid::new_v4()),
        sku_name: Some("原材料".into()),
        unit_code: "kg".into(),
        direct_component: true,
    };
    let w = Warehouse {
        warehouse_id: Uuid::new_v4(),
        code: Some("QA-W".into()),
        name: "原料仓".into(),
    };
    let mut line = Line::new(&c, &w)?;
    line.qty = "0.250000".into();
    Ok((
        Draft {
            document_id: Some(Uuid::new_v4()),
            acknowledged_source: Some(order.source()),
            order: Some(order),
            lines: vec![line],
            changed: true,
            ..Default::default()
        },
        scope,
    ))
}
pub(super) fn saved(d: &Draft, p: &Prepared) -> Result<Value, String> {
    Ok(
        json!({"schema":1,"document_id":d.document_id,"work_order_id":d.order.as_ref().ok_or("order")?.work_order_id,
        "version_no":1,"request_id":p.request,"actor_user_id":p.context["actor_user_id"],"lines":d.lines.iter().map(Line::saved).collect::<Result<Vec<_>,_>>()?,
        "source":d.acknowledged_source,"status":"DRAFT","posted":false,"revision_id":Uuid::new_v4(),"saved_at":"2026-10-10T01:00:00Z"}),
    )
}
#[test]
fn material_ui_rejection_requires_all_context_and_original_fields() -> Result<(), String> {
    let (draft, scope) = fixture()?;
    let p = draft.prepare(Command::Save, &scope, None)?;
    let good = json!({"code":"material_document_rejected_before_commit","committed":false,"request_id":p.request,"fields":p.body,"context":p.context,"message":"source changed"});
    assert!(rejected(&p, &good)?.is_some());
    for key in ["tenant_id", "actor_user_id", "work_order_id", "document_id"] {
        let mut bad = good.clone();
        bad["context"][key] = json!(Uuid::new_v4());
        assert!(rejected(&p, &bad).is_err());
    }
    let mut bad = good.clone();
    bad["fields"]["lines"][0]["qty"] = json!("0.500000");
    assert!(rejected(&p, &bad).is_err());
    let mut bad = good.clone();
    bad["request_id"] = json!(Uuid::new_v4());
    assert!(rejected(&p, &bad).is_err());
    let mut bad = good;
    bad["committed"] = json!(true);
    assert!(rejected(&p, &bad).is_err());
    assert!(rejected(&p, &json!({"code":"VERSION_CONFLICT"}))?.is_none());
    Ok(())
}
#[test]
fn material_ui_source_adoption_preserves_local_unsaved_lines() -> Result<(), String> {
    let (draft, scope) = fixture()?;
    let first = draft.prepare(Command::Save, &scope, None)?;
    let mut draft = matched(&draft, &first, &json!({"snapshot":saved(&draft,&first)?}))?;
    let prior = draft.saved.as_ref().ok_or("save")?["source"].clone();
    draft.lines[0].qty = "0.750000".into();
    draft.changed = true;
    let mut current = prior.clone();
    current["order_qty_micros"] = json!("20000000");
    let review = json!({"document_id":draft.document_id,"work_order_id":draft.order.as_ref().ok_or("order")?.work_order_id,
        "version_no":1,"saved_source":prior,"current_source":current,"review_token":"a".repeat(64)});
    let p = draft.prepare(Command::Adopt, &scope, Some(review.clone()))?;
    let mut result = draft.saved.clone().ok_or("save")?;
    result["request_id"] = json!(p.request);
    result["version_no"] = json!(2);
    result["source"] = current;
    result["source_adoption"] =
        json!({"previous_source":prior,"review_token":review["review_token"]});
    let adopted = matched(&draft, &p, &json!({"snapshot":result}))?;
    assert_eq!(adopted.lines[0].qty, "0.750000");
    assert!(adopted.changed);
    assert_eq!(adopted.version(), 2);
    assert_eq!(
        adopted.saved.as_ref().ok_or("save")?["lines"][0]["qty_micros"],
        "250000"
    );
    assert!(adopted.prepare(Command::Confirm, &scope, None).is_err());
    assert_ne!(
        adopted.prepare(Command::Save, &scope, None)?.request,
        p.request
    );
    Ok(())
}
#[test]
fn material_ui_pending_checkpoint_rejects_foreign_scope_and_mutated_command() -> Result<(), String>
{
    let (draft, scope) = fixture()?;
    let pending = draft.prepare(Command::Save, &scope, None)?;
    let mut record = Checkpoint {
        schema: 1,
        scope: scope.clone(),
        draft,
        pending: Some(pending.clone()),
        rejected: None,
    };
    let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
    let restored = Checkpoint::decode(&bytes, &scope)?;
    assert_eq!(
        restored.pending.as_ref().ok_or("pending")?.body,
        pending.body
    );
    let client = HttpClient::new("https://owned-material.qa.invalid").map_err(|e| e.to_string())?;
    let other = ActionScope::new(&client, RowId(1), RowId(3))?;
    assert!(Checkpoint::decode(&bytes, &other).is_err());
    record.pending.as_mut().ok_or("pending")?.body["lines"][0]["qty"] = json!("2");
    assert!(
        Checkpoint::decode(
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
            &scope
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn material_ui_unknown_cost_or_missing_line_is_not_posted_success() -> Result<(), String> {
    let (draft, scope) = fixture()?;
    let first = draft.prepare(Command::Save, &scope, None)?;
    let mut draft = matched(&draft, &first, &json!({"snapshot":saved(&draft,&first)?}))?;
    let p = draft.prepare(Command::Confirm, &scope, None)?;
    let mut result = draft.saved.clone().ok_or("save")?;
    result["schema"] = json!(2);
    result["posted"] = json!(true);
    result["status"] = json!("POSTED");
    result["request_id"] = json!(p.request);
    result["posted_at"] = json!("2026-10-10T01:00:00Z");
    for key in ["operation_log_id", "audit_id", "event_id"] {
        result[key] = json!(Uuid::new_v4());
    }
    let mut effect = draft.lines[0].saved()?;
    effect["move_id"] = json!(Uuid::new_v4());
    effect["stock"] = json!({"on_hand_micros":"750000","reserved_micros":"0","frozen_micros":"0","in_transit_micros":"0"});
    effect["material"] = json!({"issued_micros":"250000","returned_micros":"0","consumed_micros":"0","shop_micros":"250000"});
    effect["value"] = json!({"state":"VALUED","amount_cent":"0","original":{"known_zero":true}});
    result["effects"] = json!([effect]);
    assert!(matched(&draft, &p, &json!({"snapshot":result.clone()}))?.posted());
    let mut bad = result.clone();
    bad["effects"][0]["value"]["state"] = json!("UNVALUED");
    assert!(matched(&draft, &p, &json!({"snapshot":bad})).is_err());
    let mut bad = result.clone();
    bad["effects"][0]["stock"]["on_hand_micros"] = Value::Null;
    assert!(matched(&draft, &p, &json!({"snapshot":bad})).is_err());
    result["effects"] = json!([]);
    assert!(matched(&draft, &p, &json!({"snapshot":result})).is_err());
    draft.lines[0].qty = "0.5".into();
    assert!(draft.prepare(Command::Confirm, &scope, None).is_err());
    Ok(())
}
#[test]
fn material_ui_latest_version_requires_explicit_adoption_and_keeps_local_inputs()
-> Result<(), String> {
    let (draft, scope) = fixture()?;
    let first = draft.prepare(Command::Save, &scope, None)?;
    let mut draft = matched(&draft, &first, &json!({"snapshot":saved(&draft,&first)?}))?;
    draft.lines[0].qty = "0.750000".into();
    draft.changed = true;
    let mut latest = draft.saved.clone().ok_or("save")?;
    latest["version_no"] = json!(2);
    latest["lines"][0]["qty_micros"] = json!("500000");
    assert_eq!(draft.version(), 1);
    draft.adopt_latest(&latest)?;
    assert_eq!(draft.version(), 2);
    assert_eq!(draft.lines[0].qty, "0.750000");
    let next = draft.prepare(Command::Save, &scope, None)?;
    assert_eq!(next.body["expected_version"], 2);
    assert_eq!(next.body["lines"][0]["qty"], "0.750000");
    assert_ne!(next.request, first.request);
    Ok(())
}
