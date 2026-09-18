use eframe::egui;

const CJK_FONT_NAME: &str = "source-han-sans-sc";
const CJK_FONT: &[u8] = include_bytes!("../assets/fonts/SourceHanSansSC-Regular.otf");

/// Installs the bundled CJK fallback once while the application is created.
/// The borrowed static bytes are embedded in native and WASM binaries, so
/// painting never reads a host font file or waits for a network request.
pub(crate) fn install(context: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        CJK_FONT_NAME.into(),
        egui::FontData::from_static(CJK_FONT).into(),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push(CJK_FONT_NAME.into());
    }
    context.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_otf_has_chinese_glyphs_in_egui_backend() {
        let context = egui::Context::default();
        install(&context);
        let mut output = context.run_ui(Default::default(), |ui| {
            ui.ctx().fonts_mut(|fonts| {
                assert!(fonts.has_glyphs(&egui::FontId::proportional(16.0), "仓储中文"));
            });
        });
        output.textures_delta.clear();
    }
}
