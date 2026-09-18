use super::*;
use erp_grid_core::{ColumnId, FilterDraft, SortRule};
use std::time::Duration;

fn sku_server() -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(_) if std::time::Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("Missing SKU request: {error}"),
                }
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                bytes.push(byte[0]);
                assert!(bytes.len() < 8192);
            }
            let header = String::from_utf8(bytes.clone()).unwrap();
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            assert!(length < 65536);
            let offset = bytes.len();
            bytes.resize(offset + length, 0);
            socket.read_exact(&mut bytes[offset..]).unwrap();
            let request = String::from_utf8(bytes).unwrap();
            let path = request.split_whitespace().nth(1).unwrap();
            let parent = path.split('/').nth(5).unwrap();
            let query: PageQuery =
                serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
            let rows: Vec<_> = (0..query.limit).map(|index| serde_json::json!({
                "sku_id": RowId(u128::from(query.offset) + u128::from(index) + 1).to_uuid(),
                "tenant_id": RowId(1).to_uuid(), "product_id": parent,
                "sku_code": format!("sku-x{index}"), "name": "Fixture", "barcode": null,
                "sku_kind": "NORMAL", "cost_cent": 100, "is_active": true, "status": "ACTIVE",
                "created_at": "2026-09-10T00:00:00Z", "updated_at": "2026-09-10T00:00:00Z"
            })).collect();
            let body = serde_json::json!({"data": rows, "total": 100}).to_string();
            requests.push(request);
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
        requests
    });
    (url, handle)
}

#[test]
fn sku_http_query_posts_scope_filters_sort_and_offset_then_parent_reset_clears_checks() {
    let (url, server) = sku_server();
    let session = Session {
        client: crate::http::HttpClient::new(&url).unwrap(),
        tenant: RowId(1),
        token: "fixture-token".into(),
        expires: web_time::Instant::now() + Duration::from_secs(60),
    };
    let ctx = egui::Context::default();
    let mut details = Details {
        parent: Some((7, RowId(10))),
        ..Default::default()
    };
    details.pager.set_page_size(20);
    details.pager.set_page(2, 100);
    details
        .state
        .quick_search
        .insert(ColumnId(0), FilterDraft::Text("sku-x".into()));
    details.state.sorts.push(SortRule {
        column: ColumnId(2),
        descending: true,
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while details.pages.page(&details.address()).is_none() {
        details.tick(&session, &ctx);
        assert!(details.error.is_none(), "{:?}", details.error);
        assert!(
            std::time::Instant::now() < deadline,
            "SKU request did not complete"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    details.state.row_selection.toggle(RowId(999));
    assert!(details.state.row_selection.checked().contains(&RowId(999)));
    details.reset(false);
    assert_eq!(
        details.address(),
        PageAddress::Offset {
            start: 40,
            limit: 20
        }
    );
    details.clear();
    details.parent = Some((7, RowId(11)));
    assert!(details.state.row_selection.checked().is_empty());
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while details.pages.page(&details.address()).is_none() {
        details.tick(&session, &ctx);
        assert!(details.error.is_none(), "{:?}", details.error);
        assert!(
            std::time::Instant::now() < deadline,
            "second SKU request did not complete"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!(
        "POST /v1/tenants/{}/products/{}/skus/query",
        RowId(1).to_uuid(),
        RowId(10).to_uuid()
    )));
    let query: erp_grid_core::paged_query::PageQuery =
        serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!((query.offset, query.limit), (40, 20));
    assert_eq!(query.search, None);
    assert_eq!(
        query.filters.get(&ColumnId(0)),
        Some(&erp_grid_core::Filter::Text("sku-x".into()))
    );
    assert_eq!(
        query.sorts,
        vec![SortRule {
            column: ColumnId(2),
            descending: true
        }]
    );
    assert!(requests[1].contains(&format!("/products/{}/skus/query", RowId(11).to_uuid())));
}

#[test]
fn sku_decode_rejects_wrong_tenant_and_parent() {
    let body = serde_json::json!({"data": [{
        "sku_id": RowId(3).to_uuid(), "tenant_id": RowId(1).to_uuid(), "product_id": RowId(2).to_uuid(),
        "sku_code": "SKU-3", "barcode": null, "name": null, "sku_kind": "NORMAL",
        "cost_cent": 1, "is_active": true, "status": "ACTIVE",
        "created_at": "2026-09-10T00:00:00Z", "updated_at": "2026-09-10T00:00:00Z"
    }], "total": 1});
    let address = PageAddress::Offset {
        start: 0,
        limit: 100,
    };
    let bytes = serde_json::to_vec(&body).unwrap();
    assert!(decode(RowId(9), RowId(2), &bytes, &address).is_err());
    assert!(decode(RowId(1), RowId(9), &bytes, &address).is_err());
}
