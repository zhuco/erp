//! Visual values from retired ERP src/styles/theme.css and workbenchPanelSizing.ts.
use eframe::egui::{self, Color32, FontId, TextStyle};
pub(crate) const PRIMARY: Color32 = Color32::from_rgb(31, 58, 95);
pub(crate) const LEFT_WIDTH: f32 = 240.0;
pub(crate) const RIGHT_WIDTH: f32 = 360.0;
pub(crate) const DETAILS_HEIGHT: f32 = 280.0;
pub(crate) fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = Color32::from_rgb(240, 243, 245);
    visuals.window_fill = Color32::from_rgb(245, 247, 250);
    visuals.faint_bg_color = Color32::from_rgb(238, 242, 246);
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.override_text_color = None;
    visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(43, 47, 51);
    visuals.text_edit_bg_color = Some(Color32::from_rgb(245, 247, 250));
    visuals.hyperlink_color = Color32::from_rgb(46, 111, 149);
    visuals.selection.bg_fill = Color32::from_rgb(31, 58, 95);
    visuals.selection.stroke.color = Color32::WHITE;
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(238, 242, 246);
    visuals.widgets.inactive.bg_stroke.color = Color32::from_rgb(138, 155, 168);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(245, 247, 250);
    visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(222, 230, 237);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(237, 239, 242);
    visuals.widgets.active.bg_fill = Color32::from_rgb(219, 223, 229);
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Light, |style| {
        for text in [TextStyle::Body, TextStyle::Button, TextStyle::Monospace] {
            style.text_styles.insert(text, FontId::proportional(12.8));
        }
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.0));
        style.spacing.interact_size.y = 24.0;
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.button_padding = egui::vec2(6.0, 2.0);
    });
}

/// Compact light workbench palette selected from the order-query reference.
pub(crate) fn grid() -> erp_ui_egui::GridTheme {
    let mut theme = erp_ui_egui::GridTheme::from_visuals(&egui::Visuals::light());
    theme.row = Color32::WHITE;
    theme.alternate_row = Color32::WHITE;
    theme.header = Color32::from_rgb(245, 247, 248);
    theme.structure = theme.header;
    theme.grid_line = Color32::from_rgb(222, 232, 241);
    theme.header_line = theme.grid_line;
    theme.structure_line = theme.grid_line;
    theme.active_row = Color32::from_rgb(222, 234, 254);
    theme.checked_row = theme.active_row;
    theme.selected_cell = Color32::from_rgb(84, 132, 229);
    theme.selected_text = Color32::WHITE;
    theme.text = Color32::from_rgb(32, 39, 46);
    theme.header_text = theme.text;
    theme.accent = Color32::from_rgb(64, 113, 209);
    theme
}
