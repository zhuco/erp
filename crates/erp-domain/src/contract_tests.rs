use crate::access::{
    AccessError, ActionAccess, TenantAccessPlan, admit_login_seat, admit_session,
    batch_execution_allowed, session_command_allowed,
};
use crate::quantity::{ScaledQty, parse_qty};
use crate::receipt::{
    CommandStatus, ReceiptError, admit_retry, backup_age_seconds, cancel_at_checkpoint,
    client_unknown_is_not_failure, duplicate_delivery_keeps_effect, lost_response_retry_decision,
    read_cancels_same_scope, refused_work_stays_queryable, restored_task_does_not_repost,
    rollback_keeps_fact, succeeded,
};
use crate::trade::{TradeLine, close_trade};

#[test]
fn batch_execution_rechecks_permission_and_frozen_warehouse() {
    let access = ActionAccess {
        actor_user_id: "user-o05b".into(),
        permission: "stock.read".into(),
        warehouse_id: "wh-o05b".into(),
        granted: vec!["stock.read".into()],
        warehouses: vec!["wh-o05b".into()],
        platform_operator: false,
    };
    assert!(batch_execution_allowed(&access, "wh-o05b").is_ok());
    assert!(batch_execution_allowed(&access, "other").is_err());
}

#[test]
fn second_active_login_is_rejected_and_sessions_are_separate() {
    let plan = TenantAccessPlan::free_multi();
    assert!(admit_login_seat(plan, 0).is_ok());
    assert_eq!(
        admit_login_seat(plan, 1),
        Err(AccessError::SeatLimitReached)
    );
    assert!(admit_session(plan, 1).is_ok());
    assert!(session_command_allowed(true, true, 2, 2).is_ok());
    assert_eq!(
        session_command_allowed(false, true, 2, 2),
        Err(AccessError::SessionInactive)
    );
    assert_eq!(
        session_command_allowed(true, true, 2, 3),
        Err(AccessError::CredentialsStale)
    );
}

#[test]
fn lost_response_only_reads_the_stored_status() {
    assert_eq!(
        client_unknown_is_not_failure(Some(CommandStatus::Succeeded)),
        CommandStatus::Succeeded
    );
    assert_eq!(
        lost_response_retry_decision(CommandStatus::Accepted, true),
        Ok("LOOKUP")
    );
    assert_eq!(
        lost_response_retry_decision(CommandStatus::Accepted, false),
        Ok("LOOKUP")
    );
    assert_eq!(
        lost_response_retry_decision(CommandStatus::Succeeded, false),
        Ok("KEEP")
    );
}

#[test]
fn backup_age_and_refused_work_use_stored_facts() {
    assert_eq!(backup_age_seconds(1_000, 2_000), Ok(1_000));
    assert!(refused_work_stays_queryable("429", "cmd-o08b").is_ok());
    assert!(refused_work_stays_queryable("QUEUED", "cmd-o08b").is_err());
}

#[test]
fn rollback_does_not_replace_a_committed_fact() {
    assert!(rollback_keeps_fact(2_500, 1_000_000, 2_500, 1_000_000).is_ok());
    assert!(rollback_keeps_fact(2_500, 1_000_000, 2_400, 1_000_000).is_err());
}

#[test]
fn a_restored_posted_task_is_not_posted_again() {
    assert_eq!(restored_task_does_not_repost(true), "KEEP");
    assert_eq!(restored_task_does_not_repost(false), "POST");
}

#[test]
fn a_new_read_cancels_only_the_same_scope() {
    assert!(read_cancels_same_scope("orders", "orders", 1, 2, false));
    assert!(!read_cancels_same_scope("orders", "stock", 1, 2, false));
    assert!(!read_cancels_same_scope("orders", "orders", 1, 2, true));
}

#[test]
fn posted_issue_needs_a_reversal_and_a_repeat_keeps_the_effect() {
    assert_eq!(cancel_at_checkpoint(true), "REVERSE_REQUIRED");
    assert_eq!(
        duplicate_delivery_keeps_effect(Some("eff-o03b")),
        Ok("KEEP")
    );
    assert!(duplicate_delivery_keeps_effect(None).is_err());
}

#[test]
fn quantity_strings_round_trip_and_legacy_one_stays_one() {
    assert_eq!(parse_qty("0.125").unwrap().as_micros(), 125_000);
    assert_eq!(parse_qty("4.5").unwrap().as_micros(), 4_500_000);
    assert_eq!(
        ScaledQty::from_legacy_whole_units(1).unwrap().as_micros(),
        1_000_000
    );
}

#[test]
fn keyset_trace_page_marks_a_short_page_and_a_node_cap_incomplete() {
    let rows = [
        crate::manufacturing::IndexedEdge {
            edge_id: "a".into(),
            edge: crate::manufacturing::GenealogyEdge {
                source_action: "DELIVER".into(),
                input_code: "SER".into(),
                output_code: "FG".into(),
                qty: ScaledQty::from_micros(1),
                work_order: "WO".into(),
                output_batch: "B".into(),
                customer: "C1".into(),
                reversed: false,
            },
        },
        crate::manufacturing::IndexedEdge {
            edge_id: "b".into(),
            edge: crate::manufacturing::GenealogyEdge {
                source_action: "DELIVER".into(),
                input_code: "SER".into(),
                output_code: "FG".into(),
                qty: ScaledQty::from_micros(1),
                work_order: "WO".into(),
                output_batch: "B".into(),
                customer: "C2".into(),
                reversed: false,
            },
        },
    ];
    let short = crate::manufacturing::keyset_genealogy_page(&rows, 1, 1, false).expect("short");
    assert!(short.incomplete);
    assert_eq!(short.next_edge_id.as_deref(), Some("a"));
    assert_eq!(short.edges[0].customer, "C1");
    let capped =
        crate::manufacturing::keyset_genealogy_page(&rows[..1], 10, 1, true).expect("capped");
    assert!(capped.incomplete);
    assert_eq!(capped.loaded, 1);
}

#[test]
fn cited_disposition_keeps_version_and_does_not_relabel_only() {
    let first = crate::manufacturing::admit_cited_disposition(
        crate::manufacturing::CitedDispositionInput {
            held: ScaledQty::from_micros(10_000_000),
            kind: "ACCEPT",
            qty: ScaledQty::from_micros(8_000_000),
            reason: "",
            authorizer: "",
            inspection_id: "hold-1",
            batch_code: "BATCH-Q02",
            location_id: "loc-1",
            prior_version: 0,
            unit_cost_cent: 0,
        },
    )
    .expect("accept");
    assert_eq!(first.held_left.as_micros(), 2_000_000);
    assert_eq!(first.quality_status, "GOOD");
    assert_eq!(first.version_no, 1);
    let second = crate::manufacturing::admit_cited_disposition(
        crate::manufacturing::CitedDispositionInput {
            held: first.held_left,
            kind: "ISOLATE",
            qty: ScaledQty::from_micros(2_000_000),
            reason: "defect",
            authorizer: "",
            inspection_id: "hold-1",
            batch_code: "BATCH-Q02",
            location_id: "loc-1",
            prior_version: first.version_no,
            unit_cost_cent: 0,
        },
    )
    .expect("isolate");
    assert_eq!(second.held_left.as_micros(), 0);
    assert_eq!(second.quality_status, "ISOLATED");
    assert_eq!(second.version_no, 2);
    assert!(
        crate::manufacturing::admit_cited_disposition(
            crate::manufacturing::CitedDispositionInput {
                held: first.held_left,
                kind: "ISOLATE",
                qty: ScaledQty::from_micros(1_000_000),
                reason: "defect",
                authorizer: "",
                inspection_id: "",
                batch_code: "BATCH-Q02",
                location_id: "loc-1",
                prior_version: 1,
                unit_cost_cent: 0,
            }
        )
        .is_err()
    );
}

#[test]
fn atomic_action_posts_once() {
    assert!(crate::manufacturing::atomic_effect_already_posted(None).is_ok());
    assert!(crate::manufacturing::atomic_effect_already_posted(Some("doc-1")).is_err());
}

#[test]
fn saved_report_restore_keeps_owner_and_query() {
    assert!(crate::trade::restore_saved_report("user-r05b", "user-r05b").is_ok());
    assert!(crate::trade::restore_saved_report("user-r05b", "user-other").is_err());
    assert!(
        crate::trade::same_saved_query(
            "operating|2026-10-01|2026-10-31",
            "operating|2026-10-01|2026-10-31"
        )
        .is_ok()
    );
    assert!(
        crate::trade::same_saved_query(
            "operating|2026-10-01|2026-10-31",
            "operating|2026-11-01|2026-11-30"
        )
        .is_err()
    );
}

#[test]
fn retained_task_effect_survives_a_success_banner() {
    let kept =
        crate::manufacturing::retained_task_effect("effect-r04b", "refresh ok").expect("kept");
    assert_eq!(kept, "effect-r04b");
    assert!(crate::manufacturing::retained_task_effect("", "refresh ok").is_err());
    assert!(crate::manufacturing::retained_task_effect("refresh ok", "refresh ok").is_err());
}

#[test]
fn export_uses_the_frozen_snapshot_and_keeps_a_cancelled_read() {
    let frozen = vec!["so-r03b".into(), "po-r03b".into(), "exclude-r03b".into()];
    let rows = crate::trade::export_snapshot_rows(&frozen, &["exclude-r03b".into()]).expect("rows");
    assert_eq!(rows, vec!["po-r03b", "so-r03b"]);
    assert!(
        crate::trade::export_snapshot_rows(&["exclude-r03b".into()], &["exclude-r03b".into()])
            .is_err()
    );
    assert!(crate::trade::export_download_allowed("user-r03b", "user-r03b", false, true).unwrap());
    assert!(
        crate::trade::export_download_allowed("user-r03b", "user-other", false, false).is_err()
    );
    assert!(crate::trade::export_download_allowed("user-r03b", "user-r03b", true, false).is_err());
}

#[test]
fn manufacturing_metrics_stay_separate() {
    assert_eq!(
        crate::manufacturing::manufacturing_report_metric("ISSUE", "2026-10-31").unwrap(),
        "ISSUE"
    );
    assert_eq!(
        crate::manufacturing::manufacturing_report_metric("CONSUME", "2026-10-31").unwrap(),
        "CONSUME"
    );
    assert_ne!(
        crate::manufacturing::manufacturing_report_metric("PLAN", "2026-10-31").unwrap(),
        crate::manufacturing::manufacturing_report_metric("RECEIPT", "2026-10-31").unwrap()
    );
    assert!(crate::manufacturing::manufacturing_report_metric("PLAN", " ").is_err());
}

#[test]
fn operating_page_cannot_replace_the_authorized_range() {
    assert!(crate::trade::page_cannot_replace_range(1, 3).is_err());
    assert!(crate::trade::page_cannot_replace_range(0, 3).is_ok());
    assert!(crate::trade::page_cannot_replace_range(3, 3).is_ok());
}

#[test]
fn component_variance_splits_price_usage_rate_and_keeps_unknown_overhead() {
    use crate::manufacturing::{
        ActualCostDriver, StandardCostDriver, component_variances, confirmed_margin,
    };
    let standard = StandardCostDriver {
        material_qty_micros: 10_000_000,
        material_price_cent: 3_000,
        labor_minutes: 60,
        labor_rate_cent: 200,
        overhead_rate_cent: 100,
    };
    let actual = ActualCostDriver {
        material_qty_micros: 5_000_000,
        material_price_cent: 6_200,
        labor_minutes: 50,
        labor_rate_cent: 250,
        overhead_cent: Some(6_500),
    };
    let (lines, standard_total, actual_total, estimated) =
        component_variances(&standard, &actual).expect("split");
    assert!(!estimated);
    assert_eq!((standard_total, actual_total), (48_000, 50_000));
    let variance: i64 = lines.iter().map(|line| line.variance_cent).sum();
    assert_eq!(variance, actual_total - standard_total);
    assert_ne!(
        lines.iter().map(|line| line.standard_cent).sum::<i64>(),
        standard_total
    );
    let grouped = |prefix: &str| {
        lines
            .iter()
            .filter(|line| line.kind.starts_with(prefix))
            .map(|line| line.variance_cent)
            .sum::<i64>()
    };
    assert_eq!(grouped("MATERIAL"), 1_000);
    assert_eq!(grouped("LABOR"), 500);
    assert_eq!(grouped("OVERHEAD"), 500);
    assert_eq!(
        lines
            .iter()
            .find(|line| line.kind == "MATERIAL_PRICE")
            .map(|line| line.variance_cent),
        Some(32_000)
    );
    assert_eq!(
        lines
            .iter()
            .find(|line| line.kind == "MATERIAL_USAGE")
            .map(|line| line.variance_cent),
        Some(-31_000)
    );
    assert_eq!(
        lines
            .iter()
            .find(|line| line.kind == "LABOR_RATE")
            .map(|line| line.variance_cent),
        Some(3_000)
    );
    assert_eq!(
        lines
            .iter()
            .find(|line| line.kind == "LABOR_EFFICIENCY")
            .map(|line| line.variance_cent),
        Some(-2_500)
    );
    assert_eq!(
        confirmed_margin(60_000, actual_total, estimated).unwrap(),
        Some(10_000)
    );
    let unknown = ActualCostDriver {
        overhead_cent: None,
        ..actual
    };
    let (unknown_lines, _, _, unknown_estimated) =
        component_variances(&standard, &unknown).expect("unknown");
    assert!(unknown_estimated);
    assert!(
        unknown_lines
            .iter()
            .any(|line| line.kind == "OVERHEAD" && line.variance_cent == 0)
    );
    assert_eq!(
        confirmed_margin(60_000, 49_500, unknown_estimated).unwrap(),
        None
    );
}

#[test]
fn invoice_entry_is_not_official_and_aging_uses_settlements() {
    assert!(!crate::trade::invoice_stays_unofficial(true));
    let aged =
        crate::trade::aging_from_settlements(Some("2026-08-01"), "2026-10-01", 20_000, &[5_000])
            .expect("aged")
            .expect("open");
    assert_eq!(aged, ("61-90", 15_000));
    assert!(
        crate::trade::aging_from_settlements(Some("2026-09-20"), "2026-10-01", 3_000, &[3_000])
            .unwrap()
            .is_none()
    );
    assert_eq!(
        crate::trade::aging_from_settlements(None, "2026-10-01", 4_000, &[]).unwrap(),
        Some(("UNDATED", 4_000))
    );
}

#[test]
fn posted_cost_facts_rebuild_the_cutoff_and_a_difference_keeps_the_ledger() {
    let facts = [
        crate::trade::PostedCostFact {
            bucket: "INVENTORY".into(),
            amount_cent: 70_000,
            source_ref: "inv-1".into(),
        },
        crate::trade::PostedCostFact {
            bucket: "FINISHED".into(),
            amount_cent: 49_000,
            source_ref: "fg-1".into(),
        },
        crate::trade::PostedCostFact {
            bucket: "FINISHED".into(),
            amount_cent: 0,
            source_ref: "fg-2".into(),
        },
    ];
    let summed = crate::trade::sum_posted_cost_facts(&facts).expect("sum");
    assert_eq!(summed[0], ("INVENTORY", 70_000, "inv-1".into()));
    assert_eq!(summed[1].1, 0);
    assert_eq!(summed[2].1, 49_000);
    assert_eq!(
        crate::trade::difference_does_not_clear(50_000, 49_000).unwrap(),
        49_000
    );
    assert!(crate::trade::difference_does_not_clear(50_000, 0).is_err());
}

#[test]
fn landed_cost_uses_the_stored_tax_rule_and_a_confirmed_purchase() {
    assert!(crate::trade::admit_tax_cost_rule(None).is_err());
    assert!(
        !crate::trade::admit_tax_cost_rule(Some(false))
            .unwrap()
            .tax_is_cost
    );
    assert!(crate::trade::admit_landed_source("DRAFT", "PURCHASE").is_err());
    assert!(crate::trade::admit_landed_source("CONFIRMED", "SALES").is_err());
    assert!(crate::trade::admit_landed_source("CONFIRMED", "PURCHASE").is_ok());
    assert_eq!(
        crate::trade::expense_cost_cent(11_300, 1_300, false).unwrap(),
        10_000
    );
    assert_eq!(
        crate::trade::expense_cost_cent(11_300, 1_300, true).unwrap(),
        11_300
    );
}

#[test]
fn cash_open_amount_is_rebuilt_from_settlements_not_a_paid_flag() {
    let (settled, open) =
        crate::trade::rebuild_settled_open(45_000, &[40_000, 2_000, -2_000], true)
            .expect("rebuild");
    assert_eq!(settled, 40_000);
    assert_eq!(open, 5_000);
    assert_eq!(
        crate::trade::return_receipt_offset(5_000, 5_000).unwrap(),
        0
    );
    assert!(crate::trade::rebuild_settled_open(45_000, &[50_000], false).is_err());
    assert!(
        !crate::trade::admit_cash_entry("RECEIPT", "CUST", "CNY", "CASH", 40_000)
            .unwrap()
            .bank_executed
    );
}

#[test]
fn confirmed_document_rebuilds_open_balance_and_keeps_an_advance_apart() {
    assert!(crate::trade::document_balance_kind("DRAFT", "SALES").is_err());
    assert_eq!(
        crate::trade::document_balance_kind("CONFIRMED", "SALES").unwrap(),
        crate::trade::PartnerBalanceKind::Receivable
    );
    assert_eq!(
        crate::trade::document_balance_kind("CONFIRMED", "PURCHASE").unwrap(),
        crate::trade::PartnerBalanceKind::Payable
    );
    assert_eq!(
        crate::trade::document_balance_kind("CONFIRMED", "CREDIT").unwrap(),
        crate::trade::PartnerBalanceKind::Receivable
    );
    assert!(crate::trade::document_balance_kind("DRAFT", "CREDIT").is_err());
    let rebuilt = crate::trade::rebuild_open_balance(
        "so-f01",
        crate::trade::PartnerBalanceKind::Receivable,
        20_000,
        5_000,
    )
    .expect("rebuild");
    assert_eq!(rebuilt.open_cent, 15_000);
    assert!(
        crate::trade::rebuild_open_balance(
            "rc-1",
            crate::trade::PartnerBalanceKind::AdvanceReceipt,
            5_000,
            0,
        )
        .is_err()
    );
    assert_eq!(
        crate::trade::admit_advance(
            crate::trade::PartnerBalanceKind::AdvanceReceipt,
            5_000,
            "CNY",
            "CUST-F01",
        )
        .unwrap(),
        5_000
    );
}

#[test]
fn cited_sampling_rule_judges_the_sample_and_a_short_recall_page_stays_incomplete() {
    let rule = crate::manufacturing::admit_sampling_rule("AQL-Q04", 1, 20, 1).expect("rule");
    let accepted = crate::manufacturing::admit_sampling_against_rule(&rule, 1).expect("accept");
    assert!(accepted.accepted);
    assert_eq!(accepted.accept_number, 1);
    assert!(
        !crate::manufacturing::admit_sampling_against_rule(&rule, 2)
            .unwrap()
            .accepted
    );
    assert!(crate::manufacturing::sampling_period_is_new(&["2026-09"], "2026-09").is_err());
    assert!(crate::manufacturing::sampling_period_is_new(&["2026-09"], "2026-10").is_ok());
    let short =
        crate::manufacturing::recall_scope_from_page(&["FG-A".into()], &["C1".into()], false, true)
            .expect("short page");
    assert!(short.incomplete);
    assert!(crate::manufacturing::admit_complete_recall_page(&short).is_err());
    assert_eq!(
        crate::manufacturing::recall_freeze_ledger("freeze", "DONE", 3_000_000).unwrap(),
        Some(3_000_000)
    );
    assert!(
        crate::manufacturing::recall_freeze_ledger("notice", "FAILED", 0)
            .unwrap()
            .is_none()
    );
}

#[test]
fn frozen_inspection_keeps_the_cited_range_and_zero_is_a_reading() {
    let line = crate::manufacturing::FrozenInspectionLine {
        item_code: "width".into(),
        required: true,
        min_micros: 900_000,
        max_micros: 1_100_000,
        sample_count: 2,
    };
    let frozen = crate::manufacturing::freeze_inspection_template(
        "WORK_ORDER",
        "wo-1",
        "FG-1",
        "BATCH-1",
        1,
        &[line],
    )
    .expect("freeze");
    assert_eq!(frozen[0].max_micros, 1_100_000);
    assert!(
        crate::manufacturing::admit_inspection_reading(
            "width",
            true,
            900_000,
            1_100_000,
            Some(2_000_000)
        )
        .is_err()
    );
    assert_eq!(
        crate::manufacturing::admit_inspection_reading("length", true, 0, 2_000_000, Some(0))
            .unwrap(),
        0
    );
    assert!(
        crate::manufacturing::admit_inspection_reading("width", true, 900_000, 1_100_000, None)
            .is_err()
    );
    assert!(!crate::manufacturing::reading_passes(
        true, 900_000, 1_100_000, None
    ));
}

#[test]
fn trade_sample_and_retry_do_not_double_post() {
    let close = close_trade(
        TradeLine {
            qty: parse_qty("50").unwrap(),
            unit_cost_cent: 2_000,
            unit_price_cent: 3_000,
        },
        parse_qty("20").unwrap(),
        parse_qty("5").unwrap(),
        40_000,
        70_000,
    )
    .unwrap();
    assert_eq!(close.ending_qty, parse_qty("35").unwrap());
    assert_eq!(close.cogs_cent, 30_000);
    let stored = succeeded("trade-1", "payload", "effect-1");
    let replay = admit_retry(Some(&stored), "trade-1", "payload")
        .unwrap()
        .unwrap();
    assert_eq!(replay.effect_id, stored.effect_id);
    assert_eq!(
        admit_retry(Some(&stored), "trade-1", "other"),
        Err(ReceiptError::PayloadConflict)
    );
}
