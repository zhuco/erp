use egui::{Id, Ui};
use erp_grid_core::Pagination;

/// Returns true when the host must rebind its page and clear positional selection.
pub fn show_pagination(
    ui: &mut Ui,
    id: Id,
    state: &mut Pagination,
    total: usize,
    enabled: bool,
) -> bool {
    let before = *state;
    state.set_page(state.page(), total);
    ui.push_id(id, |ui| {
        let controls = ui.horizontal(|ui| {
            ui.add_enabled_ui(enabled, |ui| {
                if ui
                    .add_enabled(state.page() > 0, egui::Button::new("|<"))
                    .on_hover_text("First page")
                    .clicked()
                {
                    state.set_page(0, total);
                }
                if ui
                    .add_enabled(state.page() > 0, egui::Button::new("<"))
                    .on_hover_text("Previous page")
                    .clicked()
                {
                    state.set_page(state.page().saturating_sub(1), total);
                }
                let pages = state.page_count(total);
                let mut page = if total == 0 { 0 } else { state.page() + 1 };
                if total == 0 {
                    ui.label("0");
                } else {
                    if ui
                        .add_enabled(
                            total > 0,
                            egui::DragValue::new(&mut page)
                                .range(1..=pages.max(1))
                                .speed(0.0),
                        )
                        .on_hover_text("Enter page number")
                        .changed()
                    {
                        state.set_page(page.saturating_sub(1), total);
                    }
                }
                ui.label(format!("/ {pages}"));
                if ui
                    .add_enabled(state.page() + 1 < pages, egui::Button::new(">"))
                    .on_hover_text("Next page")
                    .clicked()
                {
                    state.set_page(state.page() + 1, total);
                }
                if ui
                    .add_enabled(state.page() + 1 < pages, egui::Button::new(">|"))
                    .on_hover_text("Last page")
                    .clicked()
                {
                    state.set_page(pages.saturating_sub(1), total);
                }
                egui::ComboBox::from_id_salt("page-size")
                    .selected_text(format!("{} / page", state.page_size()))
                    .show_ui(ui, |ui| {
                        for size in [10, 20, 50, 100, 200, 500] {
                            if ui
                                .selectable_label(state.page_size() == size, size.to_string())
                                .clicked()
                            {
                                state.set_page_size(size);
                            }
                        }
                    });
            });
            let range = state.range(total);
            ui.label(format!(
                "{}–{} / {total}",
                if total == 0 { 0 } else { range.start + 1 },
                range.end
            ));
            ui.min_rect().width()
        });
        ui.ctx()
            .data_mut(|data| data.insert_temp(id.with("width"), controls.inner));
    });
    *state != before
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pager_buttons_and_page_size_menu_change_the_visible_range() {
        let ctx = egui::Context::default();
        let mut pager = Pagination::default();
        fn frame(
            ctx: &egui::Context,
            pager: &mut Pagination,
            events: Vec<egui::Event>,
        ) -> Vec<(String, egui::Pos2)> {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    show_pagination(ui, Id::new("test-pager"), pager, 250, true);
                },
            );
            fn collect(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Pos2)>) {
                match shape {
                    egui::epaint::Shape::Text(text) => {
                        out.push((text.galley.text().into(), text.pos + egui::vec2(4.0, 4.0)))
                    }
                    egui::epaint::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect(shape, out);
                        }
                    }
                    _ => {}
                }
            }
            output.textures_delta.clear();
            let mut texts = Vec::new();
            for shape in output.shapes {
                collect(&shape.shape, &mut texts);
            }
            texts
        }
        fn click(
            ctx: &egui::Context,
            pager: &mut Pagination,
            pos: egui::Pos2,
        ) -> Vec<(String, egui::Pos2)> {
            frame(ctx, pager, vec![egui::Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                frame(
                    ctx,
                    pager,
                    vec![egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    }],
                );
            }
            frame(ctx, pager, vec![])
        }
        fn at(text: &[(String, egui::Pos2)], label: &str) -> egui::Pos2 {
            text.iter().find(|(text, _)| text == label).unwrap().1
        }
        let text = frame(&ctx, &mut pager, vec![]);
        let text = click(&ctx, &mut pager, at(&text, ">"));
        assert_eq!(pager.range(250), 100..200);
        let text = click(&ctx, &mut pager, at(&text, "100 / page"));
        click(&ctx, &mut pager, at(&text, "10"));
        assert_eq!(pager.page_size(), 10);
        assert_eq!(pager.range(250), 0..10);
    }
}
