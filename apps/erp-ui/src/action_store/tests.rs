use super::*;
use crate::{http::HttpClient, remote_actions::ActionScope};
use erp_grid_core::{
    RowId,
    actions::{ActionBatch, ActionCommand, ActionState},
};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

fn scope(actor: u128) -> ActionScope {
    ActionScope::new(
        &HttpClient::new("http://127.0.0.1:1/erp").unwrap(),
        RowId(1),
        RowId(actor),
    )
    .unwrap()
}
fn runner(actor: u128, source: u128) -> ActionRunner {
    let mut runner = ActionRunner::new(
        scope(actor),
        ActionBatch::new(vec![ActionCommand {
            operation_id: RowId(source + 100),
            source: RowId(source),
            action: "products.copy".into(),
            expected_version: None,
            values: Default::default(),
        }])
        .unwrap(),
    );
    runner.tick(
        &HttpClient::new("http://127.0.0.1:1/erp").unwrap(),
        &scope(actor),
        "not-stored",
        &egui::Context::default(),
    );
    runner
}
fn wait<T>(mut task: StorageTask<T>) -> Result<T, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = task.poll() {
            return result;
        }
        assert!(Instant::now() < deadline, "Storage task timeout");
        thread::sleep(Duration::from_millis(5));
    }
}
fn open(root: &Path, actor: u128) -> LoadedStore {
    wait(ActionStore::open_at(root.to_owned(), scope(actor), &egui::Context::default()).unwrap())
        .unwrap()
}
fn commit(store: &ActionStore, runner: &mut ActionRunner, revision: u64) -> u64 {
    let write = runner.checkpoint().unwrap();
    let ticket = write.ticket.clone();
    let revision = wait(
        store
            .commit(revision, write, &egui::Context::default())
            .unwrap(),
    )
    .unwrap();
    assert!(runner.acknowledge(&ticket, Ok(())));
    revision
}
fn close(store: ActionStore) {
    crate::http::runtime().unwrap().block_on(store.pool.close());
}

fn open_draft(store: &ActionStore, grid: &str) -> LoadedDraft {
    wait(
        store
            .open_drafts(grid, 1, &egui::Context::default())
            .unwrap(),
    )
    .unwrap()
}

fn draft_bytes(store: &DraftStore, now: u64, row: u128) -> Vec<u8> {
    crate::edit_drafts::encode(
        store.scope(),
        now,
        &[crate::edit_drafts::DraftCell {
            row: RowId(row),
            column: erp_grid_core::ColumnId(1),
            before: "old".into(),
            after: "wanted".into(),
            expected_version: "v1".into(),
            parent: None,
        }],
    )
    .unwrap()
}

fn commit_draft(
    store: &DraftStore,
    expected: u64,
    bytes: Option<&[u8]>,
    now: u64,
) -> Result<u64, String> {
    wait(
        store
            .commit(expected, bytes, now, &egui::Context::default())
            .unwrap(),
    )
}

#[test]
fn sqlite_reopen_keeps_pending_identity_and_refuses_cross_scope_or_new_batch_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let LoadedStore {
        store,
        revision,
        bytes,
    } = open(dir.path(), 2);
    assert_eq!(revision, 0);
    assert!(bytes.is_none());
    let mut action = runner(2, 10);
    assert_eq!(commit(&store, &mut action, revision), 1);
    close(store);
    drop(action);
    let recovered = open(dir.path(), 2);
    assert_eq!(recovered.revision, 1);
    let bytes = recovered.bytes.as_ref().unwrap();
    assert!(!String::from_utf8_lossy(bytes).contains("not-stored"));
    let restored = ActionRunner::restore(scope(2), bytes).unwrap();
    assert_eq!(restored.items()[0].state(), &ActionState::Unknown);
    assert_eq!(restored.items()[0].command().operation_id, RowId(110));
    let new_batch = runner(2, 11);
    let result = wait(
        recovered
            .store
            .commit(
                1,
                new_batch.checkpoint().unwrap(),
                &egui::Context::default(),
            )
            .unwrap(),
    );
    assert!(result.unwrap_err().contains("Unresolved"));
    let other_actor = open(dir.path(), 3);
    assert_eq!(other_actor.revision, 0);
    assert!(other_actor.bytes.is_none());
    assert!(
        wait(
            other_actor
                .store
                .commit(
                    0,
                    new_batch.checkpoint().unwrap(),
                    &egui::Context::default()
                )
                .unwrap()
        )
        .is_err()
    );
    close(other_actor.store);
    close(recovered.store);
}

#[test]
fn sqlite_draft_reopens_and_global_epoch_rejects_a_cross_scope_writer() {
    let dir = tempfile::tempdir().unwrap();
    let first_actions = open(dir.path(), 2).store;
    let second_actions = open(dir.path(), 3).store;
    let first = open_draft(&first_actions, "products");
    let stale_other_scope = open_draft(&second_actions, "skus");
    assert_eq!(first.revision, stale_other_scope.revision);
    assert!(first.bytes.is_none());
    assert!(stale_other_scope.bytes.is_none());
    let now = 2_000_000_000;
    let bytes = draft_bytes(&first.store, now, 10);
    let committed = commit_draft(&first.store, first.revision, Some(&bytes), now).unwrap();
    let other_bytes = draft_bytes(&stale_other_scope.store, now, 11);
    assert!(
        commit_draft(
            &stale_other_scope.store,
            stale_other_scope.revision,
            Some(&other_bytes),
            now,
        )
        .unwrap_err()
        .contains("changed")
    );
    close(first_actions);
    close(second_actions);

    let reopened_actions = open(dir.path(), 2).store;
    let reopened = open_draft(&reopened_actions, "products");
    assert_eq!(reopened.revision, committed);
    assert_eq!(reopened.bytes, Some(bytes));
    close(reopened_actions);
}

#[test]
fn sqlite_draft_clear_releases_capacity_and_rejects_a_pre_clear_writer() {
    let dir = tempfile::tempdir().unwrap();
    let actions = open(dir.path(), 2).store;
    let now = 2_000_000_000;
    let mut epoch = 0;
    for index in 0..crate::draft_store::MAX_DRAFTS {
        let loaded = open_draft(&actions, &format!("grid-{index}"));
        assert_eq!(loaded.revision, epoch);
        let bytes = draft_bytes(&loaded.store, now, index as u128 + 1);
        epoch = commit_draft(&loaded.store, epoch, Some(&bytes), now).unwrap();
    }
    let stale = open_draft(&actions, "replacement");
    let full_bytes = draft_bytes(&stale.store, now, 100);
    assert!(
        commit_draft(&stale.store, epoch, Some(&full_bytes), now)
            .unwrap_err()
            .contains("32 scopes")
    );

    let existing = open_draft(&actions, "grid-0");
    assert_eq!(existing.revision, epoch);
    epoch = commit_draft(&existing.store, epoch, None, now).unwrap();
    assert!(
        commit_draft(&stale.store, stale.revision, Some(&full_bytes), now)
            .unwrap_err()
            .contains("changed")
    );
    let replacement = open_draft(&actions, "replacement");
    assert_eq!(replacement.revision, epoch);
    let final_epoch = commit_draft(&replacement.store, epoch, Some(&full_bytes), now).unwrap();
    assert_eq!(final_epoch, epoch + 1);
    close(actions);
}

#[test]
fn sqlite_cas_allows_one_writer_and_failure_rolls_back_the_previous_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let first = open(dir.path(), 2);
    let mut initial = runner(2, 10);
    commit(&first.store, &mut initial, 0);
    let second = open(dir.path(), 2);
    initial.pause().unwrap();
    let ctx = egui::Context::default();
    let a = first
        .store
        .commit(1, initial.checkpoint().unwrap(), &ctx)
        .unwrap();
    let b = second
        .store
        .commit(1, initial.checkpoint().unwrap(), &ctx)
        .unwrap();
    let results = [wait(a), wait(b)];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(
        results
            .iter()
            .find_map(|result| result.as_ref().err())
            .unwrap()
            .contains("changed")
    );
    let loaded = open(dir.path(), 2);
    assert_eq!(loaded.revision, 2);
    let previous = loaded.bytes.unwrap();
    crate::http::runtime().unwrap().block_on(async {
        sqlx::query("create trigger fail_recovery before update on action_checkpoints begin select raise(abort, 'fixture disk failure'); end")
            .execute(&loaded.store.pool).await.unwrap();
    });
    assert!(
        wait(
            loaded
                .store
                .commit(2, initial.checkpoint().unwrap(), &ctx)
                .unwrap()
        )
        .is_err()
    );
    close(first.store);
    close(second.store);
    close(loaded.store);
    let after_failure = open(dir.path(), 2);
    assert_eq!(after_failure.revision, 2);
    assert_eq!(after_failure.bytes, Some(previous));
    close(after_failure.store);
}

#[test]
fn sqlite_corruption_and_scope_quota_fail_closed_without_resetting_records() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = open(dir.path(), 2);
    let mut initial = runner(2, 10);
    commit(&loaded.store, &mut initial, 0);
    crate::http::runtime().unwrap().block_on(async {
        sqlx::query("update action_checkpoints set body=x'00' where scope=?")
            .bind(scope(2).storage_key().unwrap())
            .execute(&loaded.store.pool)
            .await
            .unwrap();
        for index in 0..31 {
            sqlx::query("insert into action_checkpoints(scope,revision,body) values(?,1,x'00')")
                .bind(format!("quota-fixture-{index}"))
                .execute(&loaded.store.pool)
                .await
                .unwrap();
        }
    });
    close(loaded.store);
    assert!(
        wait(
            ActionStore::open_at(dir.path().to_owned(), scope(2), &egui::Context::default())
                .unwrap()
        )
        .is_err()
    );
    let other = open(dir.path(), 3);
    let initial = runner(3, 10);
    assert!(
        wait(
            other
                .store
                .commit(0, initial.checkpoint().unwrap(), &egui::Context::default())
                .unwrap()
        )
        .unwrap_err()
        .contains("32 account scopes")
    );
    close(other.store);
}

#[test]
fn native_storage_rejects_relative_and_parent_traversal_paths() {
    assert!(native::prepare("relative".into()).is_err());
    let dir = tempfile::tempdir().unwrap();
    assert!(native::prepare(dir.path().join("../elsewhere")).is_err());
}

#[test]
fn sqlite_checkpoint_and_http_recovery_complete_copy_without_duplicate_post() {
    use erp_grid_core::actions::{ActionOutcome, ActionReceipt};
    let input = ActionCommand {
        operation_id: RowId(110),
        source: RowId(10),
        action: "products.copy".into(),
        expected_version: None,
        values: Default::default(),
    };
    let applied = ActionReceipt {
        operation_id: input.operation_id,
        source: input.source,
        command_fingerprint: input.fingerprint(),
        outcome: ActionOutcome::Applied {
            row: RowId(200),
            version: "2026-09-10T00:00:00Z".into(),
            values: Default::default(),
        },
    };
    let (client, server) = crate::remote_actions::tests::server(vec![
        None,
        Some((200, serde_json::to_string(&applied).unwrap())),
    ]);
    let scope = ActionScope::new(&client, RowId(1), RowId(2)).unwrap();
    let ctx = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let loaded =
        wait(ActionStore::open_at(dir.path().to_owned(), scope.clone(), &ctx).unwrap()).unwrap();
    let mut original = ActionRunner::new(scope.clone(), ActionBatch::new(vec![input]).unwrap());
    original.tick(&client, &scope, "fixture-token", &ctx);
    commit(&loaded.store, &mut original, 0);
    original.tick(&client, &scope, "fixture-token", &ctx);
    let deadline = Instant::now() + Duration::from_secs(10);
    while original.checkpoint().is_none() {
        original.tick(&client, &scope, "fixture-token", &ctx);
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(original.items()[0].state(), &ActionState::Unknown);
    drop(original);
    close(loaded.store); // No post-response checkpoint was persisted.
    let loaded =
        wait(ActionStore::open_at(dir.path().to_owned(), scope.clone(), &ctx).unwrap()).unwrap();
    let mut recovered =
        ActionRunner::restore(scope.clone(), loaded.bytes.as_ref().unwrap()).unwrap();
    recovered
        .lookup(&client, &scope, "fixture-token", &ctx)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while recovered.checkpoint().is_none() {
        recovered.tick(&client, &scope, "fixture-token", &ctx);
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(recovered.take_receipt(), Some(applied));
    commit(&loaded.store, &mut recovered, loaded.revision);
    recovered.tick(&client, &scope, "fixture-token", &ctx);
    assert!(recovered.finished());
    close(loaded.store);
    let final_store =
        wait(ActionStore::open_at(dir.path().to_owned(), scope.clone(), &ctx).unwrap()).unwrap();
    let persisted = ActionRunner::restore(scope, final_store.bytes.as_ref().unwrap()).unwrap();
    assert!(matches!(
        persisted.items()[0].state(),
        ActionState::Complete(ActionOutcome::Applied {
            row: RowId(200),
            ..
        })
    ));
    close(final_store.store);
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("POST "));
    assert!(requests[1].starts_with("GET "));
}

#[test]
fn sqlite_process_interruption_rolls_back_uncommitted_checkpoint() {
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
    };
    let dir = tempfile::tempdir().unwrap();
    let loaded = open(dir.path(), 2);
    let mut action = runner(2, 10);
    commit(&loaded.store, &mut action, 0);
    close(loaded.store);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "action_store::tests::sqlite_crash_worker",
            "--nocapture",
        ])
        .env("ERP_GRID_CRASH_FIXTURE_DIR", dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let output = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            if line.contains("UNCOMMITTED_CHECKPOINT_READY") {
                let _ = sender.send(());
                break;
            }
        }
    });
    let ready = receiver.recv_timeout(Duration::from_secs(10));
    // Only this test-owned child is terminated, while its SQLite transaction is open.
    child.kill().unwrap();
    child.wait().unwrap();
    reader.join().unwrap();
    assert!(
        ready.is_ok(),
        "Crash fixture did not enter an uncommitted transaction"
    );
    let recovered = open(dir.path(), 2);
    assert_eq!(recovered.revision, 1);
    let runner = ActionRunner::restore(scope(2), recovered.bytes.as_ref().unwrap()).unwrap();
    assert_eq!(runner.items()[0].state(), &ActionState::Unknown);
    assert_eq!(runner.items()[0].command().operation_id, RowId(110));
    close(recovered.store);
}

#[test]
fn sqlite_crash_worker() {
    use std::io::Write;
    let Some(directory) = std::env::var_os("ERP_GRID_CRASH_FIXTURE_DIR") else {
        return;
    };
    let path = native::prepare(directory.into()).unwrap();
    crate::http::runtime().unwrap().block_on(async {
        let pool = native::connect(&path).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("update action_checkpoints set revision=revision+1,body=x'00' where scope=?")
            .bind(scope(2).storage_key().unwrap())
            .execute(&mut *tx)
            .await
            .unwrap();
        println!("UNCOMMITTED_CHECKPOINT_READY");
        std::io::stdout().flush().unwrap();
        thread::sleep(Duration::from_secs(30));
        panic!("Parent did not terminate crash fixture");
    });
}
