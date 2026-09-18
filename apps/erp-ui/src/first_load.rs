//! PERF-01: data-ready to a rendered grid accepting a synthetic keyboard event.
//! Excludes fixture generation/network and OS keyboard/GPU presentation latency.
use eframe::egui;
use erp_grid_core::{Cell, Column, ColumnId, RowId};
use erp_ui_egui::{DataGrid, GridState};
use web_time::Instant;

pub struct FirstLoadLab {
    data: Vec<String>,
    columns: Vec<Column>,
    grid: GridState,
    ready: Instant,
    phase: u8,
    frame: Option<u64>,
    completed: usize,
    samples: Vec<f64>,
    error: Option<String>,
    finished: bool,
}
impl FirstLoadLab {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        let data = (0..300_000)
            .map(|i| format!("Row {} / Column {}", i / 30 + 1, i % 30 + 1))
            .collect();
        let columns = (0..30)
            .map(|i| Column::new(ColumnId(i), format!("Column {}", i + 1), 120.0))
            .collect();
        Self {
            data,
            columns,
            grid: GridState::default(),
            ready: Instant::now(),
            phase: 0,
            frame: None,
            completed: 0,
            samples: Vec::with_capacity(30),
            error: None,
            finished: false,
        }
    }
    fn finish(&mut self, ctx: &egui::Context) {
        let mut sorted = self.samples.clone();
        sorted.sort_by(f64::total_cmp);
        let p95 = sorted
            .get((sorted.len() * 95).div_ceil(100).saturating_sub(1))
            .copied();
        let report = serde_json::json!({
            "scenario":"PERF-01", "rows":10000, "columns":30, "warmup_samples":5,
            "samples_ms":self.samples, "p95_ms":p95, "error":self.error,
            "release":!cfg!(debug_assertions), "pixels_per_point":ctx.pixels_per_point(),
            "budget_met":!cfg!(debug_assertions) && self.samples.len()==30 && self.error.is_none() && p95.is_some_and(|p|p<=500.0),
            "boundary":"materialized local data; new GridState per sample; synthetic egui ArrowDown accepted after initial draw; excludes network, OS input and GPU presentation"
        }).to_string();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(error) = std::fs::write("qa/performance/first-load-native.json", &report) {
                eprintln!("Cannot save first-load report: {error}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.set_attribute("data-grid-first-load-result", &report);
        }
        self.finished = true;
    }
}
impl eframe::App for FirstLoadLab {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.phase == 1 && !self.finished {
            // Controlled virtual-input target, independent of OS foreground state.
            input.focused = true;
            ctx.memory_mut(|memory| {
                memory.request_focus(egui::Id::new("first-load-grid").with("interaction"))
            });
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
        let id = egui::Id::new("first-load-grid");
        let drawn = std::cell::Cell::new(0usize);
        egui::CentralPanel::default().show(root, |ui| {
            DataGrid {
                id,
                columns: &mut self.columns,
                rows: 10000,
                state: &mut self.grid,
                row_id: Some(&|row| RowId(row as u128 + 1)),
            }
            .show(ui, |row, column| {
                drawn.set(drawn.get() + 1);
                self.data[row * 30 + column.0 as usize].clone()
            });
        });
        if fresh {
            if drawn.get() == 0 {
                self.error = Some("No data cells rendered".into());
            }
            if self.phase == 0 {
                self.grid.focus = Some(Cell { row: 0, column: 0 });
                self.phase = 1;
            } else {
                if self.grid.focus != Some(Cell { row: 1, column: 0 }) {
                    self.error = Some("Grid did not handle the readiness input".into());
                }
                self.completed += 1;
                if self.completed > 5 {
                    self.samples
                        .push(self.ready.elapsed().as_secs_f64() * 1000.0);
                }
                self.grid = GridState::default();
                self.ready = Instant::now();
                self.phase = 0;
            }
            if self.error.is_some() || self.completed == 35 {
                self.finish(&ctx);
            }
        }
        ctx.request_repaint();
    }
}
