use super::detail_state::{
    DetailField, DetailSaveOutcome, DetailSavePhase, DetailSaveRequest, FormState,
};

fn fields(name: &str) -> Vec<DetailField> {
    vec![
        DetailField {
            id: "name".into(),
            label: "名称".into(),
            value: name.into(),
            editable: true,
            required: true,
        },
        DetailField {
            id: "note".into(),
            label: "备注".into(),
            value: String::new(),
            editable: true,
            required: false,
        },
        DetailField {
            id: "phone".into(),
            label: "手机号".into(),
            value: "PRIVATE_UNCHANGED_PHONE".into(),
            editable: false,
            required: false,
        },
    ]
}
fn state() -> FormState {
    let mut state = FormState::new("service/tenant/actor/page".into());
    state.bind("one", "v0", &fields("old")).expect("bind");
    state.draft.insert("name".into(), "draft".into());
    state
}
fn committed(version: &str, name: &str) -> DetailSaveOutcome {
    DetailSaveOutcome::Committed {
        version: version.into(),
        values: [("name".into(), name.into())].into_iter().collect(),
    }
}

#[test]
fn detail_save_unknown_blocks_duplicate_reset_and_binding() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    assert!(state.begin_save().is_err());
    assert!(state.reset().is_err());
    assert!(state.bind("other", "v0", &fields("other")).is_err());
    state
        .resolve(&request, DetailSaveOutcome::Unknown("response lost".into()))
        .expect("unknown");
    assert_eq!(state.phase(), Some(DetailSavePhase::Unknown));
    assert!(state.begin_save().is_err());
    assert!(state.reset().is_err());
    assert_eq!(state.begin_read().expect("original lookup"), request);
    assert!(state.begin_read().is_err());
    state
        .resolve(&request, committed("v1", "normalized draft"))
        .expect("original stored result");
    assert!(!state.dirty());
    assert_eq!(
        state.draft.get("name").map(String::as_str),
        Some("normalized draft")
    );
}

#[test]
fn detail_save_late_old_receipt_cannot_clear_newer_draft() {
    let mut state = state();
    let first = state.begin_save().expect("first");
    state
        .resolve(
            &first,
            DetailSaveOutcome::Rejected {
                code: "VALIDATION".into(),
                message: "rejected".into(),
            },
        )
        .expect("known rejection");
    state.draft.insert("name".into(), "new draft".into());
    let second = state.begin_save().expect("fresh intent");
    assert_ne!(first.operation_id, second.operation_id);
    assert!(state.resolve(&first, committed("v1", "old draft")).is_err());
    assert_eq!(state.pending_request(), Some(&second));
    assert_eq!(
        state.draft.get("name").map(String::as_str),
        Some("new draft")
    );
}

#[test]
fn detail_save_same_id_different_scope_or_payload_remains_unknown() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    let mut foreign = request.clone();
    foreign.scope = "other actor".into();
    assert!(state.resolve(&foreign, committed("v1", "draft")).is_err());
    assert_eq!(state.phase(), Some(DetailSavePhase::Unknown));
    assert_eq!(state.pending_request(), Some(&request));
}

#[test]
fn detail_save_version_conflict_requires_explicit_adoption_and_fresh_identity() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    state
        .resolve(
            &request,
            DetailSaveOutcome::Rejected {
                code: "VERSION_CONFLICT".into(),
                message: "changed".into(),
            },
        )
        .expect("conflict");
    assert!(state.begin_save().is_err());
    state
        .bind("one", "v1", &fields("other writer"))
        .expect("latest bound");
    assert_eq!(state.version, "v0");
    assert_eq!(state.draft.get("name").map(String::as_str), Some("draft"));
    state.adopt_latest().expect("explicit adoption");
    assert_eq!(state.version, "v1");
    assert_eq!(
        state.base.get("name").map(String::as_str),
        Some("other writer")
    );
    let next = state.begin_save().expect("new operation");
    assert_ne!(next.operation_id, request.operation_id);
    assert_eq!(next.observed_version, "v1");
    assert_eq!(
        next.before.get("name").map(String::as_str),
        Some("other writer")
    );
}

#[test]
fn detail_save_newer_version_during_pending_does_not_overwrite_original_intent() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    state
        .bind("one", "v2", &fields("newer data"))
        .expect("latest view");
    assert_eq!(state.pending_request(), Some(&request));
    assert_eq!(state.version, "v0");
    assert!(state.adopt_latest().is_err());
    state
        .resolve(&request, committed("v1", "draft"))
        .expect("original result");
    assert_eq!(state.version, "v1");
    state
        .bind("one", "v2", &fields("newer data"))
        .expect("current separately");
    assert_eq!(state.version, "v2");
}

#[test]
fn detail_save_partial_or_nonadvancing_success_cannot_clean_form() {
    let mut state = state();
    state.draft.insert("note".into(), "note draft".into());
    let request = state.begin_save().expect("intent");
    assert!(state.resolve(&request, committed("v1", "draft")).is_err());
    assert_eq!(state.phase(), Some(DetailSavePhase::Unknown));
    assert!(state.dirty());
    let values = request.changes.clone();
    assert!(
        state
            .resolve(
                &request,
                DetailSaveOutcome::Committed {
                    version: "v0".into(),
                    values
                }
            )
            .is_err()
    );
    assert_eq!(state.pending_request(), Some(&request));
}

#[test]
fn detail_save_required_unchanged_field_and_revoked_editability_block_save() {
    let mut state = FormState::new("scope".into());
    state.bind("one", "v0", &fields("")).expect("bind");
    state.draft.insert("note".into(), "changed".into());
    assert!(state.begin_save().is_err());
    let mut state = state_with_valid_name();
    state.draft.insert("name".into(), "draft".into());
    let mut schema = fields("old");
    schema[0].editable = false;
    state
        .bind("one", "v0", &schema)
        .expect("permissions changed");
    assert!(state.begin_save().is_err());
    assert_eq!(state.draft.get("name").map(String::as_str), Some("draft"));
}
fn state_with_valid_name() -> FormState {
    let mut state = FormState::new("scope".into());
    state.bind("one", "v0", &fields("old")).expect("bind");
    state
}

#[test]
fn detail_save_removed_empty_dirty_field_is_not_silently_dropped_on_adoption() {
    let mut state = state_with_valid_name();
    let mut schema = fields("old");
    schema[1].value = "old note".into();
    state.bind("one", "v0", &schema).expect("original");
    state.draft.insert("note".into(), String::new());
    schema.remove(1);
    state.bind("one", "v1", &schema).expect("removed latest");
    state.adopt_latest().expect("adopt latest");
    assert!(state.changes().contains_key("note"));
    assert!(state.begin_save().is_err());
}

#[test]
fn detail_save_checkpoint_retains_identity_but_excludes_unmodified_row_data() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    let bytes = state.checkpoint().expect("encode").expect("pending bytes");
    let text = String::from_utf8(bytes.clone()).expect("UTF8");
    assert!(!text.contains("PRIVATE_UNCHANGED_PHONE"));
    let mut restored = FormState::restore(&state.scope, &bytes).expect("restore");
    assert_eq!(restored.phase(), Some(DetailSavePhase::Unknown));
    assert_eq!(restored.pending_request(), Some(&request));
    assert!(restored.begin_save().is_err());
    assert!(restored.reset().is_err());
    assert_eq!(restored.begin_read().expect("read original"), request);
    restored
        .bind("one", "v0", &fields("old"))
        .expect("fresh metadata");
    restored
        .resolve(&request, committed("v1", "draft"))
        .expect("stored proof");
    assert!(!restored.dirty());
    assert!(FormState::restore("other actor", &bytes).is_err());
}

#[test]
fn detail_save_checkpoint_rejects_tampered_request_and_future_schema() {
    let mut state = state();
    state.begin_save().expect("intent");
    let bytes = state.checkpoint().expect("encode").expect("bytes");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    value["pending"]["changes"]["name"] = serde_json::json!("forged");
    let forged = serde_json::to_vec(&value).expect("encode");
    assert!(FormState::restore(&state.scope, &forged).is_err());
    value["schema"] = serde_json::json!(2);
    assert!(
        FormState::restore(&state.scope, &serde_json::to_vec(&value).expect("encode")).is_err()
    );
}

#[test]
fn detail_save_unbound_restore_requires_current_field_metadata() {
    let state = state();
    let bytes = state.checkpoint().expect("encode").expect("draft");
    let mut restored = FormState::restore(&state.scope, &bytes).expect("restore");
    assert!(restored.begin_save().is_err());
    restored.bind("one", "v0", &fields("old")).expect("fresh");
    let request = restored.begin_save().expect("save now");
    assert_eq!(
        request.changes.get("name").map(String::as_str),
        Some("draft")
    );
}

#[test]
fn detail_save_field_limits_fail_without_destroying_previous_draft() {
    let mut state = state();
    let invalid = vec![fields("old")[0].clone(); 257];
    assert!(state.bind("one", "v0", &invalid).is_err());
    assert!(state.begin_save().is_err());
    assert_eq!(state.draft.get("name").map(String::as_str), Some("draft"));
    state
        .bind("one", "v0", &fields("old"))
        .expect("fixed schema");
    assert!(state.begin_save().is_ok());
}

#[test]
fn detail_save_composing_input_cannot_be_submitted() {
    let mut state = state();
    state.composing = true;
    assert!(state.begin_save().is_err());
    state.composing = false;
    assert!(state.begin_save().is_ok());
}

#[test]
fn detail_save_confirmed_noop_uses_canonical_value_without_inventing_version() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    state
        .resolve(&request, committed("v0", "old"))
        .expect("authoritative no-op");
    assert!(!state.dirty());
    assert_eq!(state.draft.get("name").map(String::as_str), Some("old"));
}

#[test]
fn detail_save_empty_required_commit_remains_unknown() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    assert!(state.resolve(&request, committed("v1", "")).is_err());
    assert!(state.unresolved());
}

#[test]
fn detail_save_reset_and_late_receipt_never_rebind_another_record() {
    let mut state = state();
    let request = state.begin_save().expect("intent");
    state
        .resolve(
            &request,
            DetailSaveOutcome::Rejected {
                code: "VALIDATION".into(),
                message: "known rejection".into(),
            },
        )
        .expect("reject");
    state.reset().expect("discard draft");
    state
        .bind("two", "v5", &fields("different record"))
        .expect("other");
    assert!(state.resolve(&request, committed("v1", "draft")).is_err());
    assert_eq!(state.record_id.as_deref(), Some("two"));
    assert_eq!(state.version, "v5");
}

#[test]
fn detail_save_nil_identity_is_rejected_before_recovery() {
    let mut state = state();
    state.begin_save().expect("intent");
    let bytes = state.checkpoint().expect("encode").expect("bytes");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    value["pending"]["operation_id"] = serde_json::json!(uuid::Uuid::nil());
    assert!(
        FormState::restore(&state.scope, &serde_json::to_vec(&value).expect("encode")).is_err()
    );
}

#[test]
fn detail_save_lookup_keeps_all_original_fields_and_only_one_read_in_flight() {
    let mut state = state();
    let request: DetailSaveRequest = state.begin_save().expect("intent");
    state
        .resolve(&request, DetailSaveOutcome::Unknown("unknown".into()))
        .expect("unknown");
    let read = state.begin_read().expect("read");
    assert_eq!(read, request);
    assert!(state.begin_read().is_err());
    state
        .resolve(
            &read,
            DetailSaveOutcome::Unknown("404 does not prove absence".into()),
        )
        .expect("still unknown");
    assert_eq!(state.pending_request(), Some(&request));
    assert!(state.begin_save().is_err());
}

#[test]
fn detail_save_same_version_refresh_preserves_dirty_and_updates_clean_values() {
    let mut state = state();
    let mut refreshed = fields("server value");
    refreshed[2].value = "CURRENT_PHONE".into();
    state
        .bind("one", "v0", &refreshed)
        .expect("same version refresh");
    assert_eq!(state.draft.get("name").map(String::as_str), Some("draft"));
    assert_eq!(
        state.draft.get("phone").map(String::as_str),
        Some("CURRENT_PHONE")
    );
}

#[test]
fn detail_save_invalid_multibyte_draft_restores_but_cannot_be_submitted() {
    let mut state = state();
    let draft = "汉".repeat(2000);
    state.draft.insert("name".into(), draft.clone());
    assert!(state.begin_save().is_err());
    let bytes = state
        .checkpoint()
        .expect("bounded invalid draft")
        .expect("draft");
    let mut restored = FormState::restore(&state.scope, &bytes).expect("recover invalid input");
    restored
        .bind("one", "v0", &fields("old"))
        .expect("current schema");
    assert_eq!(restored.draft.get("name"), Some(&draft));
    assert!(restored.begin_save().is_err());
    restored
        .draft
        .insert("name".into(), "x".repeat(16 * 1024 + 1));
    assert!(restored.checkpoint().is_err());
}
