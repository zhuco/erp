#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let erp_app = std::env::args().any(|argument| argument == "--erp");
    let modules = std::env::args().any(|argument| argument == "--ui-modules");
    let performance = std::env::args().any(|argument| argument == "--grid-perf");
    let updates = std::env::args().any(|argument| argument == "--grid-updates");
    let first_load = std::env::args().any(|argument| argument == "--grid-first-load");
    let pages = std::env::args().any(|argument| argument == "--grid-pages");
    // The production window uses the compositor-safe default.  AutoNoVsync
    // can select Immediate/Mailbox. Prefer the framework's default while
    // diagnosing presentation through Windows virtual displays. Keep the low-latency mode
    // available for the dedicated benchmark and its explicit comparison.
    let no_vsync = !erp_app
        && !modules
        && !(performance
            && std::env::var("ERP_GRID_PERF_PRESENT_MODE")
                .is_ok_and(|value| value == "auto-vsync"));
    let mut native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        renderer: eframe::Renderer::Wgpu,
        persist_window: !(performance || updates || first_load || pages),
        ..Default::default()
    };
    // The remote Windows acceptance run showed the ERP surface rendering with
    // DX12. Prefer it for the formal native app, while retaining wgpu's
    // `WGPU_BACKEND` override for driver/display comparisons and fallback
    // diagnosis. The benchmark/lab keeps the normal wgpu backend selection.
    if cfg!(target_os = "windows")
        && (erp_app || modules)
        && std::env::var_os("WGPU_BACKEND").is_none()
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) =
            &mut native_options.wgpu_options.wgpu_setup
    {
        setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
    }
    if no_vsync {
        native_options.wgpu_options.surface.present_mode = eframe::wgpu::PresentMode::AutoNoVsync;
    }
    eprintln!(
        "erp-ui native renderer=wgpu present_mode={} wgpu_backend={}",
        if no_vsync {
            "auto-no-vsync"
        } else {
            "auto-vsync"
        },
        std::env::var("WGPU_BACKEND").unwrap_or_else(|_| {
            if cfg!(target_os = "windows") && erp_app {
                "dx12".to_owned()
            } else {
                "default".to_owned()
            }
        }),
    );
    eframe::run_native(
        if modules {
            "易格 UI 模块示例"
        } else if erp_app {
            "易格 ERP"
        } else if performance {
            "ERP Grid PERF-02"
        } else {
            "ERP Grid Lab"
        },
        native_options,
        Box::new(move |cc| {
            if modules {
                Ok(Box::new(erp_ui::module_demo::ModuleDemo::new(cc)))
            } else if erp_app {
                Ok(Box::new(erp_ui::erp_app::ErpApp::new(cc)))
            } else if pages {
                Ok(Box::new(
                    erp_ui::page_lab::PageLab::new(cc).map_err(std::io::Error::other)?,
                ))
            } else if first_load {
                Ok(Box::new(erp_ui::first_load::FirstLoadLab::new(cc)))
            } else if updates {
                Ok(Box::new(
                    erp_ui::update_lab::UpdateLab::new(cc).map_err(std::io::Error::other)?,
                ))
            } else if performance {
                Ok(Box::new(erp_ui::performance::PerformanceLab::new(
                    cc, no_vsync,
                )))
            } else {
                Ok(Box::new(erp_ui::GridLab::new(cc)))
            }
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
