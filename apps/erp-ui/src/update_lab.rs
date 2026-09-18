//! Local ordered producer driving the public remote cache and actual renderer.
//! This measures scheduled-event to UI draw, not network or GPU presentation.
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId, remote::*};
use erp_ui_egui::{DataGrid, GridState};
use std::collections::BTreeMap;
use web_time::Instant;

pub struct UpdateLab {
    pages: RemotePages,
    queue: UpdateQueue,
    address: PageAddress,
    columns: Vec<Column>,
    grid: GridState,
    resident: Vec<RemoteRow>,
    start: Option<Instant>,
    warmup: usize,
    frame: Option<u64>,
    sent: usize,
    samples: Vec<f64>,
    error: Option<String>,
    result: Option<String>,
}

impl UpdateLab {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        crate::fonts::install(&cc.egui_ctx);
        let mut pages = RemotePages::default();
        // Local dataset residency is separate from the bounded remote page cache.
        // Never enlarge the production cache just to accommodate a benchmark.
        let resident: Vec<_> = (1..=10_000)
            .map(|id| RemoteRow {
                id: RowId(id),
                version: Some("0".into()),
                cells: BTreeMap::from([
                    (ColumnId(1), format!("Row {id}")),
                    (ColumnId(2), "0".into()),
                ]),
            })
            .collect();
        pages.bind(RemoteQuery {
            scope: DataScope {
                source: "local-update-fixture".into(),
                schema: 1,
                authorization: "fixture".into(),
            },
            expression: None,
            filters: BTreeMap::new(),
            sorts: Vec::new(),
            search: None,
        });
        let address = PageAddress::Offset {
            start: 0,
            limit: 100,
        };
        let PageStart::Fetch(request) = pages.start(address.clone())? else {
            return Err("Fixture request missing".into());
        };
        let page = RemotePage {
            rows: resident[..100].to_vec(),
            total: RowCount::Exact(10_000),
            snapshot: None,
            next_cursor: None,
        };
        if pages.accept(&request, Ok(page)) != PageAcceptance::Stored {
            return Err("Fixture page rejected".into());
        }
        Ok(Self {
            queue: UpdateQueue::new(pages.generation()),
            pages,
            address,
            columns: vec![
                Column::new(ColumnId(1), "Record", 200.0),
                Column::new(ColumnId(2), "Revision", 180.0),
            ],
            grid: GridState::default(),
            resident,
            start: None,
            warmup: 0,
            frame: None,
            sent: 0,
            samples: Vec::with_capacity(6000),
            error: None,
            result: None,
        })
    }

    fn finish(&mut self, ctx: &egui::Context) {
        if self.error.is_none()
            && self.resident.iter().enumerate().any(|(index, row)| {
                let expected = if index < 10 { "600" } else { "0" };
                row.id != RowId((index + 1) as u128)
                    || row.version.as_deref() != Some(expected)
                    || row.cells.get(&ColumnId(2)).map(String::as_str) != Some(expected)
            })
        {
            self.error = Some("Resident row identity/version verification failed".into());
        }
        let mut sorted = self.samples.clone();
        sorted.sort_by(f64::total_cmp);
        let p95 = sorted.get((sorted.len() * 95).div_ceil(100).saturating_sub(1));
        let versions: Vec<_> = self
            .pages
            .page(&self.address)
            .into_iter()
            .flat_map(|p| p.rows.iter().take(10))
            .map(|r| r.version.clone())
            .collect();
        let report = serde_json::json!({
            "scenario": "resident-grid-update", "logical_rows":10000, "resident_rows":self.resident.len(), "cached_rows":self.pages.cached_rows(),
            "updated_visible_rows":10, "scheduled_hz":100, "duration_seconds":60,
            "scheduled_events":self.sent, "draw_samples_ms":self.samples, "draw_p95_ms":p95,
            "versions":versions, "error":self.error,
            "queue_pending":self.queue.pending_count(), "queue_bytes":self.queue.pending_bytes(),
            "release":!cfg!(debug_assertions), "pixels_per_point":ctx.pixels_per_point(),
            "boundary":"local 10000-row resident dataset; bounded 100-row update cache; UI draw only; excludes network and GPU presentation"
        });
        let result = report.to_string();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(error) = std::fs::write("qa/performance/update-latest.json", &result) {
                eprintln!("Cannot save update report: {error}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.set_attribute("data-grid-update-result", &result);
        }
        self.result = Some(result);
    }
}

impl eframe::App for UpdateLab {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        if self.result.is_some() {
            return;
        }
        let frame = ctx.cumulative_frame_nr();
        let fresh = self.frame != Some(frame);
        self.frame = Some(frame);
        let elapsed = self
            .start
            .map_or(0.0, |start| start.elapsed().as_secs_f64());
        let mut deadlines = Vec::new();
        if fresh && self.start.is_some() && self.error.is_none() {
            let due = ((elapsed * 100.0).floor() as usize).min(6000);
            if due.saturating_sub(self.sent) > 1000 {
                self.error = Some("Producer backlog exceeds queue budget".into());
            } else {
                while self.sent < due {
                    let event = self.sent;
                    let patch = RowPatch {
                        id: RowId((event % 10 + 1) as u128),
                        expected_version: (event / 10).to_string(),
                        version: (event / 10 + 1).to_string(),
                        cells: BTreeMap::from([(ColumnId(2), (event / 10 + 1).to_string())]),
                    };
                    if let Err(error) = self.queue.push(self.pages.generation(), patch) {
                        self.error = Some(error);
                        break;
                    }
                    self.sent += 1;
                    deadlines.push(self.sent as f64 * 10.0);
                }
                match self.queue.flush(&mut self.pages, true) {
                    Ok(receipt) => {
                        for row in receipt.changed_rows {
                            if let Some(updated) = self.pages.page(&self.address).and_then(|page| {
                                page.rows.iter().find(|candidate| candidate.id == row)
                            }) && let Some(resident) = usize::try_from(row.0)
                                .ok()
                                .and_then(|id| id.checked_sub(1))
                                .and_then(|index| self.resident.get_mut(index))
                            {
                                resident.clone_from(updated);
                            } else {
                                self.error =
                                    Some("Updated row missing from resident dataset".into());
                            }
                            self.grid.notify_row_changed(row);
                        }
                    }
                    Err(error) => {
                        self.error = Some(error);
                        deadlines.clear();
                    }
                }
            }
        }
        let drawn = std::cell::Cell::new(0u16);
        egui::CentralPanel::default().show(root, |ui| {
            if let Some(error) = &self.error {
                ui.label(error);
            }
            {
                DataGrid {
                    id: egui::Id::new("updates"),
                    columns: &mut self.columns,
                    rows: self.resident.len(),
                    state: &mut self.grid,
                    row_id: Some(&|index| self.resident[index].id),
                }
                .show(ui, |index, column| {
                    if index < 10 && column == ColumnId(2) {
                        drawn.set(drawn.get() | (1 << index));
                    }
                    self.resident[index]
                        .cells
                        .get(&column)
                        .cloned()
                        .unwrap_or_default()
                });
            }
        });
        if fresh {
            if self.start.is_none() {
                self.warmup += 1;
                if self.warmup >= 5 {
                    self.start = Some(Instant::now());
                }
            } else if let Some(start) = self.start {
                let now_ms = start.elapsed().as_secs_f64() * 1000.0;
                if !deadlines.is_empty() && drawn.get() != 1023 {
                    self.error = Some("Updated rows were not all drawn".into());
                }
                self.samples
                    .extend(deadlines.into_iter().map(|due| now_ms - due));
            }
            if self.error.is_some() || self.sent == 6000 {
                self.finish(&ctx);
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(10));
    }
}
