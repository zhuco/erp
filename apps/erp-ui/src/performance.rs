//! Opt-in actual-renderer measurement; never connects to business services.
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId};
use erp_ui_egui::{DataGrid, GridState};
use std::cell::Cell;
use web_time::Instant;

#[derive(serde::Serialize)]
struct Sample {
    focused: bool,
    viewport_focused: Option<bool>,
    viewport_minimized: Option<bool>,
    viewport_occluded: Option<bool>,
    round: usize,
    elapsed_ms: f64,
    frame_interval_ms: f64,
    draw_cpu_ms: f64,
    /// eframe's complete CPU time for the previous frame, excluding vsync wait.
    previous_frame_cpu_ms: Option<f64>,
    cell_text_calls: usize,
}

pub struct PerformanceLab {
    columns: Vec<Column>,
    grid: GridState,
    warmup: usize,
    started: Option<Instant>,
    previous: Instant,
    last_frame: Option<u64>,
    samples: Vec<Sample>,
    result: Option<String>,
    adapter: Option<serde_json::Value>,
    native_no_vsync: bool,
}

impl PerformanceLab {
    pub fn new(cc: &eframe::CreationContext<'_>, native_no_vsync: bool) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        let adapter = cc.wgpu_render_state.as_ref().map(|state| {
            let info = state.adapter.get_info();
            serde_json::json!({"name":info.name,"backend":format!("{:?}",info.backend),"device_type":format!("{:?}",info.device_type),"driver":info.driver,"driver_info":info.driver_info})
        });
        let columns = (0..100)
            .map(|index| {
                let mut column = Column::new(ColumnId(index), format!("Column {index}"), 120.0);
                column.pinned = index < 2;
                column.pinned_right = index >= 98;
                column
            })
            .collect();
        Self {
            columns,
            grid: GridState::default(),
            warmup: 0,
            started: None,
            previous: Instant::now(),
            last_frame: None,
            samples: Vec::with_capacity(4096),
            result: None,
            adapter,
            native_no_vsync,
        }
    }

    fn finish(&mut self, ctx: &egui::Context) {
        let mut intervals: Vec<_> = self.samples.iter().map(|s| s.frame_interval_ms).collect();
        intervals.sort_by(f64::total_cmp);
        let rounds_complete = (1..=3).all(|round| {
            self.samples
                .iter()
                .filter(|sample| sample.round == round)
                .count()
                >= 30
        });
        let p95 = intervals
            .get((intervals.len() * 95).div_ceil(100).saturating_sub(1))
            .copied();
        let report = serde_json::json!({
            "scenario": "PERF-02", "build": if cfg!(debug_assertions) { "debug" } else { "release" },
            "platform": if cfg!(target_arch = "wasm32") { "web" } else { std::env::consts::OS },
            "rows": 10000, "columns": 100, "left_pinned": 2, "right_pinned": 2,
            "adapter": self.adapter,
            "presentation_mode": if self.native_no_vsync { "auto_no_vsync" } else { "default" },
            "rounds": 3, "seconds_per_round": 10, "warmup_frames": 5,
            "pixels_per_point": ctx.pixels_per_point(),
            "viewport_points": [ctx.input(|i| i.content_rect().width()), ctx.input(|i| i.content_rect().height())],
            "frame_interval_p95_ms": p95,
            "measured_seconds": self.started.map(|start| start.elapsed().as_secs_f64()),
            "rounds_complete": rounds_complete,
            "interval_budget_met": !cfg!(debug_assertions) && rounds_complete && self.started.is_some_and(|start| start.elapsed().as_secs_f64() >= 30.0) && p95.is_some_and(|value| value <= 32.0),
            "max_cell_text_calls": self.samples.iter().map(|s| s.cell_text_calls).max(),
            "samples": self.samples,
            "boundaries": ["frame interval counts unique egui frame numbers; additional layout passes are not frames", "draw CPU covers the first UI pass and excludes GPU presentation", "memory and hardware metadata are collected by the launcher", "visual gaps require screenshot review", "not PERF-01/03/04/05 acceptance"]
        });
        let result = serde_json::to_string(&report)
            .unwrap_or_else(|error| format!("{{\"error\":\"{error}\"}}"));
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = std::path::Path::new("qa/performance/native-latest.json");
            let saved = std::fs::create_dir_all("qa/performance")
                .and_then(|()| std::fs::write(path, &result));
            match saved {
                Ok(()) => println!("Grid performance report: {}", path.display()),
                Err(error) => eprintln!("Cannot save performance report: {error}"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.set_attribute("data-grid-perf-result", &result);
        }
        self.result = Some(result);
    }
}

impl eframe::App for PerformanceLab {
    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let now = Instant::now();
        let frame_number = ctx.cumulative_frame_nr();
        let new_frame = self.last_frame != Some(frame_number);
        if self.result.is_some() {
            return;
        }
        let elapsed = self
            .started
            .map_or(0.0, |start| now.duration_since(start).as_secs_f64());
        if elapsed >= 30.0 || self.samples.len() >= 20000 {
            self.finish(&ctx);
            return;
        }
        let phase = elapsed % 10.0 / 10.0;
        let wave = if phase < 0.5 {
            phase * 2.0
        } else {
            (1.0 - phase) * 2.0
        };
        self.grid.set_scroll_offset(egui::vec2(
            (wave * 10600.0) as f32,
            (wave * 279000.0) as f32,
        ));
        let calls = Cell::new(0usize);
        let draw = Instant::now();
        egui::CentralPanel::default().show(root, |ui| {
            DataGrid {
                id: egui::Id::new("perf-02"),
                columns: &mut self.columns,
                rows: 10000,
                state: &mut self.grid,
                row_id: Some(&|row| RowId(row as u128 + 1)),
            }
            .show(ui, |row, column| {
                calls.set(calls.get() + 1);
                format!("{row}:{}", column.0)
            });
        });
        if new_frame && self.warmup < 5 {
            self.warmup += 1;
            if self.warmup == 5 {
                self.started = Some(Instant::now());
            }
        } else if new_frame {
            let viewport = ctx.input(|input| input.viewport().clone());
            self.samples.push(Sample {
                focused: ctx.input(|input| input.focused),
                viewport_focused: viewport.focused,
                viewport_minimized: viewport.minimized,
                viewport_occluded: viewport.occluded,
                round: (elapsed / 10.0) as usize + 1,
                elapsed_ms: elapsed * 1000.0,
                frame_interval_ms: now.duration_since(self.previous).as_secs_f64() * 1000.0,
                draw_cpu_ms: draw.elapsed().as_secs_f64() * 1000.0,
                previous_frame_cpu_ms: frame
                    .info()
                    .cpu_usage
                    .map(|seconds| f64::from(seconds) * 1000.0),
                cell_text_calls: calls.get(),
            });
        }
        if new_frame {
            self.previous = now;
            self.last_frame = Some(frame_number);
        }
        ctx.request_repaint();
    }
}
