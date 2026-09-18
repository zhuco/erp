use super::tests::*;
use super::*;
use erp_grid_core::{
    Cell,
    actions::{ActionCommand, ActionOutcome, ActionState},
};

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}
fn click(ctx: &egui::Context, view: &mut RemoteProductView, label: &str) {
    let output = frame(ctx, view, vec![]);
    let point = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape
                && text.galley.text() == label
            {
                Some(text.pos + text.galley.size() / 2.0)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("Missing button {label}"));
    frame(ctx, view, vec![egui::Event::PointerMoved(point)]);
    for pressed in [true, false] {
        frame(
            ctx,
            view,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
}

#[test]
fn remote_edit_keeps_conflicting_draft_and_requires_review_before_a_new_versioned_save() {
    let root = tempfile::tempdir().unwrap();
    let profile = serde_json::json!({"user": {"user_id": RowId(2).to_uuid(), "tenant_id": RowId(1).to_uuid(), "status": "ACTIVE"}, "permissions": ["BASIC.SKU.READ", "BASIC.SKU.WRITE"]}).to_string();
    let (url, server) = server_with_conflict(vec![
        (200, serde_json::json!({"access_token": "fixture-token", "token_type": "Bearer", "expires_in": 3600}).to_string()),
        (200, page(10, 1, 1)), (200, page(10, 1, 1)),
    ], Some(profile), true);
    let ctx = egui::Context::default();
    let mut view = RemoteProductView::default();
    view.actions.test_root = Some(root.path().to_owned());
    login(&mut view, url, &ctx);
    wait(&ctx, &mut view, |v| {
        v.actions.available() && v.pages.page(&v.address).is_some()
    });
    view.state
        .request_edit(Cell { row: 0, column: 1 }, RowId(10), products::NAME);
    frame(&ctx, &mut view, vec![]);
    frame(&ctx, &mut view, vec![egui::Event::Text(" edited".into())]);
    let proposed = view.state.edit_draft().unwrap().after;
    assert_eq!(proposed, "Remote product 10 edited");
    frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
    wait(&ctx, &mut view, |v| v.actions.available());
    assert!(view.dirty());
    assert_eq!(view.state.edit_draft().unwrap().after, proposed);
    assert!(matches!(
        view.actions.runner.as_ref().unwrap().items()[0].state(),
        ActionState::Complete(ActionOutcome::Rejected { .. })
    ));
    let original_op = view.actions.runner.as_ref().unwrap().items()[0]
        .command()
        .operation_id;
    click(&ctx, &mut view, "Load latest version");
    wait(&ctx, &mut view, |v| {
        v.editing
            .as_ref()
            .is_some_and(|editing| editing.latest.is_some())
    });
    assert_eq!(
        view.actions.runner.as_ref().unwrap().items()[0]
            .command()
            .operation_id,
        original_op
    );
    click(&ctx, &mut view, "Use latest version");
    assert_eq!(
        view.state.edit_draft().unwrap().before,
        "Other user's value"
    );
    assert_eq!(view.state.edit_draft().unwrap().after, proposed);
    assert_eq!(
        view.actions.runner.as_ref().unwrap().items()[0]
            .command()
            .operation_id,
        original_op,
        "Review button must not submit automatically"
    );
    // Review returns focus to the retained editor; Enter explicitly retries.
    frame(&ctx, &mut view, vec![]);
    frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
    wait(&ctx, &mut view, |v| {
        !v.dirty() && v.actions.available() && v.pages.page(&v.address).is_some()
    });
    assert_eq!(
        view.pages.page(&view.address).unwrap().rows[0].cells[&products::NAME],
        proposed
    );
    let requests = server.join().unwrap();
    let commands: Vec<ActionCommand> = requests
        .iter()
        .filter(|request| {
            request.starts_with("POST ")
                && request
                    .lines()
                    .next()
                    .unwrap()
                    .contains("/products/actions")
        })
        .map(|request| serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap())
        .collect();
    assert_eq!(commands.len(), 2);
    assert_ne!(commands[0].operation_id, commands[1].operation_id);
    assert_eq!(
        commands[0].expected_version.as_deref(),
        Some("2026-09-09T00:00:00Z")
    );
    assert_eq!(
        commands[1].expected_version.as_deref(),
        Some("2026-09-10T00:00:00Z")
    );
    assert_eq!(commands[1].values.get(&products::NAME), Some(&proposed));
    crate::http::runtime()
        .unwrap()
        .block_on(view.actions.store.as_ref().unwrap().close());
}

#[test]
fn remote_tab_waits_for_save_then_crosses_pages_and_shift_tab_returns_to_saved_value() {
    let root = tempfile::tempdir().unwrap();
    let profile = serde_json::json!({"user": {"user_id": RowId(2).to_uuid(), "tenant_id": RowId(1).to_uuid(), "status": "ACTIVE"}, "permissions": ["BASIC.SKU.READ", "BASIC.SKU.WRITE"]}).to_string();
    let (url, server) = server_with_conflict(vec![
        (200, serde_json::json!({"access_token": "fixture-token", "token_type": "Bearer", "expires_in": 3600}).to_string()),
        (200, page(1, 10, 11)), (200, page(1, 10, 11)), (200, page(11, 1, 11)),
    ], Some(profile), false);
    let ctx = egui::Context::default();
    let mut view = RemoteProductView {
        address: PageAddress::Offset {
            start: 0,
            limit: 10,
        },
        ..Default::default()
    };
    view.actions.test_root = Some(root.path().to_owned());
    login(&mut view, url, &ctx);
    wait(&ctx, &mut view, |v| {
        v.actions.available() && v.pages.page(&v.address).is_some()
    });
    view.state.row_selection.toggle(RowId(2));
    view.state
        .request_edit(Cell { row: 9, column: 3 }, RowId(10), products::CATEGORY);
    frame(&ctx, &mut view, vec![]);
    frame(&ctx, &mut view, vec![egui::Event::Text("Tools".into())]);
    frame(&ctx, &mut view, vec![key(egui::Key::Tab)]);
    wait(&ctx, &mut view, |v| {
        v.page_number == 1
            && v.state
                .edit_draft()
                .is_some_and(|draft| draft.row == RowId(11) && draft.column == products::CODE)
    });
    let mut shift_tab = key(egui::Key::Tab);
    if let egui::Event::Key { modifiers, .. } = &mut shift_tab {
        *modifiers = egui::Modifiers::SHIFT;
    }
    frame(&ctx, &mut view, vec![shift_tab]);
    wait(&ctx, &mut view, |v| {
        v.page_number == 0
            && v.state
                .edit_draft()
                .is_some_and(|draft| draft.row == RowId(10) && draft.column == products::CATEGORY)
    });
    assert_eq!(view.state.edit_draft().unwrap().before, "Tools");
    assert!(view.state.row_selection.checked().contains(&RowId(2)));
    let requests = server.join().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request
                .lines()
                .next()
                .unwrap()
                .contains("/products/actions"))
            .count(),
        1
    );
    crate::http::runtime()
        .unwrap()
        .block_on(view.actions.store.as_ref().unwrap().close());
}

#[test]
fn expiry_keeps_unsent_editor_and_reauthentication_preserves_its_original_version() {
    let root = tempfile::tempdir().unwrap();
    let profile = serde_json::json!({"user": {"user_id": RowId(2).to_uuid(), "tenant_id": RowId(1).to_uuid(), "status": "ACTIVE"}, "permissions": ["BASIC.SKU.READ", "BASIC.SKU.WRITE"]}).to_string();
    let token = serde_json::json!({"access_token": "fixture-token", "token_type": "Bearer", "expires_in": 3600}).to_string();
    let (url, server) = server_with_conflict(
        vec![
            (200, token.clone()),
            (200, page(10, 1, 1)),
            (200, token),
            (200, page(10, 1, 1)),
        ],
        Some(profile),
        false,
    );
    let ctx = egui::Context::default();
    let mut view = RemoteProductView::default();
    view.actions.test_root = Some(root.path().to_owned());
    login(&mut view, url, &ctx);
    wait(&ctx, &mut view, |v| {
        v.actions.available() && v.pages.page(&v.address).is_some()
    });
    view.state
        .request_edit(Cell { row: 0, column: 1 }, RowId(10), products::NAME);
    frame(&ctx, &mut view, vec![]);
    frame(&ctx, &mut view, vec![egui::Event::Text(" retained".into())]);
    view.session.as_mut().unwrap().expires = web_time::Instant::now();
    frame(&ctx, &mut view, vec![]);
    assert_eq!(
        view.state.edit_draft().unwrap().after,
        "Remote product 10 retained"
    );
    assert!(view.dirty());
    view.password = "fixture-password".into();
    view.start_login(&ctx).unwrap();
    wait(&ctx, &mut view, |v| v.actions.available());
    // The input survives reauthentication and must be explicitly submitted.
    assert_eq!(
        view.state.edit_draft().unwrap().after,
        "Remote product 10 retained"
    );
    frame(&ctx, &mut view, vec![]);
    frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
    wait(&ctx, &mut view, |v| {
        !v.dirty() && v.actions.available() && v.pages.page(&v.address).is_some()
    });
    let requests = server.join().unwrap();
    let command: ActionCommand = requests
        .iter()
        .find(|request| {
            request
                .lines()
                .next()
                .unwrap()
                .contains("/products/actions")
        })
        .map(|request| serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap())
        .unwrap();
    assert_eq!(
        command.expected_version.as_deref(),
        Some("2026-09-09T00:00:00Z")
    );
    crate::http::runtime()
        .unwrap()
        .block_on(view.actions.store.as_ref().unwrap().close());
}

#[test]
fn remote_paste_review_saves_one_versioned_command_per_row_and_preserves_partial_results() {
    let root = tempfile::tempdir().unwrap();
    let profile = serde_json::json!({"user": {"user_id": RowId(2).to_uuid(), "tenant_id": RowId(1).to_uuid(), "status": "ACTIVE"}, "permissions": ["BASIC.SKU.READ", "BASIC.SKU.WRITE"]}).to_string();
    let (url, server) = server_with_conflict(vec![
        (200, serde_json::json!({"access_token":"fixture-token","token_type":"Bearer","expires_in":3600}).to_string()),
        (200, page(10,2,2)), (200, page(10,2,2)),
    ], Some(profile), true);
    let ctx = egui::Context::default();
    let mut view = RemoteProductView::default();
    view.actions.test_root = Some(root.path().to_owned());
    login(&mut view, url, &ctx);
    wait(&ctx, &mut view, |v| {
        v.actions.available() && v.pages.page(&v.address).is_some()
    });
    click(&ctx, &mut view, "Remote product 10");
    frame(
        &ctx,
        &mut view,
        vec![egui::Event::Paste(
            "New first\tBrand A\nNew second\tBrand B".into(),
        )],
    );
    assert!(view.paste.is_some());
    assert!(view.dirty());
    frame(&ctx, &mut view, vec![]);
    assert!(view.actions.runner.is_none());
    click(&ctx, &mut view, "Confirm");
    wait(&ctx, &mut view, |v| {
        v.actions
            .runner
            .as_ref()
            .is_some_and(|runner| runner.finished())
            && v.actions.available()
    });
    let items = view.actions.runner.as_ref().unwrap().items();
    assert_eq!(items.len(), 2);
    assert!(matches!(
        items[0].state(),
        ActionState::Complete(ActionOutcome::Rejected { .. })
    ));
    assert!(matches!(
        items[1].state(),
        ActionState::Complete(ActionOutcome::Applied { .. })
    ));
    assert!(view.dirty());
    click(&ctx, &mut view, "读取失败项最新版本");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let output = frame(&ctx, &mut view, vec![]);
        if output.shapes.iter().any(|shape| matches!(&shape.shape,egui::epaint::Shape::Text(text) if text.galley.text()=="确认按新版本重试")) { break; }
        assert!(
            std::time::Instant::now() < deadline,
            "retry preview did not load"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(view.actions.runner.as_ref().unwrap().items().len(), 2);
    click(&ctx, &mut view, "确认按新版本重试");
    wait(&ctx, &mut view, |v| {
        v.actions.available()
            && v.actions
                .runner
                .as_ref()
                .is_some_and(|runner| runner.items().len() == 1 && runner.finished())
    });
    assert!(matches!(
        view.actions.runner.as_ref().unwrap().items()[0].state(),
        ActionState::Complete(ActionOutcome::Applied { .. })
    ));
    click(&ctx, &mut view, "Close");
    wait(&ctx, &mut view, |v| {
        !v.dirty() && v.pages.page(&v.address).is_some()
    });
    let requests = server.join().unwrap();
    let commands: Vec<ActionCommand> = requests
        .iter()
        .filter(|request| {
            request
                .lines()
                .next()
                .unwrap()
                .contains("/products/actions")
        })
        .map(|request| serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap())
        .collect();
    assert_eq!(commands.len(), 3);
    assert!(commands[..2].iter().all(|command| command.values.len() == 2
        && command.expected_version.as_deref() == Some("2026-09-09T00:00:00Z")));
    assert_ne!(commands[0].operation_id, commands[1].operation_id);
    assert_eq!(commands[2].source, commands[0].source);
    assert_ne!(commands[2].operation_id, commands[0].operation_id);
    assert_eq!(commands[2].values, commands[0].values);
    assert_eq!(
        commands[2].expected_version.as_deref(),
        Some("2026-09-10T00:00:00Z")
    );
    crate::http::runtime()
        .unwrap()
        .block_on(view.actions.store.as_ref().unwrap().close());
}
