use super::{Session, details::editing::decode_one};
use crate::{
    edit_drafts::{DraftCell, DraftSnapshot},
    http::{HttpError, HttpRequest, HttpTask},
    remote_paste::{self, Preview},
    remote_products,
};
use eframe::egui;
use erp_grid_core::actions::{ActionCommand, ActionState};
use erp_grid_core::{CellEdit, Column, EditKind, RowId};
use std::collections::BTreeMap;

const MAX_BYTES: usize = 8 * 1024 * 1024;

struct Pending {
    index: usize,
    task: HttpTask,
}

pub(super) struct Restore {
    snapshot: DraftSnapshot,
    columns: Vec<Column>,
    parent: Option<RowId>,
    preview: Preview,
    latest: Vec<erp_grid_core::remote::RemoteRow>,
    pending: Option<Pending>,
    cursor: usize,
    bytes: usize,
    failed: Option<String>,
    permission_lost: bool,
}

impl Restore {
    pub(super) fn new(snapshot: DraftSnapshot, columns: &[Column]) -> Result<Self, String> {
        if snapshot.rows.is_empty() {
            return Err("草稿没有可恢复单元格".into());
        }
        let parent = snapshot.rows[0].parent;
        if snapshot.rows.iter().any(|cell| cell.parent != parent) {
            return Err("草稿包含不同父记录".into());
        }
        let action = if parent.is_some() {
            "skus.update"
        } else {
            "products.update"
        };
        let by_row = grouped_cells(&snapshot.rows)?;
        let mut commands = Vec::with_capacity(by_row.len());
        let mut edits = Vec::with_capacity(snapshot.rows.len());
        for (row, cells) in by_row {
            let version = cells[0].expected_version.clone();
            let mut values = BTreeMap::new();
            for cell in cells {
                let column = columns
                    .iter()
                    .find(|column| column.id == cell.column)
                    .ok_or_else(|| "草稿列已不存在".to_string())?;
                if matches!(column.edit_kind, EditKind::ReadOnly) {
                    return Err("草稿包含不可编辑列".into());
                }
                values.insert(cell.column, cell.after.clone());
                edits.push(CellEdit {
                    row: cell.row,
                    column: cell.column,
                    before: cell.before.clone(),
                    after: cell.after.clone(),
                });
            }
            let command = ActionCommand {
                operation_id: RowId(uuid::Uuid::new_v4().as_u128()),
                action: action.into(),
                source: row,
                expected_version: Some(version),
                values,
            };
            command.validate().map_err(String::from)?;
            commands.push(command);
        }
        Ok(Self {
            snapshot,
            columns: columns.to_vec(),
            parent,
            preview: Preview { commands, edits },
            latest: Vec::new(),
            pending: None,
            cursor: 0,
            bytes: 0,
            failed: None,
            permission_lost: false,
        })
    }

    pub(super) fn progress(&self) -> (usize, usize) {
        (
            self.cursor.min(self.preview.commands.len()),
            self.preview.commands.len(),
        )
    }

    pub(super) fn permission_lost(&self) -> bool {
        self.permission_lost
    }

    pub(super) fn tick(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
    ) -> Result<Option<Preview>, String> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if session.expires <= web_time::Instant::now() {
            self.permission_lost = true;
            return self.fail("认证已失效，草稿恢复已暂停".into());
        }
        let now = match web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(_) => return self.fail("无法确认草稿有效期".into()),
        };
        if self.snapshot.scope.server != session.client.base_url()
            || self.snapshot.scope.tenant != session.tenant
            || self.snapshot.expires_at <= now
        {
            return self.fail("草稿作用域或有效期不匹配".into());
        }
        let completed = self
            .pending
            .as_mut()
            .and_then(|pending| pending.task.poll().map(|result| (pending.index, result)));
        if let Some((index, result)) = completed {
            self.pending = None;
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(error) => {
                    let message = match error {
                        HttpError::Status(401 | 403) => {
                            self.permission_lost = true;
                            "认证或权限已失效，草稿恢复已暂停".to_string()
                        }
                        error => error.to_string(),
                    };
                    return self.fail(message);
                }
            };
            let Some(total_bytes) = self.bytes.checked_add(bytes.len()) else {
                return self.fail("恢复数据超出 8 MiB".into());
            };
            self.bytes = total_bytes;
            if self.bytes > MAX_BYTES {
                return self.fail("恢复数据超出 8 MiB".into());
            }
            let row = match self.decode_row(session, index, &bytes) {
                Ok(row) => row,
                Err(error) => return self.fail(error),
            };
            self.latest.push(row);
            self.cursor += 1;
        }
        if self.pending.is_some() {
            return Ok(None);
        }
        if self.cursor < self.preview.commands.len() {
            let row = self.preview.commands[self.cursor].source;
            let request = HttpRequest {
                path: if self.parent.is_some() {
                    format!(
                        "/v1/tenants/{}/skus/{}",
                        session.tenant.to_uuid(),
                        row.to_uuid()
                    )
                } else {
                    format!(
                        "/v1/tenants/{}/products/{}",
                        session.tenant.to_uuid(),
                        row.to_uuid()
                    )
                },
                query: vec![],
                json: None,
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            };
            let task = match session.client.start(request, ctx) {
                Ok(task) => task,
                Err(error) => return self.fail(error.to_string()),
            };
            self.pending = Some(Pending {
                index: self.cursor,
                task,
            });
            return Ok(None);
        }
        let outcomes = self
            .preview
            .commands
            .iter()
            .map(|command| (command.operation_id, ActionState::Cancelled))
            .collect();
        match remote_paste::prepare_retry(&self.preview, &outcomes, &self.latest, &self.columns) {
            Ok(preview) => Ok(Some(preview)),
            Err(error) => self.fail(error),
        }
    }

    fn decode_row(
        &self,
        session: &Session,
        index: usize,
        bytes: &[u8],
    ) -> Result<erp_grid_core::remote::RemoteRow, String> {
        let row = self.preview.commands[index].source;
        let decoded = if let Some(parent) = self.parent {
            decode_one(session.tenant, parent, row, bytes)?
        } else {
            remote_products::decode_product(session.tenant, row, bytes)?
        };
        if decoded.id != row {
            return Err("恢复记录身份不匹配".into());
        }
        Ok(decoded)
    }

    fn fail<T>(&mut self, error: String) -> Result<T, String> {
        self.failed = Some(error.clone());
        self.pending = None;
        Err(error)
    }
}

fn grouped_cells(rows: &[DraftCell]) -> Result<BTreeMap<RowId, Vec<DraftCell>>, String> {
    let mut grouped = BTreeMap::new();
    for cell in rows {
        if cell.row.0 == 0 || cell.expected_version.is_empty() {
            return Err("草稿身份或版本无效".into());
        }
        let cells = grouped.entry(cell.row).or_insert_with(Vec::new);
        if cells
            .iter()
            .any(|other: &DraftCell| other.expected_version != cell.expected_version)
        {
            return Err("同一行草稿版本不一致".into());
        }
        cells.push(cell.clone());
    }
    Ok(grouped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::ColumnId;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::{Duration, Instant},
    };

    fn snapshot(parent: Option<RowId>) -> DraftSnapshot {
        DraftSnapshot {
            scope: crate::edit_drafts::DraftScope {
                server: "https://example.test/".into(),
                tenant: RowId(1),
                actor: RowId(2),
                grid: "products".into(),
                schema: 1,
            },
            saved_at: 1,
            expires_at: 2,
            rows: vec![DraftCell {
                row: RowId(10),
                column: ColumnId(1),
                before: "old".into(),
                after: "target".into(),
                expected_version: "v1".into(),
                parent,
            }],
        }
    }

    #[test]
    fn new_rebases_commands_from_snapshot_without_dispatching() {
        let mut name = Column::new(ColumnId(1), "Name", 100.0);
        name.edit_kind = EditKind::Text;
        let columns = vec![name];
        let restore = Restore::new(snapshot(None), &columns).unwrap();
        assert_eq!(restore.progress(), (0, 1));
        assert_eq!(
            restore.preview.commands[0].expected_version.as_deref(),
            Some("v1")
        );
        assert_eq!(
            restore.preview.commands[0].values.get(&ColumnId(1)),
            Some(&"target".into())
        );
        assert!(restore.pending.is_none());
    }

    #[test]
    fn new_rejects_mixed_parent_or_row_versions() {
        let mut name = Column::new(ColumnId(1), "Name", 100.0);
        name.edit_kind = EditKind::Text;
        let columns = vec![name];
        let mut mixed = snapshot(Some(RowId(9)));
        mixed.rows.push(DraftCell {
            row: RowId(11),
            column: ColumnId(1),
            before: "old".into(),
            after: "target".into(),
            expected_version: "v1".into(),
            parent: Some(RowId(10)),
        });
        assert!(Restore::new(mixed, &columns).is_err());
        let mut version = snapshot(None);
        version.rows.push(DraftCell {
            row: RowId(10),
            column: ColumnId(2),
            before: "old".into(),
            after: "target".into(),
            expected_version: "v2".into(),
            parent: None,
        });
        let mut name = Column::new(ColumnId(1), "Name", 100.0);
        name.edit_kind = EditKind::Text;
        let mut brand = Column::new(ColumnId(2), "Brand", 100.0);
        brand.edit_kind = EditKind::Text;
        let columns = vec![name, brand];
        assert!(Restore::new(version, &columns).is_err());
    }

    fn fixture_server(body: String) -> (crate::http::HttpClient, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let client =
            crate::http::HttpClient::new(&format!("http://{}/erp", listener.local_addr().unwrap()))
                .unwrap();
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                    Err(error) => panic!("missing restore request: {error}"),
                }
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).unwrap();
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
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
            }
            let request = String::from_utf8(bytes).unwrap();
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            vec![request]
        });
        (client, server)
    }

    fn snapshot_for(server: String, parent: Option<RowId>) -> DraftSnapshot {
        let now = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        DraftSnapshot {
            scope: crate::edit_drafts::DraftScope {
                server,
                tenant: RowId(1),
                actor: RowId(2),
                grid: "products".into(),
                schema: 1,
            },
            saved_at: now,
            expires_at: now + 7 * 24 * 60 * 60,
            rows: vec![DraftCell {
                row: RowId(10),
                column: ColumnId(1),
                before: "Old".into(),
                after: "Target".into(),
                expected_version: "v-old".into(),
                parent,
            }],
        }
    }

    #[tokio::test]
    async fn draft_restore_permission_loss_requests_reauthentication() {
        let (client, server) =
            crate::remote_actions::tests::server(vec![Some((403, "Forbidden".into()))]);
        let mut column = Column::new(ColumnId(1), "Name", 100.0);
        column.edit_kind = EditKind::Text;
        let mut restore =
            Restore::new(snapshot_for(client.base_url().into(), None), &[column]).unwrap();
        let session = Session {
            client,
            tenant: RowId(1),
            token: "fixture".into(),
            expires: web_time::Instant::now() + Duration::from_secs(60),
        };
        let ctx = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < deadline);
            match restore.tick(&session, &ctx) {
                Err(_) => break,
                Ok(None) => tokio::time::sleep(Duration::from_millis(3)).await,
                Ok(Some(_)) => panic!("permission loss must not produce a preview"),
            }
        }
        assert!(restore.permission_lost());
        assert!(restore.pending.is_none());
        assert_eq!(server.join().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tick_reads_latest_product_and_preserves_target_without_post() {
        let body = serde_json::json!({"product_id": RowId(10).to_uuid(), "tenant_id": RowId(1).to_uuid(), "name": "Latest", "status": "ACTIVE", "created_at": "2026-09-10T00:00:00Z", "updated_at": "v-new"}).to_string();
        let (client, server) = fixture_server(body);
        let mut column = Column::new(ColumnId(1), "Name", 100.0);
        column.edit_kind = EditKind::Text;
        let mut restore =
            Restore::new(snapshot_for(client.base_url().to_string(), None), &[column]).unwrap();
        let session = Session {
            client,
            tenant: RowId(1),
            token: "token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(60),
        };
        let ctx = egui::Context::default();
        let preview = loop {
            if let Some(preview) = restore.tick(&session, &ctx).unwrap() {
                break preview;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        };
        assert_eq!(
            preview.commands[0].expected_version.as_deref(),
            Some("v-new")
        );
        assert_eq!(
            preview.commands[0].values.get(&ColumnId(1)),
            Some(&"Target".into())
        );
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(
            requests[0]
                .starts_with("GET /erp/v1/tenants/00000000-0000-0000-0000-000000000001/products/")
        );
        assert!(!requests[0].starts_with("POST"));
    }

    #[tokio::test]
    async fn tick_rejects_sku_parent_identity_mismatch() {
        let body = serde_json::json!({"sku_id": RowId(10).to_uuid(), "tenant_id": RowId(1).to_uuid(), "product_id": RowId(99).to_uuid(), "sku_code": "SKU", "barcode": null, "name": "Latest", "sku_kind": "NORMAL", "cost_cent": 100, "is_active": true, "status": "ACTIVE", "created_at": "2026-09-10T00:00:00Z", "updated_at": "v-new"}).to_string();
        let (client, server) = fixture_server(body);
        let mut column = Column::new(ColumnId(1), "Name", 100.0);
        column.edit_kind = EditKind::Text;
        let mut restore = Restore::new(
            snapshot_for(client.base_url().to_string(), Some(RowId(9))),
            &[column],
        )
        .unwrap();
        let session = Session {
            client,
            tenant: RowId(1),
            token: "token".into(),
            expires: web_time::Instant::now() + Duration::from_secs(60),
        };
        let result = loop {
            match restore.tick(&session, &egui::Context::default()) {
                Ok(None) => tokio::time::sleep(Duration::from_millis(5)).await,
                other => break other,
            }
        };
        assert!(result.is_err());
        assert!(restore.progress().0 < restore.progress().1);
        let requests = server.join().unwrap();
        assert!(
            requests[0]
                .starts_with("GET /erp/v1/tenants/00000000-0000-0000-0000-000000000001/skus/")
        );
    }
}
