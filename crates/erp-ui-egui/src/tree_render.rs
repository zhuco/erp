use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};

use crate::TreeRowVisual;

const INDENT: f32 = 16.0;
const ICON: f32 = 16.0;
const LEFT_PAD: f32 = 6.0;

pub(crate) fn arrow_rect(cell: Rect, visual: &TreeRowVisual) -> Option<Rect> {
    visual.has_children.then(|| {
        let max_depth = ((cell.width() - ICON - LEFT_PAD).max(0.0) / INDENT) as u8;
        let depth = visual.depth.min(max_depth);
        Rect::from_min_size(
            Pos2::new(
                cell.left() + LEFT_PAD + f32::from(depth) * INDENT,
                cell.center().y - ICON / 2.0,
            ),
            Vec2::splat(ICON),
        )
    })
}

pub(crate) fn text_left(cell: Rect, visual: &TreeRowVisual) -> f32 {
    let max_depth = ((cell.width() - ICON - LEFT_PAD).max(0.0) / INDENT) as u8;
    let depth = visual.depth.min(max_depth);
    cell.left() + LEFT_PAD + f32::from(depth) * INDENT + ICON + 4.0
}

pub(crate) fn paint(
    ui: &Ui,
    clip: Rect,
    cell: Rect,
    visual: &TreeRowVisual,
    color: Color32,
    status_color: Color32,
) {
    let Some(rect) = arrow_rect(cell, visual) else {
        return;
    };
    let painter = ui.painter().with_clip_rect(clip.intersect(cell));
    if visual.loading {
        painter.circle_stroke(rect.center(), 5.0, Stroke::new(1.5, color));
        painter.line_segment(
            [rect.center(), rect.center() + Vec2::new(4.0, 0.0)],
            Stroke::new(1.5, color),
        );
    } else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            if visual.expanded { "▾" } else { "▸" },
            FontId::proportional(13.0),
            color,
        );
    }
    if visual.error.is_some() {
        painter.circle_filled(rect.right_top() - Vec2::new(2.0, -2.0), 2.5, status_color);
    }
}
