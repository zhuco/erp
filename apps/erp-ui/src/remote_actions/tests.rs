use super::*;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

fn command(source: u128) -> ActionCommand {
    ActionCommand {
        operation_id: RowId(source + 100),
        action: "products.copy".into(),
        source: RowId(source),
        expected_version: None,
        values: BTreeMap::new(),
    }
}
fn applied(source: u128) -> ActionReceipt {
    ActionReceipt {
        operation_id: RowId(source + 100),
        source: RowId(source),
        command_fingerprint: command(source).fingerprint(),
        outcome: ActionOutcome::Applied {
            row: RowId(source + 1000),
            version: "2026-09-10T00:00:00Z".into(),
            values: BTreeMap::from([(erp_grid_core::ColumnId(1), "Copy result".into())]),
        },
    }
}
pub(crate) fn server(
    responses: Vec<Option<(u16, String)>>,
) -> (HttpClient, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client =
        HttpClient::new(&format!("http://{}/erp", listener.local_addr().unwrap())).unwrap();
    let task = thread::spawn(move || {
        let mut requests = vec![];
        for response in responses {
            let deadline = Instant::now() + Duration::from_secs(8);
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                    Err(error) => panic!("Missing action request: {error}"),
                }
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")?
                                .trim()
                                .parse::<usize>()
                                .ok()
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
                assert!(bytes.len() <= 32768);
            }
            requests.push(String::from_utf8(bytes).unwrap());
            if let Some((status, body)) = response {
                write!(socket, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        }
        requests
    });
    (client, task)
}
fn scope(client: &HttpClient) -> ActionScope {
    ActionScope::new(client, RowId(1), RowId(2)).unwrap()
}
fn runner(client: &HttpClient, count: u128) -> ActionRunner {
    ActionRunner::new(
        scope(client),
        ActionBatch::new((1..=count).map(command).collect()).unwrap(),
    )
}
fn tick(runner: &mut ActionRunner, client: &HttpClient) {
    runner.tick(
        client,
        &scope(client),
        "test-secret-token",
        &egui::Context::default(),
    );
}
fn commit(runner: &mut ActionRunner) -> Vec<u8> {
    let write = runner
        .checkpoint()
        .expect("must persist before next network write");
    let bytes = write.bytes.clone();
    let ticket = write.ticket.clone();
    assert!(!String::from_utf8_lossy(&bytes).contains("test-secret-token"));
    assert!(runner.acknowledge(&ticket, Ok(())));
    bytes
}
fn wait_save(runner: &mut ActionRunner, client: &HttpClient) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while runner.checkpoint().is_none() {
        tick(runner, client);
        assert!(
            Instant::now() < deadline,
            "Did not reach checkpoint: {:?}",
            runner.notice()
        );
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn storage_failure_and_wrong_session_never_dispatch_and_stale_acks_cannot_unlock() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = HttpClient::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let mut execution = runner(&client, 1);
    tick(&mut execution, &client);
    let ticket = execution.checkpoint().unwrap().ticket.clone();
    assert!(execution.acknowledge(&ticket, Err("disk full".into())));
    for _ in 0..10 {
        tick(&mut execution, &client);
    }
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    assert!(execution.notice().unwrap().contains("disk full"));
    let mut other = runner(&client, 1);
    tick(&mut other, &client);
    assert!(
        !other.acknowledge(&ticket, Ok(())),
        "another executor cannot accept this storage callback"
    );
    commit(&mut execution);
    let wrong_actor = ActionScope::new(&client, RowId(1), RowId(3)).unwrap();
    execution.tick(&client, &wrong_actor, "token", &egui::Context::default());
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    assert!(!execution.acknowledge(&ticket, Ok(())));
}
#[test]
fn restart_after_lost_response_looks_up_receipt_and_never_creates_second_product() {
    let (client, server) = server(vec![
        None,
        Some((200, serde_json::to_string(&applied(1)).unwrap())),
    ]);
    let mut original = runner(&client, 2);
    tick(&mut original, &client);
    let persisted = commit(&mut original);
    tick(&mut original, &client);
    wait_save(&mut original, &client);
    assert_eq!(original.items()[0].state(), &ActionState::Unknown);
    drop(original); // Simulate loss before the latest state reached storage.
    let mut restored = ActionRunner::restore(scope(&client), &persisted).unwrap();
    assert!(restored.resume().is_err());
    restored
        .lookup(&client, &scope(&client), "token", &egui::Context::default())
        .unwrap();
    wait_save(&mut restored, &client);
    assert_eq!(restored.take_receipt(), Some(applied(1)));
    commit(&mut restored);
    restored.pause().unwrap();
    restored.cancel_unstarted().unwrap();
    let completed = commit(&mut restored);
    let mut restored = ActionRunner::restore(scope(&client), &completed).unwrap();
    restored.resume().unwrap();
    tick(&mut restored, &client);
    assert!(restored.finished());
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("POST /erp/v1/tenants/"));
    assert!(requests[1].starts_with("GET /erp/v1/tenants/"));
    assert!(requests[1].contains(&command(1).operation_id.to_uuid()));
}
#[test]
fn missing_receipt_replays_exact_body_only_after_second_durable_checkpoint() {
    let (client, server) = server(vec![
        None,
        Some((404, "{}".into())),
        Some((200, serde_json::to_string(&applied(1)).unwrap())),
    ]);
    let mut execution = runner(&client, 1);
    tick(&mut execution, &client);
    commit(&mut execution);
    tick(&mut execution, &client);
    wait_save(&mut execution, &client);
    commit(&mut execution);
    execution
        .lookup(&client, &scope(&client), "token", &egui::Context::default())
        .unwrap();
    wait_save(&mut execution, &client);
    assert_eq!(execution.items()[0].state(), &ActionState::Sending);
    let ticket = execution.checkpoint().unwrap().ticket.clone();
    assert!(execution.acknowledge(&ticket, Err("not durable yet".into())));
    assert!(execution.pending.is_none());
    commit(&mut execution);
    tick(&mut execution, &client);
    wait_save(&mut execution, &client);
    assert_eq!(execution.take_receipt(), Some(applied(1)));
    commit(&mut execution);
    tick(&mut execution, &client);
    assert!(execution.finished());
    let requests = server.join().unwrap();
    assert_eq!(
        requests[0].split("\r\n\r\n").nth(1),
        requests[2].split("\r\n\r\n").nth(1)
    );
    assert!(requests[1].starts_with("GET "));
}
#[test]
fn wrong_receipt_and_denied_lookup_remain_unknown_and_do_not_advance_batch() {
    let (client, server) = server(vec![
        Some((200, serde_json::to_string(&applied(99)).unwrap())),
        Some((403, "{}".into())),
    ]);
    let mut execution = runner(&client, 2);
    tick(&mut execution, &client);
    commit(&mut execution);
    tick(&mut execution, &client);
    wait_save(&mut execution, &client);
    assert_eq!(execution.items()[0].state(), &ActionState::Unknown);
    assert!(execution.take_receipt().is_none());
    let bytes = commit(&mut execution);
    for actor in [3, 4] {
        assert!(
            ActionRunner::restore(
                ActionScope::new(&client, RowId(1), RowId(actor)).unwrap(),
                &bytes
            )
            .is_err()
        );
    }
    let other = HttpClient::new("http://127.0.0.1:1").unwrap();
    assert!(ActionRunner::restore(scope(&other), &bytes).is_err());
    execution
        .lookup(&client, &scope(&client), "token", &egui::Context::default())
        .unwrap();
    wait_save(&mut execution, &client);
    commit(&mut execution);
    assert_eq!(execution.items()[0].state(), &ActionState::Unknown);
    assert_eq!(execution.items()[1].state(), &ActionState::Ready);
    assert!(execution.resume().is_err());
    assert_eq!(server.join().unwrap().len(), 2);
}
#[test]
fn business_rejection_is_persisted_and_next_target_keeps_its_original_identity() {
    let rejected = ActionReceipt {
        operation_id: RowId(101),
        source: RowId(1),
        command_fingerprint: command(1).fingerprint(),
        outcome: ActionOutcome::Rejected {
            code: "VERSION_CONFLICT".into(),
            message: "Source changed".into(),
        },
    };
    let (client, server) = server(vec![
        Some((200, serde_json::to_string(&rejected).unwrap())),
        Some((200, serde_json::to_string(&applied(2)).unwrap())),
    ]);
    let mut execution = runner(&client, 2);
    for expected in [rejected, applied(2)] {
        tick(&mut execution, &client);
        commit(&mut execution);
        tick(&mut execution, &client);
        wait_save(&mut execution, &client);
        assert_eq!(execution.take_receipt(), Some(expected));
        commit(&mut execution);
    }
    tick(&mut execution, &client);
    assert!(execution.finished());
    let requests = server.join().unwrap();
    let second: ActionCommand =
        serde_json::from_str(requests[1].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(second, command(2));
}

#[test]
fn identity_conflict_cannot_accept_another_commands_receipt_even_after_restart() {
    let (client, server) = server(vec![Some((409, "{}".into()))]);
    let mut execution = runner(&client, 2);
    tick(&mut execution, &client);
    commit(&mut execution);
    tick(&mut execution, &client);
    wait_save(&mut execution, &client);
    let bytes = commit(&mut execution);
    assert!(execution.notice().unwrap().contains("identity conflict"));
    let mut recovered = ActionRunner::restore(scope(&client), &bytes).unwrap();
    assert_eq!(recovered.items()[0].state(), &ActionState::IdentityConflict);
    assert!(recovered.resume().is_err());
    assert!(
        recovered
            .lookup(&client, &scope(&client), "token", &egui::Context::default())
            .is_err()
    );
    recovered.cancel_unstarted().unwrap();
    commit(&mut recovered);
    assert_eq!(recovered.items()[0].state(), &ActionState::IdentityConflict);
    assert_eq!(recovered.items()[1].state(), &ActionState::Cancelled);
    assert_eq!(server.join().unwrap().len(), 1);
}

#[test]
fn checkpoint_rejects_unsupported_versions_and_excessive_items_before_restoration() {
    let client = HttpClient::new("http://127.0.0.1:1").unwrap();
    let mut execution = runner(&client, 1);
    tick(&mut execution, &client);
    let original: serde_json::Value =
        serde_json::from_slice(&execution.checkpoint().unwrap().bytes).unwrap();
    let mut future = original.clone();
    future["version"] = serde_json::json!(2);
    assert!(ActionRunner::restore(scope(&client), &serde_json::to_vec(&future).unwrap()).is_err());
    let mut excessive = original;
    let item = excessive["batch"]["items"][0].clone();
    excessive["batch"]["items"] = serde_json::Value::Array(vec![item; 10_001]);
    assert!(
        ActionRunner::restore(scope(&client), &serde_json::to_vec(&excessive).unwrap()).is_err()
    );
}

#[test]
fn crash_before_persisting_conflict_does_not_accept_receipt_for_same_id_other_payload() {
    let (client, server) = server(vec![
        Some((409, "{}".into())),
        Some((200, serde_json::to_string(&applied(1)).unwrap())),
    ]);
    let mut intended = command(1);
    intended.expected_version = Some("2026-09-10T00:00:01Z".into());
    let mut execution =
        ActionRunner::new(scope(&client), ActionBatch::new(vec![intended]).unwrap());
    tick(&mut execution, &client);
    let last_durable = commit(&mut execution);
    tick(&mut execution, &client);
    wait_save(&mut execution, &client);
    drop(execution); // 409 reached the client but its new checkpoint did not.
    let mut recovered = ActionRunner::restore(scope(&client), &last_durable).unwrap();
    recovered
        .lookup(&client, &scope(&client), "token", &egui::Context::default())
        .unwrap();
    wait_save(&mut recovered, &client);
    assert_eq!(recovered.items()[0].state(), &ActionState::IdentityConflict);
    assert!(recovered.take_receipt().is_none());
    commit(&mut recovered);
    assert!(recovered.resume().is_err());
    assert_eq!(server.join().unwrap().len(), 2);
}

#[test]
fn sku_action_restart_uses_sku_receipt_without_resending_save() {
    let mut input = command(7);
    input.action = "skus.update".into();
    input.expected_version = Some("2026-09-09T00:00:00Z".into());
    input
        .values
        .insert(erp_grid_core::ColumnId(2), "SKU new name".into());
    let receipt = ActionReceipt {
        operation_id: input.operation_id,
        source: input.source,
        command_fingerprint: input.fingerprint(),
        outcome: ActionOutcome::Applied {
            row: input.source,
            version: "2026-09-10T00:00:00Z".into(),
            values: input.values.clone(),
        },
    };
    let (client, server) = server(vec![
        None,
        Some((200, serde_json::to_string(&receipt).unwrap())),
    ]);
    let mut original = ActionRunner::new(
        scope(&client),
        ActionBatch::new(vec![input.clone()]).unwrap(),
    );
    tick(&mut original, &client);
    let persisted = commit(&mut original);
    tick(&mut original, &client);
    wait_save(&mut original, &client);
    assert_eq!(original.items()[0].state(), &ActionState::Unknown);
    drop(original);
    let mut restored = ActionRunner::restore(scope(&client), &persisted).unwrap();
    restored
        .lookup(&client, &scope(&client), "token", &egui::Context::default())
        .unwrap();
    wait_save(&mut restored, &client);
    assert_eq!(restored.take_receipt(), Some(receipt));
    commit(&mut restored);
    tick(&mut restored, &client);
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[0]
            .lines()
            .next()
            .unwrap()
            .contains("/skus/actions?")
    );
    assert!(
        requests[1]
            .lines()
            .next()
            .unwrap()
            .contains(&format!("/skus/actions/{}", input.operation_id.to_uuid()))
    );
    let body: ActionCommand =
        serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body, input);
}

#[test]
fn update_receipt_for_another_result_row_stays_unknown() {
    for action in ["products.update", "skus.update"] {
        let mut input = command(7);
        input.action = action.into();
        input.expected_version = Some("2026-09-10T00:00:00Z".into());
        input
            .values
            .insert(erp_grid_core::ColumnId(1), "value".into());
        let receipt = ActionReceipt {
            operation_id: input.operation_id,
            source: input.source,
            command_fingerprint: input.fingerprint(),
            outcome: ActionOutcome::Applied {
                row: RowId(999),
                version: "2026-09-10T01:00:00Z".into(),
                values: input.values.clone(),
            },
        };
        let (client, server) = server(vec![Some((200, serde_json::to_string(&receipt).unwrap()))]);
        let mut runner = ActionRunner::new(scope(&client), ActionBatch::new(vec![input]).unwrap());
        tick(&mut runner, &client);
        commit(&mut runner);
        tick(&mut runner, &client);
        wait_save(&mut runner, &client);
        assert_eq!(runner.items()[0].state(), &ActionState::Unknown);
        assert!(runner.take_receipt().is_none());
        assert_eq!(server.join().unwrap().len(), 1);
    }
}
