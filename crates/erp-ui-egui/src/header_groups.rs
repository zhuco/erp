use egui::{Align2, FontId, Painter, Rect, Stroke};
use erp_grid_core::{Column, HeaderGroup, header_group_runs};

use crate::GridTheme;

pub const COMPACT_HEADER_GROUP_HEIGHT: f32 = 28.0;

/// Paint the single group-header row using final visible column geometry.
pub fn paint_header_groups(
    painter: &Painter,
    groups: &[HeaderGroup],
    visible_columns: &[Column],
    row_rect: Rect,
    column_rect: impl Fn(erp_grid_core::ColumnId) -> Option<Rect>,
    band_clip: impl Fn(u8) -> Rect,
    theme: &GridTheme,
) {
    for run in header_group_runs(groups, visible_columns) {
        let (Some(first), Some(last)) =
            (column_rect(run.first_column), column_rect(run.last_column))
        else {
            continue;
        };
        let rect = Rect::from_min_max(
            egui::pos2(first.left(), row_rect.top()),
            egui::pos2(last.right(), row_rect.bottom()),
        )
        .intersect(row_rect)
        .intersect(band_clip(run.pin_band));
        if !rect.is_positive() {
            continue;
        }
        let band_painter = painter.with_clip_rect(rect);
        band_painter.rect_filled(rect, 0.0, theme.header);
        band_painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            run.title,
            FontId::proportional(12.0),
            theme.header_text,
        );
        band_painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, theme.header_line),
        );
        band_painter.line_segment(
            [rect.right_top(), rect.right_bottom()],
            Stroke::new(1.0, theme.header_line),
        );
    }
}
