//! The workflow records rejected output before disposing of it as scrap.
use erp_domain::manufacturing::{
    FrozenInspectionLine, MaterialMove, MaterialMoveKind, OperationReport, ReceiptStatus,
};
use erp_domain::quantity::{ScaledQty, parse_qty};
use erp_repo::shared_contract::{SharedContractRepo, SharedWrite};
use erp_types::TenantId;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn record_reported_scrap(
    repo: &SharedContractRepo<'_>,
    pool: &PgPool,
    tenant: TenantId,
    work_order: Uuid,
) {
    let one = parse_qty("1").expect("one rejected item");
    let missing_report = repo
        .save_work_order_exception(
            tenant,
            "ex-scrap-missing",
            "ex-scrap-missing",
            work_order,
            "SCRAP",
            one,
            "defect destination",
            1_000,
            None,
            0,
        )
        .await;
    assert!(
        missing_report
            .expect_err("scrap requires actual rejected output")
            .to_string()
            .contains("scrap exceeds reported rejects")
    );

    let report = OperationReport {
        operation: "assemble".into(),
        good: ScaledQty::ZERO,
        rejected: one,
        rework: ScaledQty::ZERO,
        minutes: 20,
        operator_id: "worker-exception".into(),
        session_user_id: "login-user".into(),
        session_id: "session-exception".into(),
        crew_code: String::new(),
        started_at: String::new(),
        ended_at: String::new(),
        reverses_report_id: None,
    };
    let (write, _) = repo
        .save_operation_report(
            tenant,
            "ex-reject-report",
            "ex-reject-report",
            work_order,
            parse_qty("10").expect("planned output"),
            &report,
        )
        .await
        .expect("save rejected output")
        .expect("admit rejected output");
    let SharedWrite::Stored(stored_report) = write else {
        panic!("first rejected report must store");
    };
    let report_id = Uuid::parse_str(&stored_report.effect_id).expect("actual report id");
    let reported: (i64, i64) = sqlx::query_as(
        "select good_micros,rejected_micros from mfg_reports where tenant_id=$1 and report_id=$2 and work_order_id=$3",
    ).bind(tenant.0).bind(report_id).bind(work_order).fetch_one(pool).await.expect("stored rejected output");
    assert_eq!(reported, (0, 1_000_000));

    let write = repo
        .save_work_order_exception(
            tenant,
            "ex-scrap",
            "ex-scrap",
            work_order,
            "SCRAP",
            one,
            "defect destination",
            1_000,
            None,
            0,
        )
        .await
        .expect("scrap")
        .expect("admit scrap");
    let SharedWrite::Stored(stored) = write else {
        panic!("first scrap must store");
    };
    let replay = repo
        .save_work_order_exception(
            tenant,
            "ex-scrap",
            "ex-scrap",
            work_order,
            "SCRAP",
            one,
            "defect destination",
            1_000,
            None,
            0,
        )
        .await
        .expect("scrap replay")
        .expect("same original scrap");
    let SharedWrite::Replay(replayed) = replay else {
        panic!("scrap retry must replay");
    };
    assert_eq!(replayed, stored);
    let over = repo
        .save_work_order_exception(
            tenant,
            "ex-scrap-over",
            "ex-scrap-over",
            work_order,
            "SCRAP",
            one,
            "second defect destination",
            1_000,
            None,
            0,
        )
        .await;
    assert!(
        over.expect_err("already disposed rejected output cannot be scrapped twice")
            .to_string()
            .contains("scrap exceeds reported rejects")
    );
    let disposed: (i64, i64, i64) = sqlx::query_as(
        "select count(*)::bigint,coalesce(sum(qty_micros),0)::bigint,coalesce(sum(cost_cent),0)::bigint from mfg_exceptions where tenant_id=$1 and work_order_id=$2 and kind='SCRAP'",
    ).bind(tenant.0).bind(work_order).fetch_one(pool).await.expect("actual scrap destination");
    assert_eq!(disposed, (1, 1_000_000, 1_000));
    let refused_receipts: i64 = sqlx::query_scalar(
        "select count(*) from command_receipts where tenant_id=$1 and request_id in ('ex-scrap-missing','ex-scrap-over')",
    ).bind(tenant.0).fetch_one(pool).await.expect("refusals have no committed command");
    assert_eq!(refused_receipts, 0);
    eprintln!(
        "QA workflow_reported_scrap tenant={tenant} work_order={work_order} report={report_id} rejected=1000000 scrap=1000000 cost=1000 replay=true refused_receipts=0"
    );
}

pub(super) async fn inspect_and_release(
    repo: &SharedContractRepo<'_>,
    pool: &PgPool,
    tenant: TenantId,
    bom: Uuid,
    warehouse: Uuid,
    sku: Uuid,
) -> Uuid {
    let qty = |text| parse_qty(text).expect("quality fixture quantity");
    let write = repo
        .save_work_order(tenant, "wo-qc", "wo-qc", "WO-QC", "P", Some(bom), qty("2"))
        .await
        .expect("quality order")
        .expect("admit quality order");
    let SharedWrite::Stored(order) = write else {
        panic!("quality order must store");
    };
    let work_order = Uuid::parse_str(&order.effect_id).expect("actual quality work order");
    repo.transition_saved_work_order(tenant, "wo-qc-release", "qc-rel", work_order, "release")
        .await
        .expect("release work order")
        .expect("admit work order release");
    let lines = [
        FrozenInspectionLine {
            item_code: "width".into(),
            required: true,
            min_micros: 0,
            max_micros: 2_000_000,
            sample_count: 1,
        },
        FrozenInspectionLine {
            item_code: "length".into(),
            required: true,
            min_micros: 900_000,
            max_micros: 1_100_000,
            sample_count: 1,
        },
        FrozenInspectionLine {
            item_code: "length-pass".into(),
            required: true,
            min_micros: 900_000,
            max_micros: 1_100_000,
            sample_count: 1,
        },
    ];
    let (_, template, _) = repo
        .save_inspection_template(
            tenant,
            "qc-template",
            "qc-template",
            "STD-WORKFLOW-QC",
            "INCOMING",
            &lines,
        )
        .await
        .expect("quality standard")
        .expect("admit quality standard");
    let frozen = repo
        .freeze_inspection_template(
            tenant,
            "qc-freeze",
            "qc-freeze",
            template,
            "WORK_ORDER",
            work_order,
            "P",
            "BATCH-QC",
        )
        .await
        .expect("freeze original standard")
        .expect("admit original standard");
    let SharedWrite::Stored(frozen) = frozen else {
        panic!("quality freeze must store");
    };
    let freeze = Uuid::parse_str(&frozen.effect_id).expect("actual frozen standard");
    let blocked = repo
        .save_material_move(
            tenant,
            "qc-issue",
            "qc-issue",
            work_order,
            MaterialMove {
                kind: MaterialMoveKind::Issue,
                qty: qty("1"),
                supplement_reason_required: false,
            },
            None,
            Some(warehouse),
            Some(sku),
        )
        .await;
    assert!(
        blocked
            .expect_err("unread frozen inspection blocks issue")
            .to_string()
            .contains("incomplete")
    );
    let missing = repo
        .save_frozen_inspection_reading(
            tenant,
            "qc-missing",
            "qc-missing",
            freeze,
            "width",
            None,
            "inspector-1",
            "reviewer-1",
        )
        .await;
    assert!(
        missing
            .expect_err("missing required reading")
            .to_string()
            .contains("missing")
    );
    let out_of_range = repo
        .save_frozen_inspection_reading(
            tenant,
            "qc-bad",
            "qc-bad",
            freeze,
            "length",
            Some(2_000_000),
            "inspector-1",
            "reviewer-1",
        )
        .await;
    assert!(
        out_of_range
            .expect_err("reading outside original limits")
            .to_string()
            .contains("InvalidText")
    );
    for (request, item) in [
        ("qc-read", "length"),
        ("qc-width", "width"),
        ("qc-pass", "length-pass"),
    ] {
        let saved = repo
            .save_frozen_inspection_reading(
                tenant,
                request,
                request,
                freeze,
                item,
                Some(1_000_000),
                "inspector-1",
                "reviewer-1",
            )
            .await
            .expect("read frozen quality item")
            .expect("admit frozen reading");
        assert!(matches!(saved, SharedWrite::Stored(_)));
    }
    let report = OperationReport {
        operation: "inspect".into(),
        good: qty("1"),
        rejected: ScaledQty::ZERO,
        rework: ScaledQty::ZERO,
        minutes: 5,
        operator_id: "worker-qc".into(),
        session_user_id: "login-user".into(),
        session_id: "session-qc".into(),
        crew_code: String::new(),
        started_at: String::new(),
        ended_at: String::new(),
        reverses_report_id: None,
    };
    let (reported, _) = repo
        .save_operation_report(tenant, "rpt-qc", "rpt-qc", work_order, qty("2"), &report)
        .await
        .expect("qc report")
        .expect("admit qc report");
    let SharedWrite::Stored(reported) = reported else {
        panic!("qc report must store");
    };
    let report_id = Uuid::parse_str(&reported.effect_id).expect("actual qc report");
    let (held, _, available) = repo
        .save_production_receipt(
            tenant,
            "qc-hold",
            "qc-hold",
            work_order,
            qty("1"),
            ReceiptStatus::Hold,
            report_id,
            "BATCH-QC",
            Uuid::new_v4(),
            warehouse,
            sku,
        )
        .await
        .expect("held qc arrival")
        .expect("admit held qc arrival");
    assert_eq!(available, 0);
    let SharedWrite::Stored(held) = held else {
        panic!("qc hold must store");
    };
    let receipt = Uuid::parse_str(&held.effect_id).expect("actual qc receipt");
    let missing = repo
        .release_after_quality(tenant, "qc-rel-none", "qc-rel-none", receipt, " ")
        .await;
    assert!(
        missing
            .expect_err("inspector required")
            .to_string()
            .contains("inspector")
    );
    let approved = repo
        .release_after_quality(tenant, "qc-rel", "qc-rel", receipt, "inspector-1")
        .await
        .expect("approve quality")
        .expect("admit quality approval");
    let SharedWrite::Stored(approved) = approved else {
        panic!("approval must store");
    };
    let replay = repo
        .release_after_quality(tenant, "qc-rel", "qc-rel", receipt, "inspector-1")
        .await
        .expect("approval replay")
        .expect("same original approval");
    let SharedWrite::Replay(replayed) = replay else {
        panic!("approval must replay");
    };
    assert_eq!(replayed, approved);
    let facts: (i64, i64, i64, String) = sqlx::query_as(
        "select (select count(*) from mfg_quality_items where tenant_id=$1 and freeze_id=$2 and passed and reading_present), (select count(*) from mfg_quality_release_proofs where tenant_id=$1 and receipt_id=$3), available_micros,status from mfg_receipts where tenant_id=$1 and receipt_id=$3",
    ).bind(tenant.0).bind(freeze).bind(receipt).fetch_one(pool).await.expect("original frozen proof and physical state");
    assert_eq!(
        facts,
        (3, 1, 0, "HOLD".into()),
        "quality approval retains physical hold until a separate stock release"
    );
    eprintln!(
        "QA workflow_frozen_quality tenant={tenant} work_order={work_order} receipt={receipt} freeze={freeze} readings=3 proofs=1 available=0 status=HOLD replay=true"
    );
    work_order
}
