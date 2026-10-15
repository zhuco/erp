//! Actual HTTP + shared page state + renderer under a synthetic million-row server.
use crate::http::{HttpClient, HttpRequest, HttpTask};
use eframe::egui;
use erp_grid_core::{Cell, Column, ColumnId, RowId, remote::*};
use erp_ui_egui::{DataGrid, GridState};
use std::collections::BTreeMap;
use web_time::Instant;

#[derive(serde::Deserialize)]
struct WirePage {
    total: u64,
    offset: u64,
    data: Vec<WireRow>,
}
#[derive(serde::Deserialize)]
struct WireRow {
    id: u64,
    version: String,
    code: String,
    value: u64,
}

pub struct PageLab {
    client: HttpClient,
    pages: RemotePages,
    jobs: Vec<(PageRequest, HttpTask, bool)>,
    columns: Vec<Column>,
    grid: GridState,
    address: PageAddress,
    cycle: usize,
    phase: u8,
    completed_at: Option<Instant>,
    frame: Option<u64>,
    samples: Vec<f64>,
    stale: usize,
    bytes: usize,
    error: Option<String>,
    finished: bool,
}
impl PageLab {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        crate::fonts::install(&cc.egui_ctx);
        let mut pages = RemotePages::default();
        pages.bind(RemoteQuery {
            scope: DataScope {
                source: "million-row-fixture".into(),
                schema: 1,
                authorization: "synthetic-only".into(),
            },
            expression: None,
            filters: BTreeMap::new(),
            sorts: vec![],
            search: None,
        });
        let mut lab = Self {
            client: HttpClient::new("http://127.0.0.1:15444").map_err(|e| e.to_string())?,
            pages,
            jobs: vec![],
            columns: vec![
                Column::new(ColumnId(0), "Code", 220.0),
                Column::new(ColumnId(1), "Value", 160.0),
            ],
            grid: GridState::default(),
            address: PageAddress::Offset {
                start: 0,
                limit: 100,
            },
            cycle: 0,
            phase: 0,
            completed_at: None,
            frame: None,
            samples: vec![],
            stale: 0,
            bytes: 0,
            error: None,
            finished: false,
        };
        lab.begin(&cc.egui_ctx)?;
        Ok(lab)
    }
    fn request(&mut self, offset: u64, old: bool, ctx: &egui::Context) -> Result<(), String> {
        let PageStart::Fetch(request) = self.pages.start(PageAddress::Offset {
            start: offset,
            limit: 100,
        })?
        else {
            return Err("Expected uncached fixture request".into());
        };
        let task = self
            .client
            .start(
                HttpRequest {
                    path: "/page".into(),
                    query: vec![("delay_ms".into(), if old { "250" } else { "0" }.into())],
                    json: Some(
                        serde_json::to_vec(
                            &serde_json::json!({"schema":1,"offset":offset,"limit":100}),
                        )
                        .map_err(|e| e.to_string())?,
                    ),
                    bearer: None,
                    idempotency_key: None,
                },
                ctx,
            )
            .map_err(|e| e.to_string())?;
        self.jobs.push((request, task, old));
        Ok(())
    }
    fn begin(&mut self, ctx: &egui::Context) -> Result<(), String> {
        self.pages.invalidate();
        let offset = ((self.cycle * 283) % 9999) as u64 * 100;
        self.request(offset, true, ctx)?;
        // Deliberately retain the old transport to simulate cancellation not honored.
        // Production hosts should cancel it; core generation isolation must stand alone.
        self.pages.invalidate();
        self.address = PageAddress::Offset {
            start: offset + 100,
            limit: 100,
        };
        self.grid = GridState::default();
        self.grid.row_number_start = offset + 100;
        self.phase = 0;
        self.completed_at = None;
        self.request(offset + 100, false, ctx)
    }
    fn decode(request: &PageRequest, bytes: &[u8]) -> Result<RemotePage, String> {
        let page: WirePage = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let PageAddress::Offset { start, limit } = request.address else {
            return Err("Offset required".into());
        };
        if page.total != 1_000_000 || page.offset != start || page.data.len() != usize::from(limit)
        {
            return Err("Fixture page scope/length mismatch".into());
        }
        let mut rows = Vec::with_capacity(page.data.len());
        for (index, row) in page.data.into_iter().enumerate() {
            if row.id != start + index as u64 + 1 || row.value != row.id || row.version != "1" {
                return Err("Fixture identity/version mismatch".into());
            }
            rows.push(RemoteRow {
                id: RowId(u128::from(row.id)),
                version: Some(row.version),
                cells: BTreeMap::from([
                    (ColumnId(0), row.code),
                    (ColumnId(1), row.value.to_string()),
                ]),
            });
        }
        Ok(RemotePage {
            rows,
            total: RowCount::Exact(page.total),
            snapshot: None,
            next_cursor: None,
        })
    }
    fn finish(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        let _ = ctx; // Browser results are published on the document, not a viewport command.
        let mut sorted = self.samples.clone();
        sorted.sort_by(f64::total_cmp);
        let p95 = sorted.get((sorted.len() * 95).div_ceil(100).saturating_sub(1));
        let stats = self.pages.stats();
        let report=serde_json::json!({"scenario":"PERF-03-fixture","logical_rows":1000000,"page_rows":100,
            "warmup":5,"samples_ms":self.samples,"p95_ms":p95,"stale_replies_rejected":self.stale,
            "received_bytes":self.bytes,"cached_rows":stats.rows,"cached_blocks":stats.blocks,
            "active_http":self.client.active_requests(),"error":self.error,"release":!cfg!(debug_assertions),
            "boundary":"loopback synthetic server; actual HTTP and shared RemotePages; virtual input; excludes ERP database, OS input, GPU presentation"}).to_string();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(error) = std::fs::write("qa/performance/page-native.json", &report) {
                eprintln!("Cannot save page report: {error}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.set_attribute("data-grid-page-result", &report);
        }
        self.finished = true;
    }
}
impl eframe::App for PageLab {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.phase == 1 && !self.finished {
            input.focused = true;
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("page-lab").with("interaction")));
            input.events.push(egui::Event::Key {
                key: egui::Key::ArrowDown,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
    }
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.finished {
            return;
        }
        let ctx = root.ctx().clone();
        let frame = ctx.cumulative_frame_nr();
        let fresh = self.frame != Some(frame);
        self.frame = Some(frame);
        let mut index = 0;
        while index < self.jobs.len() {
            let Some(result) = self.jobs[index].1.poll() else {
                index += 1;
                continue;
            };
            let (request, task, old) = self.jobs.remove(index);
            let result = result.map_err(|e| e.to_string()).and_then(|bytes| {
                self.bytes += bytes.len();
                Self::decode(&request, &bytes)
            });
            if let Err(error) = &result {
                self.error = Some(error.clone());
            }
            match self.pages.accept(&request, result) {
                PageAcceptance::Stale if old => self.stale += 1,
                PageAcceptance::Stored if !old => self.completed_at = task.completed_at(),
                other => self.error = Some(format!("Unexpected page acceptance: {other:?}")),
            }
        }
        let page = self.pages.page(&self.address);
        let loaded = page.is_some();
        egui::CentralPanel::default().show(root, |ui| {
            if let Some(page) = page {
                DataGrid {
                    id: egui::Id::new("page-lab"),
                    columns: &mut self.columns,
                    rows: page.rows.len(),
                    state: &mut self.grid,
                    row_id: Some(&|row| page.rows[row].id),
                }
                .show(ui, |row, column| {
                    page.rows[row]
                        .cells
                        .get(&column)
                        .cloned()
                        .unwrap_or_default()
                });
            } else {
                ui.spinner();
            }
        });
        if fresh && loaded {
            if self.phase == 0 {
                self.grid.focus = Some(Cell { row: 0, column: 0 });
                self.phase = 1;
            } else if self.phase == 1 {
                if self.grid.focus != Some(Cell { row: 1, column: 0 }) {
                    self.error = Some("Page readiness input failed".into());
                }
                if self.cycle >= 5
                    && let Some(at) = self.completed_at
                {
                    self.samples.push(at.elapsed().as_secs_f64() * 1000.0);
                }
                self.phase = 2;
            }
        }
        if fresh && self.phase == 2 && self.jobs.is_empty() && self.client.active_requests() == 0 {
            self.cycle += 1;
            if self.cycle == 35 {
                self.finish(&ctx);
            } else if let Err(error) = self.begin(&ctx) {
                self.error = Some(error);
            }
        }
        if self.error.is_some() {
            self.finish(&ctx);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(5));
    }
}
