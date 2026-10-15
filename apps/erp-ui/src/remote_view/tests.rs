use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Instant,
};

pub(super) fn server(responses: Vec<(u16, String)>) -> (String, thread::JoinHandle<Vec<String>>) {
    server_with_profile(responses, None)
}
fn server_with_profile(
    responses: Vec<(u16, String)>,
    profile: Option<String>,
) -> (String, thread::JoinHandle<Vec<String>>) {
    server_with_conflict(responses, profile, false)
}
pub(super) fn server_with_conflict(
    responses: Vec<(u16, String)>,
    profile: Option<String>,
    mut conflict: bool,
) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let task = thread::spawn(move || {
        let mut requests = Vec::new();
        let mut saved: Option<erp_grid_core::actions::ActionCommand> = None;
        let mut responses = responses.into_iter().peekable();
        while responses.peek().is_some() {
            let deadline = Instant::now() + Duration::from_secs(8);
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                    Err(error) => panic!("Missing HTTP request: {error}"),
                }
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 2048];
            loop {
                let read = socket.read(&mut buffer).unwrap();
                assert!(read > 0);
                request.extend_from_slice(&buffer[..read]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let content_length = header
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + content_length {
                        break;
                    }
                }
                assert!(request.len() < 32_768);
            }
            let request = String::from_utf8(request).unwrap();
            let path = request
                .split_whitespace()
                .nth(1)
                .unwrap()
                .split('?')
                .next()
                .unwrap();
            let (status, body) = if request.starts_with("GET ") && path == "/me" {
                profile
                    .clone()
                    .map(|body| (200, body))
                    .unwrap_or((403, "{}".into()))
            } else if request.starts_with("POST ") && path.ends_with("/skus/query") {
                (200, "{\"data\":[],\"total\":0}".into())
            } else if request.starts_with("POST ") && path.ends_with("/products/options") {
                (200, "{\"values\":[],\"total\":0}".into())
            } else if profile.is_some() && path.ends_with("/products/actions") {
                use erp_grid_core::actions::{ActionCommand, ActionOutcome, ActionReceipt};
                let command: ActionCommand =
                    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
                let receipt = ActionReceipt {
                    operation_id: command.operation_id,
                    source: command.source,
                    command_fingerprint: command.fingerprint(),
                    outcome: if command.action == "products.update" && std::mem::take(&mut conflict)
                    {
                        ActionOutcome::Rejected {
                            code: "VERSION_CONFLICT".into(),
                            message: "Changed by another user".into(),
                        }
                    } else {
                        ActionOutcome::Applied {
                            row: if command.action == "products.copy" {
                                RowId(200)
                            } else {
                                command.source
                            },
                            version: "2026-09-10T00:00:00Z".into(),
                            values: Default::default(),
                        }
                    },
                };
                if command.action == "products.update"
                    && matches!(receipt.outcome, ActionOutcome::Applied { .. })
                {
                    saved = Some(command);
                }
                requests.push(request);
                (200, serde_json::to_string(&receipt).unwrap())
            } else if profile.is_some()
                && request.starts_with("GET ")
                && path.contains("/products/")
            {
                let mut latest: serde_json::Value = serde_json::from_str(&page(10, 1, 1)).unwrap();
                latest["data"][0]["name"] = "Other user's value".into();
                latest["data"][0]["updated_at"] = "2026-09-10T00:00:00Z".into();
                requests.push(request);
                (200, latest["data"][0].to_string())
            } else {
                let mut response = responses.next().unwrap();
                if path.ends_with("/products/query")
                    && let Some(command) = &saved
                {
                    let mut body: serde_json::Value = serde_json::from_str(&response.1).unwrap();
                    for row in body["data"].as_array_mut().unwrap() {
                        if row["product_id"] == command.source.to_uuid() {
                            for (column, value) in &command.values {
                                row[["product_code", "name", "brand", "category"]
                                    [column.0 as usize]] = value.clone().into();
                            }
                            row["updated_at"] = "2026-09-10T00:00:00Z".into();
                        }
                    }
                    response.1 = body.to_string();
                }
                requests.push(request);
                response
            };
            write!(socket, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    (url, task)
}
pub(super) fn page(id: u128, count: usize, total: u64) -> String {
    let data: Vec<_> = (0..count).map(|index| serde_json::json!({"product_id": RowId(id + index as u128).to_uuid(), "tenant_id": RowId(1).to_uuid(), "name": format!("Remote product {}", id + index as u128), "status": "ACTIVE", "created_at": "2026-09-09T00:00:00Z", "updated_at": "2026-09-09T00:00:00Z"})).collect();
    serde_json::json!({"data": data, "total": total}).to_string()
}
pub(super) fn frame(
    ctx: &egui::Context,
    view: &mut RemoteProductView,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| view.ui(ui));
        },
    );
    output.textures_delta.clear();
    output
}
#[track_caller]
pub(super) fn wait(
    ctx: &egui::Context,
    view: &mut RemoteProductView,
    predicate: impl Fn(&RemoteProductView) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        frame(ctx, view, vec![]);
        if predicate(view) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Remote UI did not converge: {:?}, page {:?}, address {:?}, jobs {}, cache {}, editor {:?}, action states {:?}",
            view.notice,
            view.page_number,
            view.address,
            view.jobs.len(),
            view.pages.cached_rows(),
            view.state.message,
            view.actions.runner.as_ref().map(|runner| runner
                .items()
                .iter()
                .map(|item| item.state())
                .collect::<Vec<_>>())
        );
        thread::sleep(Duration::from_millis(5));
    }
}
pub(super) fn login(view: &mut RemoteProductView, url: String, ctx: &egui::Context) {
    view.server = url;
    view.tenant = RowId(1).to_uuid();
    view.email = "fixture@example.test".into();
    view.password = "fixture-password".into();
    view.start_login(ctx).unwrap();
    assert!(view.password.is_empty());
}

#[test]
fn copy_button_uses_checked_source_refreshes_results_and_reopens_durable_receipt() {
    use crate::{
        action_store::ActionStore,
        remote_actions::{ActionRunner, ActionScope},
    };
    use erp_grid_core::actions::{ActionCommand, ActionOutcome, ActionState};
    let root = tempfile::tempdir().unwrap();
    let profile = serde_json::json!({"user": {"user_id": RowId(2).to_uuid(), "tenant_id": RowId(1).to_uuid(), "status": "ACTIVE"}, "permissions": ["BASIC.SKU.READ", "BASIC.SKU.WRITE"]}).to_string();
    let (url, server) = server_with_profile(vec![
        (200, serde_json::json!({"access_token": "fixture-token", "token_type": "Bearer", "expires_in": 3600}).to_string()),
        (200, page(10, 1, 1)),
        (200, page(200, 2, 2)),
    ], Some(profile));
    let ctx = egui::Context::default();
    let mut view = RemoteProductView::default();
    view.actions.test_root = Some(root.path().to_owned());
    login(&mut view, url, &ctx);
    wait(&ctx, &mut view, |v| {
        v.actions.available() && v.pages.page(&v.address).is_some()
    });
    // A checked source overrides the unrelated active row, including across pages.
    view.state.row_selection.active = Some(RowId(99));
    view.state.row_selection.toggle(RowId(10));
    let output = frame(&ctx, &mut view, vec![]);
    let more = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape
                && text.galley.text() == "更多操作"
            {
                Some(text.pos + text.galley.size() / 2.0)
            } else {
                None
            }
        })
        .expect("more actions control is rendered");
    frame(&ctx, &mut view, vec![egui::Event::PointerMoved(more)]);
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut view,
            vec![egui::Event::PointerButton {
                pos: more,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    assert!(
        view.advanced_tools,
        "actual pointer click expands advanced actions"
    );
    let output = frame(&ctx, &mut view, vec![]);
    let point = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape
                && text.galley.text() == "复制商品"
            {
                Some(text.pos + text.galley.size() / 2.0)
            } else {
                None
            }
        })
        .expect("copy button is rendered");
    frame(&ctx, &mut view, vec![egui::Event::PointerMoved(point)]);
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut view,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    wait(&ctx, &mut view, |v| {
        v.actions
            .runner
            .as_ref()
            .is_some_and(ActionRunner::finished)
            && v.pages
                .page(&v.address)
                .is_some_and(|page| page.rows[0].id == RowId(200))
    });
    let requests = server.join().unwrap();
    let actions: Vec<_> = requests
        .iter()
        .filter(|request| {
            request
                .split_whitespace()
                .nth(1)
                .unwrap()
                .split('?')
                .next()
                .unwrap()
                .ends_with("/products/actions")
        })
        .collect();
    assert_eq!(
        actions.len(),
        1,
        "One button click must not duplicate products"
    );
    let command: ActionCommand =
        serde_json::from_str(actions[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(command.source, RowId(10));
    assert_eq!(command.action, "products.copy");
    assert!(command.values.is_empty());
    let scope =
        ActionScope::new(&view.session.as_ref().unwrap().client, RowId(1), RowId(2)).unwrap();
    crate::http::runtime()
        .unwrap()
        .block_on(view.actions.store.as_ref().unwrap().close());
    drop(view);
    let mut task = ActionStore::open_at(root.path().to_owned(), scope.clone(), &ctx).unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    let stored = loop {
        if let Some(result) = task.poll() {
            break result.unwrap();
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    };
    let bytes = stored.bytes.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("fixture-token"));
    let recovered = ActionRunner::restore(scope, &bytes).unwrap();
    assert_eq!(recovered.items()[0].command(), &command);
    assert!(matches!(
        recovered.items()[0].state(),
        ActionState::Complete(ActionOutcome::Applied {
            row: RowId(200),
            ..
        })
    ));
    crate::http::runtime()
        .unwrap()
        .block_on(stored.store.close());
}

#[test]
fn actual_login_paging_and_search_drive_grid_without_local_catalogue() {
    let (url, server) = server(vec![
        (200, serde_json::json!({"access_token": "fixture-token", "token_type": "Bearer", "expires_in": 3600}).to_string()),
        (200, page(1, 100, 101)), (200, page(101, 1, 101)), (200, page(3, 1, 1)),
    ]);
    let ctx = egui::Context::default();
    let mut view = RemoteProductView::default();
    login(&mut view, url, &ctx);
    wait(&ctx, &mut view, |v| v.pages.page(&v.address).is_some());
    assert_eq!(view.pages.cached_rows(), 100);
    assert_eq!(
        view.pages.page(&view.address).unwrap().total,
        RowCount::Exact(101)
    );
    view.state.row_selection.toggle(RowId(1));
    let output = frame(&ctx, &mut view, vec![]);
    let mut next_button = None;
    let mut rendered_product = false;
    for shape in &output.shapes {
        if let egui::epaint::Shape::Text(text) = &shape.shape {
            if text.galley.text() == ">" && next_button.is_none() {
                next_button = Some(text.pos + text.galley.size() / 2.0);
            }
            if text.galley.text() == "Remote product 1"
                && shape
                    .clip_rect
                    .contains(text.pos + text.galley.size() / 2.0)
            {
                rendered_product = true;
            }
        }
    }
    assert!(rendered_product);
    let point = next_button.unwrap();
    frame(&ctx, &mut view, vec![egui::Event::PointerMoved(point)]);
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut view,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    wait(&ctx, &mut view, |v| {
        v.page_number == 1 && v.pages.page(&v.address).is_some()
    });
    assert_eq!(
        view.pages.page(&view.address).unwrap().rows[0].id,
        RowId(101)
    );
    view.state.row_selection.toggle(RowId(101));
    assert_eq!(view.state.row_selection.targets(), [RowId(1), RowId(101)]);
    view.search = "Unique & 中文".into();
    view.reset_query(true);
    wait(&ctx, &mut view, |v| v.pages.page(&v.address).is_some());
    assert!(view.state.row_selection.checked().is_empty());
    assert_eq!(view.pages.cached_rows(), 1);
    assert_eq!(view.pages.page(&view.address).unwrap().rows[0].id, RowId(3));
    let requests = server.join().unwrap();
    assert!(requests[0].starts_with("POST /auth/login"));
    assert!(requests[0].contains("fixture@example.test"));
    assert!(!requests[0].to_lowercase().contains("authorization:"));
    assert!(requests[1].starts_with(&format!(
        "POST /v1/tenants/{}/products/query",
        RowId(1).to_uuid()
    )));
    let query = |index: usize| {
        serde_json::from_str::<erp_grid_core::paged_query::PageQuery>(
            requests[index].split_once("\r\n\r\n").unwrap().1,
        )
        .unwrap()
    };
    assert_eq!((query(1).offset, query(1).limit), (0, 100));
    assert_eq!(query(2).offset, 100);
    assert_eq!(query(3).search.as_deref(), Some("Unique & 中文"));
    view.disconnect();
    assert!(view.search.is_empty());
    assert!(view.session.is_none() && view.jobs.is_empty() && view.login.is_none());
    assert_eq!(view.pages.cached_rows(), 0);
}

#[test]
fn rejected_login_never_creates_session_or_product_request() {
    let (url, server) = server(vec![(401, "private server diagnostic".into())]);
    let ctx = egui::Context::default();
    let mut view = RemoteProductView::default();
    login(&mut view, url, &ctx);
    wait(&ctx, &mut view, |v| v.notice.is_some());
    assert_eq!(view.notice.as_deref(), Some("HTTP 401"));
    assert!(view.session.is_none());
    assert_eq!(view.pages.cached_rows(), 0);
    assert_eq!(server.join().unwrap().len(), 1);
}

#[test]
fn product_query_sort_keeps_checks_but_filter_change_clears_even_during_debounce() {
    use erp_grid_core::{DateTimeParts, FilterDraft, SortRule};
    let mut view = RemoteProductView {
        session: Some(Session {
            client: HttpClient::new("http://127.0.0.1:15439").unwrap(),
            tenant: RowId(1),
            token: "fixture".into(),
            expires: web_time::Instant::now() + Duration::from_secs(60),
        }),
        ..Default::default()
    };
    view.reset_query(true);
    view.state.row_selection.toggle(RowId(50));
    view.state.sorts.push(SortRule {
        column: products::NAME,
        descending: true,
    });
    view.reset_query(false);
    assert_eq!(view.state.row_selection.targets(), [RowId(50)]);
    view.state
        .quick_search
        .insert(products::NAME, FilterDraft::Text("Needle".into()));
    view.reset_query(false);
    assert!(view.state.row_selection.checked().is_empty());
    assert_eq!(
        view.query.as_ref().unwrap().filters.get(&products::NAME),
        Some(&erp_grid_core::Filter::Text("needle".into()))
    );
    view.state.quick_search.insert(
        products::CREATED,
        FilterDraft::DateTime {
            start: Some(DateTimeParts {
                year: 2025,
                month: 2,
                day: 29,
                hour: 0,
                minute: 0,
                second: 0,
            }),
            end: None,
        },
    );
    view.reset_query(false);
    assert!(view.notice.is_some());
    assert_eq!(view.pages.cached_rows(), 0);
    assert!(view.jobs.is_empty());
}
