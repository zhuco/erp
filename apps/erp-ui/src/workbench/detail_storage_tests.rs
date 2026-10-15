#![cfg(not(target_arch = "wasm32"))]
use super::detail::{DetailField, DetailPanel, DetailSaveOutcome, DetailSavePhase};
use super::detail_storage::{DetailStore, wait_storage};
use std::path::PathBuf;
fn root() -> PathBuf {
    std::env::current_dir()
        .expect("cwd")
        .join("qa/acceptance/detail-durable-native")
        .join(uuid::Uuid::new_v4().to_string())
}
fn panel(scope: &str) -> DetailPanel {
    let mut panel = DetailPanel::new(scope.into());
    panel.bind(
        "one",
        "0",
        &[DetailField {
            id: "name".into(),
            label: "Name".into(),
            value: "old".into(),
            editable: true,
            required: true,
        }],
    );
    panel.edit_value("name", "changed".into()).expect("edit");
    panel
}
#[test]
fn detail_durable_sqlite_readback_authorizes_only_matching_original() {
    let mut panel = panel("owner/page");
    let request = panel.begin_save().expect("intent");
    assert_eq!(panel.save_phase(), Some(DetailSavePhase::Checkpointing));
    let ctx = eframe::egui::Context::default();
    let loaded =
        wait_storage(DetailStore::open_at(root(), request.scope.clone(), &ctx).expect("open"))
            .expect("loaded");
    let receipt = wait_storage(
        loaded
            .store
            .commit(0, panel.persistence_write().expect("write"), &ctx)
            .expect("commit"),
    )
    .expect("stored/readback");
    assert_eq!(receipt.revision(), 1);
    let mut wrong = request.clone();
    wrong.operation_id = uuid::Uuid::new_v4();
    assert!(panel.authorize_send(&wrong, &receipt).is_err());
    assert_eq!(panel.save_phase(), Some(DetailSavePhase::Checkpointing));
    panel
        .authorize_send(&request, &receipt)
        .expect("authorize original");
    assert_eq!(panel.save_phase(), Some(DetailSavePhase::Awaiting));
}
#[test]
fn detail_durable_independent_connection_reopens_unknown_without_new_identity() {
    let root = root();
    let ctx = eframe::egui::Context::default();
    let mut panel = panel("owner/page");
    let request = panel.begin_save().expect("intent");
    let loaded = wait_storage(
        DetailStore::open_at(root.clone(), request.scope.clone(), &ctx).expect("open"),
    )
    .expect("loaded");
    wait_storage(
        loaded
            .store
            .commit(0, panel.persistence_write().expect("write"), &ctx)
            .expect("commit"),
    )
    .expect("stored");
    crate::http::runtime()
        .expect("runtime")
        .block_on(loaded.store.close());
    let independent = wait_storage(
        DetailStore::open_at(root, request.scope.clone(), &ctx).expect("independent open"),
    )
    .expect("loaded2");
    assert_eq!(independent.revision, 1);
    let restored = DetailPanel::restore(&request.scope, &independent.bytes.expect("actual bytes"))
        .expect("restore");
    assert_eq!(restored.save_phase(), Some(DetailSavePhase::Unknown));
    assert_eq!(restored.pending_request(), Some(&request));
}
#[test]
fn detail_durable_stale_window_and_unresolved_clear_are_rejected() {
    let root = root();
    let ctx = eframe::egui::Context::default();
    let mut first = panel("owner/page");
    let request = first.begin_save().expect("intent");
    let a = wait_storage(
        DetailStore::open_at(root.clone(), request.scope.clone(), &ctx).expect("open"),
    )
    .expect("a");
    let b = wait_storage(
        DetailStore::open_at(root.clone(), request.scope.clone(), &ctx).expect("open2"),
    )
    .expect("b");
    wait_storage(
        a.store
            .commit(0, first.persistence_write().expect("write"), &ctx)
            .expect("commit"),
    )
    .expect("stored");
    assert!(
        wait_storage(
            b.store
                .commit(0, first.persistence_write().expect("write"), &ctx)
                .expect("schedule stale")
        )
        .is_err()
    );
    let empty = DetailPanel::new(request.scope.clone());
    assert!(
        wait_storage(
            a.store
                .commit(1, empty.persistence_write().expect("empty"), &ctx)
                .expect("schedule clear")
        )
        .is_err()
    );
    let still =
        wait_storage(DetailStore::open_at(root, request.scope.clone(), &ctx).expect("read"))
            .expect("load");
    assert_eq!(still.revision, 1);
    assert!(still.bytes.is_some());
}
#[test]
fn detail_durable_terminal_clear_keeps_revision_and_other_scope() {
    let root = root();
    let ctx = eframe::egui::Context::default();
    let mut a = panel("owner/page");
    let request = a.begin_save().expect("intent");
    let store = wait_storage(
        DetailStore::open_at(root.clone(), request.scope.clone(), &ctx).expect("open"),
    )
    .expect("a");
    let proof = wait_storage(
        store
            .store
            .commit(0, a.persistence_write().expect("write"), &ctx)
            .expect("commit"),
    )
    .expect("stored");
    a.authorize_send(&request, &proof).expect("authorize");
    a.resolve_save(
        &request,
        DetailSaveOutcome::Committed {
            version: "1".into(),
            values: [("name".into(), "changed".into())].into_iter().collect(),
        },
    )
    .expect("verified receipt");
    let clear = wait_storage(
        store
            .store
            .commit(1, a.persistence_write().expect("terminal"), &ctx)
            .expect("clear"),
    )
    .expect("cleared");
    assert_eq!(clear.revision(), 2);
    let after = wait_storage(
        DetailStore::open_at(root.clone(), request.scope.clone(), &ctx).expect("reopen"),
    )
    .expect("after");
    assert_eq!(after.revision, 2);
    assert!(after.bytes.is_none());
    let other = wait_storage(DetailStore::open_at(root, "other/page".into(), &ctx).expect("other"))
        .expect("other loaded");
    assert_eq!(other.revision, 0);
    assert!(other.bytes.is_none());
}

#[test]
fn detail_backup_independent_sqlite_copy_retains_primary_and_cannot_authorize() {
    let root = root();
    let ctx = eframe::egui::Context::default();
    let mut original = panel("owner/page");
    let original_request = original.begin_save().expect("original");
    let primary =
        wait_storage(DetailStore::open_at(root.clone(), "owner/page".into(), &ctx).expect("open"))
            .expect("loaded");
    wait_storage(
        primary
            .store
            .commit(0, original.persistence_write().expect("write"), &ctx)
            .expect("commit"),
    )
    .expect("stored");
    let mut local = panel("owner/page");
    local
        .edit_value("name", "different draft".into())
        .expect("edit");
    let local_request = local.begin_save().expect("prepared local");
    let backup = wait_storage(
        DetailStore::open_backup_at(root.clone(), "owner/page".into(), &ctx).expect("backup open"),
    )
    .expect("loaded");
    let proof = wait_storage(
        backup
            .store
            .commit(0, local.persistence_write().expect("backup write"), &ctx)
            .expect("commit"),
    )
    .expect("backed up");
    assert!(local.authorize_send(&local_request, &proof).is_err());
    assert_eq!(local.save_phase(), Some(DetailSavePhase::Checkpointing));
    let original_after = wait_storage(
        DetailStore::open_at(root.clone(), "owner/page".into(), &ctx).expect("primary reopen"),
    )
    .expect("original");
    assert_eq!(original_after.revision, 1);
    let restored = DetailPanel::restore("owner/page", &original_after.bytes.expect("bytes"))
        .expect("original decode");
    assert_eq!(restored.pending_request(), Some(&original_request));
    let copy = wait_storage(
        DetailStore::open_backup_at(root, "owner/page".into(), &ctx).expect("copy reopen"),
    )
    .expect("copy");
    assert_eq!(copy.revision, 1);
    let recovered =
        DetailPanel::restore("owner/page", &copy.bytes.expect("copy bytes")).expect("copy decode");
    assert_eq!(recovered.pending_request(), Some(&local_request));
    assert_eq!(recovered.save_phase(), Some(DetailSavePhase::Unknown));
}
#[test]
fn detail_backup_stale_or_changed_unresolved_copy_never_overwrites() {
    let root = root();
    let ctx = eframe::egui::Context::default();
    let mut local = panel("owner/page");
    local.begin_save().expect("local");
    let first = wait_storage(
        DetailStore::open_backup_at(root.clone(), "owner/page".into(), &ctx).expect("open"),
    )
    .expect("first");
    let peer = wait_storage(
        DetailStore::open_backup_at(root.clone(), "owner/page".into(), &ctx).expect("peer"),
    )
    .expect("peer");
    wait_storage(
        first
            .store
            .commit(0, local.persistence_write().expect("write"), &ctx)
            .expect("commit"),
    )
    .expect("saved");
    assert!(
        wait_storage(
            peer.store
                .commit(0, local.persistence_write().expect("write"), &ctx)
                .expect("stale")
        )
        .is_err()
    );
    let mut changed = panel("owner/page");
    changed.edit_value("name", "newer".into()).expect("edit");
    changed.begin_save().expect("different");
    assert!(
        wait_storage(
            first
                .store
                .commit(1, changed.persistence_write().expect("write"), &ctx)
                .expect("replace")
        )
        .is_err()
    );
    let copy =
        wait_storage(DetailStore::open_backup_at(root, "owner/page".into(), &ctx).expect("read"))
            .expect("copy");
    assert_eq!(copy.revision, 1);
}
