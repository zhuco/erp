use super::*;
use std::io::{BufRead, Read, Write};

#[test]
fn options_http_loads_full_catalogue_and_preserves_existing_text_semantics() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
        let mut first = String::new();
        reader.read_line(&mut first).unwrap();
        assert!(first.contains("/products/options"));
        let mut length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
        }
        assert!(length <= 32768);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["column"], 2);
        assert!(body["query"]["filters"].get("2").is_none());
        assert_eq!(body["query"]["filters"]["3"]["Text"], "Tools");
        let response = r#"{"values":["","Acme","Remote"],"total":3}"#;
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.len(),
            response
        )
        .unwrap();
    });
    let session = Session {
        client: HttpClient::new(&url).unwrap(),
        tenant: RowId(1),
        token: "test".into(),
        expires: web_time::Instant::now() + Duration::from_secs(60),
    };
    let query = RemoteQuery {
        expression: None,
        scope: DataScope {
            source: "products:1".into(),
            schema: 1,
            authorization: "session:1".into(),
        },
        filters: BTreeMap::from([
            (ColumnId(2), erp_grid_core::Filter::Text("Old".into())),
            (ColumnId(3), erp_grid_core::Filter::Text("Tools".into())),
        ]),
        sorts: vec![],
        search: None,
    };
    let mut options = ProductOptions::default();
    let ctx = egui::Context::default();
    for _ in 0..200 {
        assert!(!options.tick(&ctx, &session, &query));
        if options.entries[&ColumnId(2)].directory.complete_dropdown() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    worker.join().unwrap();
    assert!(options.entries[&ColumnId(2)].directory.complete_dropdown());
    let mut columns = vec![Column::new(ColumnId(2), "Brand", 100.0)];
    let mut drafts = BTreeMap::from([(ColumnId(2), FilterDraft::Text("Ac".into()))]);
    options.update_columns(&mut columns, &mut drafts);
    assert_eq!(columns[0].filter, FilterKind::Text);
    drafts.clear();
    options.update_columns(&mut columns, &mut drafts);
    assert_eq!(
        columns[0].filter,
        FilterKind::Enum(vec!["".into(), "Acme".into(), "Remote".into()])
    );
    options.clear();
    assert!(options.entries[&ColumnId(2)].directory.values.is_empty());
}
