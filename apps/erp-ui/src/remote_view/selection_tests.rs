use super::*;

fn frozen_page(id: u128, count: usize, total: u64, token: RowId, expires: u64) -> String {
    let mut body: serde_json::Value = serde_json::from_str(&tests::page(id, count, total)).unwrap();
    body["token"] = token.to_uuid().into();
    body["expires_at"] = expires.into();
    body.to_string()
}

#[test]
fn remote_selection_freeze_page_exclusion_and_return_to_live_use_http() {
    let token = RowId(90);
    let expires = unix_seconds() + 600;
    let mut sorted: serde_json::Value =
        serde_json::from_str(&frozen_page(11, 2, 3, token, expires)).unwrap();
    sorted["data"].as_array_mut().unwrap().reverse();
    let (url, server) = tests::server(vec![
        (200, serde_json::json!({"access_token":"fixture-token","token_type":"Bearer","expires_in":3600}).to_string()),
        (200, tests::page(10, 2, 3)),
        (200, serde_json::json!({"token":token.to_uuid(),"query_fingerprint":"0123456789abcdef0123456789abcdef","total":3,"expires_at":expires}).to_string()),
        (200, frozen_page(10, 2, 3, token, expires)),
        (200, "{\"affected\":2}".into()),
        (200, frozen_page(12, 1, 3, token, expires)),
        (200, sorted.to_string()),
        (200, tests::page(50, 2, 3)),
    ]);
    let ctx = egui::Context::default();
    let mut view = RemoteProductView {
        address: PageAddress::Offset { start: 0, limit: 2 },
        ..Default::default()
    };
    tests::login(&mut view, url, &ctx);
    tests::wait(&ctx, &mut view, |v| v.pages.page(&v.address).is_some());
    view.start_frozen(&ctx);
    tests::wait(&ctx, &mut view, |v| {
        v.frozen.is_active() && v.pages.page(&v.address).is_some()
    });
    assert_eq!(view.frozen.count(), Some(3));
    assert_eq!(view.state.row_selection.checked().len(), 2);
    let checked = std::collections::BTreeSet::from([RowId(11)]);
    view.frozen
        .set_page_checked(
            view.session.as_ref().unwrap(),
            &[RowId(10), RowId(11)],
            &checked,
            &ctx,
            unix_seconds(),
        )
        .unwrap();
    assert_eq!(view.frozen.count(), None);
    tests::wait(&ctx, &mut view, |v| v.frozen.count() == Some(2));
    assert!(!view.state.row_selection.checked().contains(&RowId(10)));
    view.page_number = 1;
    view.address = PageAddress::Offset { start: 2, limit: 2 };
    tests::wait(&ctx, &mut view, |v| v.pages.page(&v.address).is_some());
    assert_eq!(
        view.state.row_selection.checked(),
        &std::collections::BTreeSet::from([RowId(12)])
    );
    assert_eq!(view.frozen.count(), Some(2));
    assert!(view.state.read_only);
    let sorts = vec![erp_grid_core::SortRule {
        column: ColumnId(1),
        descending: true,
    }];
    view.state.sorts = sorts.clone();
    view.reset_query(false);
    tests::wait(&ctx, &mut view, |v| v.pages.page(&v.address).is_some());
    assert_eq!(view.displayed_ids, vec![RowId(12), RowId(11)]);
    assert_eq!(view.frozen.credential().unwrap().token, token);
    assert_eq!(view.frozen.count(), Some(2));
    assert!(view.frozen.exclusions().contains(&RowId(10)));
    view.clear_frozen();
    tests::wait(&ctx, &mut view, |v| v.pages.page(&v.address).is_some());
    assert_eq!(view.displayed_ids, vec![RowId(50), RowId(51)]);
    assert!(view.state.row_selection.checked().is_empty());
    assert_eq!(view.state.sorts, sorts);
    let requests = server.join().unwrap();
    let selection_requests: Vec<_> = requests
        .iter()
        .filter(|request| request.lines().next().unwrap().contains("/selections"))
        .collect();
    assert_eq!(selection_requests.len(), 5);
    let sorted_request: serde_json::Value = serde_json::from_str(
        selection_requests
            .last()
            .unwrap()
            .split_once("\r\n\r\n")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(
        sorted_request["sorts"],
        serde_json::json!([{"column":1,"descending":true}])
    );
    let count = selection_requests
        .iter()
        .find(|request| request.lines().next().unwrap().contains("/count"))
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_str(count.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["excluded"], serde_json::json!([RowId(10).to_uuid()]));
}

#[test]
fn frozen_find_transition_cancels_old_window_and_pending_focus() {
    let mut host = RemoteProductView {
        find_pending: Some(super::find::Hit {
            row_id: RowId(1),
            column: erp_grid_core::ColumnId(0),
            offset: 20,
            wrapped: false,
        }),
        ..Default::default()
    };
    host.find.open();
    host.invalidate_selection_page();
    assert!(!host.find.is_open());
    assert!(host.find_pending.is_none());
}
