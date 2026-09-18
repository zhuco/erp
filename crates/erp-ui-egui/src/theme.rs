use egui::{Color32, Visuals};

/// Semantic colors only: product branding belongs to the host application's theme.
#[derive(Clone, Copy, Debug)]
pub struct GridTheme {
    pub row: Color32,
    pub alternate_row: Color32,
    pub active_row: Color32,
    pub checked_row: Color32,
    pub selected_cell: Color32,
    pub text: Color32,
    pub selected_text: Color32,
    pub muted_text: Color32,
    pub header: Color32,
    pub header_text: Color32,
    pub structure: Color32,
    pub detail: Color32,
    pub grid_line: Color32,
    pub header_line: Color32,
    pub structure_line: Color32,
    pub accent: Color32,
}
impl GridTheme {
    pub fn from_visuals(visuals: &Visuals) -> Self {
        if visuals.dark_mode {
            Self {
                row: Color32::from_rgb(27, 32, 39),
                alternate_row: Color32::from_rgb(23, 28, 34),
                active_row: Color32::from_rgb(42, 51, 65),
                checked_row: Color32::from_rgb(32, 60, 70),
                selected_cell: Color32::from_rgb(33, 77, 115),
                text: Color32::from_gray(225),
                selected_text: Color32::from_gray(225),
                muted_text: Color32::from_gray(180),
                header: Color32::from_rgb(39, 46, 56),
                header_text: Color32::WHITE,
                structure: Color32::from_rgb(35, 42, 51),
                detail: Color32::from_rgb(18, 24, 30),
                grid_line: Color32::from_gray(48),
                header_line: Color32::from_gray(80),
                structure_line: Color32::from_gray(60),
                accent: Color32::LIGHT_BLUE,
            }
        } else {
            Self {
                row: visuals.panel_fill,
                alternate_row: visuals.faint_bg_color,
                active_row: visuals.widgets.hovered.bg_fill,
                checked_row: visuals.widgets.active.bg_fill,
                selected_cell: visuals.selection.bg_fill,
                text: visuals.text_color(),
                selected_text: visuals.selection.stroke.color,
                muted_text: visuals.weak_text_color(),
                header: visuals.widgets.inactive.bg_fill,
                header_text: visuals.text_color(),
                structure: visuals.widgets.noninteractive.bg_fill,
                detail: visuals.extreme_bg_color,
                grid_line: visuals.widgets.noninteractive.bg_stroke.color,
                header_line: visuals.widgets.inactive.bg_stroke.color,
                structure_line: visuals.widgets.noninteractive.bg_stroke.color,
                accent: visuals.selection.stroke.color,
            }
        }
    }
}
