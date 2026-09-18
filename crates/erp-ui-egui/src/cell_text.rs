use egui::{Color32, FontId, Painter, Pos2, Rect};

/// Keep overflow clipping exact without splitting GPU batches for fitting text.
pub(crate) fn paint(painter: &Painter, rect: Rect, left: f32, text: String, color: Color32) {
    let galley = painter.layout_no_wrap(text, FontId::proportional(13.0), color);
    let pos = Pos2::new(left, rect.center().y - galley.size().y * 0.5);
    let inner = rect.shrink2(egui::vec2(5.0, 0.0));
    let ink = galley.mesh_bounds.translate(pos.to_vec2());
    if inner.contains_rect(ink) {
        painter.galley(pos, galley, color);
    } else {
        painter
            .with_clip_rect(inner.intersect(painter.clip_rect()))
            .galley(pos, galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitting_text_shares_clip_but_overflow_and_multiline_stay_in_the_cell() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(800.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| {
                let rect = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(100.0, 28.0));
                paint(ui.painter(), rect, 28.0, "fit".into(), Color32::WHITE);
                paint(
                    ui.painter(),
                    rect.translate(egui::vec2(0.0, 40.0)),
                    28.0,
                    "overflow".repeat(20),
                    Color32::WHITE,
                );
                paint(
                    ui.painter(),
                    rect.translate(egui::vec2(0.0, 80.0)),
                    28.0,
                    "one\ntwo\nthree".into(),
                    Color32::WHITE,
                );
            },
        );
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape {
                    Some((text.galley.text(), shape.clip_rect))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts.len(), 3);
        assert!(texts[0].1.width() > 100.0);
        assert_eq!(texts[1].1.width(), 90.0);
        assert_eq!(texts[2].1.height(), 28.0);
        output.textures_delta.clear();
    }
}
