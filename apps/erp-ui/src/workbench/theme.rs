//! Shared visual tokens for the workbench showcase; no business state.
use eframe::egui::{self, Color32, FontId, Stroke, TextStyle};

pub fn install(ctx: &egui::Context, dark: bool) {
    for mode in [egui::Theme::Dark, egui::Theme::Light] {
        let dark = mode == egui::Theme::Dark;
        let mut visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let (panel, surface, input, border, text, muted, hover, selected) = if dark {
            (
                rgb(23, 27, 34),
                rgb(30, 35, 43),
                rgb(20, 24, 30),
                rgb(58, 67, 81),
                rgb(224, 230, 239),
                rgb(152, 164, 183),
                rgb(43, 52, 65),
                rgb(43, 70, 104),
            )
        } else {
            (
                rgb(245, 247, 250),
                Color32::WHITE,
                Color32::WHITE,
                rgb(204, 214, 226),
                rgb(34, 48, 66),
                rgb(103, 119, 139),
                rgb(232, 238, 246),
                rgb(219, 233, 251),
            )
        };
        visuals.panel_fill = panel;
        visuals.window_fill = surface;
        visuals.extreme_bg_color = input;
        visuals.faint_bg_color = panel;
        visuals.text_edit_bg_color = Some(input);
        visuals.override_text_color = Some(text);
        visuals.weak_text_color = Some(muted);
        visuals.hyperlink_color = if dark {
            rgb(111, 174, 250)
        } else {
            rgb(45, 111, 201)
        };
        visuals.selection.bg_fill = selected;
        visuals.selection.stroke = Stroke::new(1.0, text);
        visuals.window_stroke = Stroke::new(1.0, border);
        visuals.window_corner_radius = 6.into();
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.fg_stroke = Stroke::new(1.0, text);
            widget.bg_stroke = Stroke::new(1.0, border);
            widget.bg_fill = surface;
            widget.weak_bg_fill = surface;
            widget.corner_radius = 4.into();
            widget.expansion = 0.0;
        }
        visuals.widgets.hovered.bg_fill = hover;
        visuals.widgets.hovered.weak_bg_fill = hover;
        visuals.widgets.active.bg_fill = selected;
        visuals.widgets.active.weak_bg_fill = selected;
        visuals.widgets.open.bg_fill = selected;
        ctx.style_mut_of(mode, |style| {
            style.visuals = visuals;
            for text in [TextStyle::Body, TextStyle::Button, TextStyle::Monospace] {
                style.text_styles.insert(text, FontId::proportional(13.0));
            }
            style
                .text_styles
                .insert(TextStyle::Small, FontId::proportional(12.0));
            style
                .text_styles
                .insert(TextStyle::Heading, FontId::proportional(16.0));
            style.spacing.interact_size.y = 26.0;
            style.spacing.item_spacing = egui::vec2(8.0, 5.0);
            style.spacing.button_padding = egui::vec2(8.0, 3.0);
            style.spacing.scroll.bar_width = 7.0;
        });
    }
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
}

pub fn grid(visuals: &egui::Visuals) -> erp_ui_egui::GridTheme {
    let mut grid = erp_ui_egui::GridTheme::from_visuals(visuals);
    if visuals.dark_mode {
        grid.row = rgb(25, 30, 38);
        grid.alternate_row = rgb(29, 35, 44);
        grid.header = rgb(37, 44, 55);
        grid.structure = rgb(30, 36, 45);
        grid.grid_line = rgb(43, 51, 63);
        grid.header_line = rgb(65, 77, 94);
        grid.structure_line = rgb(49, 59, 73);
        grid.active_row = rgb(36, 57, 80);
        grid.checked_row = rgb(32, 63, 76);
        grid.selected_cell = rgb(43, 90, 145);
        grid.selected_text = Color32::WHITE;
        grid.text = rgb(222, 230, 241);
        grid.header_text = rgb(198, 211, 228);
        grid.muted_text = rgb(142, 159, 181);
        grid.accent = rgb(110, 172, 245);
    } else {
        grid.row = Color32::WHITE;
        grid.alternate_row = rgb(248, 250, 253);
        grid.header = rgb(236, 241, 247);
        grid.structure = rgb(242, 245, 250);
        grid.grid_line = rgb(231, 236, 243);
        grid.header_line = rgb(205, 215, 229);
        grid.structure_line = rgb(218, 226, 237);
        grid.active_row = rgb(226, 238, 254);
        grid.checked_row = rgb(220, 237, 246);
        grid.selected_cell = rgb(54, 114, 192);
        grid.selected_text = Color32::WHITE;
        grid.text = rgb(37, 52, 72);
        grid.header_text = rgb(71, 90, 115);
        grid.muted_text = rgb(111, 127, 147);
        grid.accent = rgb(45, 111, 201);
    }
    grid
}
const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
