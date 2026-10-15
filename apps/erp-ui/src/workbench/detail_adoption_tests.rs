use super::detail_state::{DetailField, DetailSaveOutcome, DetailSavePhase, FormState};
const SCOPE: &str = "service/tenant/actor/adoption";
fn fields(name: &str) -> Vec<DetailField> {
    vec![
        DetailField {
            id: "name".into(),
            label: "Name".into(),
            value: name.into(),
            editable: true,
            required: true,
        },
        DetailField {
            id: "note".into(),
            label: "Note".into(),
            value: String::new(),
            editable: true,
            required: false,
        },
        DetailField {
            id: "phone".into(),
            label: "Phone".into(),
            value: "UNCHANGED_PRIVATE_PHONE".into(),
            editable: false,
            required: false,
        },
    ]
}
fn fresh() -> FormState {
    let mut state = FormState::new(SCOPE.into());
    state
        .bind("one", "v7", &fields("current server value"))
        .expect("bind current");
    state
}
fn source() -> FormState {
    let mut state = FormState::new(SCOPE.into());
    state.bind("one", "v0", &fields("old")).expect("old bind");
    state.draft.insert("name".into(), "recovered draft".into());
    let bytes = state.checkpoint().expect("encode").expect("draft");
    assert!(!String::from_utf8_lossy(&bytes).contains("UNCHANGED_PRIVATE_PHONE"));
    FormState::restore(SCOPE, &bytes).expect("restore no old authority")
}
fn assert_unchanged(current: &FormState, before: &FormState) {
    assert_eq!(current.draft, before.draft);
    assert_eq!(current.base, before.base);
    assert_eq!(current.version, before.version);
    assert_eq!(current.record_id, before.record_id);
    assert_eq!(current.pending_request(), before.pending_request());
    assert_eq!(
        current.checkpoint().expect("after"),
        before.checkpoint().expect("before")
    );
}
#[test]
fn detail_adoption_old_backup_uses_current_values_version_and_metadata() {
    let copy = source();
    let bytes = copy.checkpoint().expect("before");
    let mut current = fresh();
    let base = current.base.clone();
    current.adopt_backup(&copy).expect("explicit adoption");
    assert_eq!(current.version, "v7");
    assert_eq!(current.base, base);
    assert_eq!(current.draft["phone"], "UNCHANGED_PRIVATE_PHONE");
    assert_eq!(current.draft["name"], "recovered draft");
    assert!(
        current.pending_request().is_none(),
        "adoption must not prepare a send"
    );
    let request = current.begin_save().expect("new intent");
    assert_eq!(request.observed_version, "v7");
    assert_eq!(request.before["name"], "current server value");
    assert_eq!(request.changes["name"], "recovered draft");
    assert!(!request.changes.contains_key("phone"));
    assert_eq!(copy.checkpoint().expect("after"), bytes);
}
#[test]
fn detail_adoption_wrong_actor_or_record_preserves_current_and_backup() {
    for mismatch in ["scope", "record"] {
        let mut copy = source();
        if mismatch == "scope" {
            copy.scope = "another actor".into();
        } else {
            copy.record_id = Some("other".into());
        }
        let bytes = copy.checkpoint().expect("source");
        let mut current = fresh();
        let before = current.clone();
        assert!(current.adopt_backup(&copy).is_err());
        assert_unchanged(&current, &before);
        assert_eq!(copy.checkpoint().expect("source retained"), bytes);
    }
}
#[test]
fn detail_adoption_current_draft_or_pending_never_gets_overwritten() {
    for pending in [false, true] {
        let mut current = fresh();
        current
            .draft
            .insert("name".into(), "my active draft".into());
        if pending {
            current.begin_save().expect("own intent");
        }
        let before = current.clone();
        assert!(current.adopt_backup(&source()).is_err());
        assert_unchanged(&current, &before);
    }
}
#[test]
fn detail_adoption_source_original_checkpointing_unknown_or_reading_is_blocked() {
    let mut copy = source();
    copy.bind("one", "v0", &fields("old"))
        .expect("current source authority");
    let request = copy.begin_save().expect("source request");
    let bytes = copy.checkpoint().expect("copy").expect("some");
    for phase in [
        DetailSavePhase::Checkpointing,
        DetailSavePhase::Unknown,
        DetailSavePhase::Reading,
    ] {
        let mut candidate = if phase == DetailSavePhase::Checkpointing {
            copy.clone()
        } else {
            FormState::restore(SCOPE, &bytes).expect("unknown restore")
        };
        if phase == DetailSavePhase::Reading {
            assert_eq!(candidate.begin_read().expect("lookup"), request);
        }
        let mut current = fresh();
        let before = current.clone();
        assert!(current.adopt_backup(&candidate).is_err());
        assert_unchanged(&current, &before);
        assert_eq!(candidate.pending_request(), Some(&request));
    }
}
#[test]
fn detail_adoption_storage_ime_invalid_binding_and_version_conflict_are_blocked() {
    for reason in ["storage", "ime", "binding", "version"] {
        let mut current = fresh();
        match reason {
            "storage" => current.storage_ready = false,
            "ime" => current.composing = true,
            "binding" => {
                let mut invalid = fields("current server value");
                invalid.push(invalid[0].clone());
                assert!(current.bind("one", "v7", &invalid).is_err());
            }
            _ => current.require_latest(),
        }
        let before = current.clone();
        assert!(current.adopt_backup(&source()).is_err());
        assert_unchanged(&current, &before);
    }
}
#[test]
fn detail_adoption_requires_rebind_after_current_checkpoint_restoration() {
    let mut original = fresh();
    original.draft.insert("name".into(), "was submitted".into());
    let request = original.begin_save().expect("intent");
    let bytes = original.checkpoint().expect("encode").expect("some");
    let mut current = FormState::restore(SCOPE, &bytes).expect("restore");
    current.begin_read().expect("lookup");
    current
        .resolve(
            &request,
            DetailSaveOutcome::Committed {
                version: "v8".into(),
                values: [("name".into(), "current v8".into())].into_iter().collect(),
            },
        )
        .expect("known result");
    assert!(!current.dirty());
    let before = current.clone();
    assert!(current.adopt_backup(&source()).is_err());
    assert_unchanged(&current, &before);
    current
        .bind("one", "v8", &fields("current v8"))
        .expect("fresh metadata");
    current
        .adopt_backup(&source())
        .expect("explicit adoption after rebind");
    assert_eq!(current.version, "v8");
}
#[test]
fn detail_adoption_readonly_or_removed_field_rejects_entire_multifield_import() {
    let mut copy = source();
    copy.base.insert("note".into(), String::new());
    copy.draft.insert("note".into(), "recovered note".into());
    let bytes = copy.checkpoint().expect("multifield source").expect("some");
    let copy = FormState::restore(SCOPE, &bytes).expect("valid multifield checkpoint");
    for removed in [false, true] {
        let mut current = fresh();
        let mut metadata = fields("current server value");
        if removed {
            metadata.retain(|field| field.id != "note");
        } else {
            metadata[1].editable = false;
        }
        current.bind("one", "v7", &metadata).expect("new authority");
        let before = current.clone();
        assert!(current.adopt_backup(&copy).is_err());
        assert_unchanged(&current, &before);
    }
}
#[test]
fn detail_adoption_invalid_or_oversized_values_keep_both_drafts() {
    for value in [String::new(), "x".repeat(4097)] {
        let mut copy = source();
        copy.draft.insert("name".into(), value);
        let bytes = copy.checkpoint().expect("recoverable invalid draft");
        let mut current = fresh();
        let before = current.clone();
        assert!(current.adopt_backup(&copy).is_err());
        assert_unchanged(&current, &before);
        assert_eq!(copy.checkpoint().expect("kept"), bytes);
    }
    let mut current = fresh();
    let mut metadata = fields("current server value");
    metadata[1].required = true;
    current
        .bind("one", "v7", &metadata)
        .expect("required unchanged field");
    let before = current.clone();
    assert!(current.adopt_backup(&source()).is_err());
    assert_unchanged(&current, &before);
}
#[test]
fn detail_adoption_rejected_original_can_adopt_only_after_known_result_with_new_identity() {
    let mut copy = source();
    copy.bind("one", "v0", &fields("old"))
        .expect("source metadata");
    let old = copy.begin_save().expect("old identity");
    let mut current = fresh();
    assert!(current.adopt_backup(&copy).is_err());
    copy.resolve(
        &old,
        DetailSaveOutcome::Rejected {
            code: "VERSION_CONFLICT".into(),
            message: "changed".into(),
        },
    )
    .expect("known authoritative rejection");
    current
        .adopt_backup(&copy)
        .expect("explicit current-version adoption");
    let new = current.begin_save().expect("new intent");
    assert_ne!(new.operation_id, old.operation_id);
    assert_eq!(new.observed_version, "v7");
    assert_eq!(new.before["name"], "current server value");
    assert_eq!(
        copy.persistence_write()
            .expect("source proof retained")
            .resolved,
        Some(old)
    );
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn detail_adoption_actual_sqlite_new_primary_request_does_not_replace_backup() {
    use super::detail::DetailPanel;
    use super::detail_storage::{DetailStore, wait_storage};
    let ctx = eframe::egui::Context::default();
    let root = std::env::current_dir()
        .expect("cwd")
        .join("qa/acceptance/detail-durable-native")
        .join(uuid::Uuid::new_v4().to_string());
    let mut copy = DetailPanel::new(SCOPE.into());
    copy.bind("one", "v0", &fields("old"));
    copy.edit_value("name", "recovered draft".into())
        .expect("source edit");
    let old = copy.begin_save().expect("old request");
    let backup = wait_storage(
        DetailStore::open_backup_at(root.clone(), SCOPE.into(), &ctx).expect("backup open"),
    )
    .expect("backup");
    let backup_receipt = wait_storage(
        backup
            .store
            .commit(0, copy.persistence_write().expect("old write"), &ctx)
            .expect("save backup"),
    )
    .expect("backup readback");
    let before = copy.checkpoint().expect("source before");
    let mut original = DetailPanel::restore(SCOPE, before.as_ref().expect("stored bytes"))
        .expect("original read model");
    original.begin_read().expect("lookup");
    original
        .resolve_save(
            &old,
            DetailSaveOutcome::Rejected {
                code: "VERSION_CONFLICT".into(),
                message: "changed".into(),
            },
        )
        .expect("known rejection");
    let mut current = DetailPanel::new(SCOPE.into());
    current.bind("one", "v8", &fields("current v8"));
    current.adopt_backup(&original).expect("explicit adoption");
    assert!(current.pending_request().is_none());
    let new = current.begin_save().expect("new identity");
    assert_ne!(new.operation_id, old.operation_id);
    assert!(current.authorize_send(&new, &backup_receipt).is_err());
    let primary =
        wait_storage(DetailStore::open_at(root.clone(), SCOPE.into(), &ctx).expect("primary open"))
            .expect("primary");
    let receipt = wait_storage(
        primary
            .store
            .commit(0, current.persistence_write().expect("new write"), &ctx)
            .expect("commit new"),
    )
    .expect("primary readback");
    current
        .authorize_send(&new, &receipt)
        .expect("only matching primary may authorize");
    let reread = wait_storage(
        DetailStore::open_at(root.clone(), SCOPE.into(), &ctx).expect("independent primary"),
    )
    .expect("new primary");
    let restored = DetailPanel::restore(SCOPE, reread.bytes.as_ref().expect("new bytes"))
        .expect("restore new");
    assert_eq!(restored.pending_request(), Some(&new));
    assert_eq!(new.observed_version, "v8");
    assert_eq!(new.before["name"], "current v8");
    let old_readback = wait_storage(
        DetailStore::open_backup_at(root, SCOPE.into(), &ctx).expect("independent backup"),
    )
    .expect("old backup");
    assert_eq!(old_readback.revision, 1);
    assert_eq!(old_readback.bytes, before);
}

#[test]
fn detail_adoption_already_applied_values_do_not_prepare_or_change_current() {
    let copy = source();
    let mut current = fresh();
    current
        .bind("one", "v8", &fields("recovered draft"))
        .expect("already applied current data");
    let before = current.clone();
    assert!(current.adopt_backup(&copy).is_err());
    assert_unchanged(&current, &before);
    assert!(current.pending_request().is_none());
    assert!(!current.dirty());
}
