use egui::{Id, Rect, Ui};
use erp_grid_core::{Column, DateTimeParts, FilterDraft, FilterKind};

pub fn show(ui: &mut Ui, rect: Rect, id: Id, column: &Column, draft: &mut FilterDraft) -> bool {
    if matches!(column.filter, FilterKind::None) {
        return false;
    }
    let before = draft.clone();
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.push_id(id, |ui| match (&mut *draft, &column.filter) {
            (FilterDraft::Text(value), FilterKind::Text) => {
                ui.put(
                    rect,
                    egui::TextEdit::singleline(value)
                        .id(id)
                        .hint_text("筛选…")
                        .char_limit(256),
                );
            }
            (FilterDraft::Money { min, max }, FilterKind::Money) => {
                let center = rect.center();
                let left = Rect::from_min_max(rect.min, egui::pos2(center.x - 7.0, rect.max.y));
                let right = Rect::from_min_max(egui::pos2(center.x + 7.0, rect.min.y), rect.max);
                ui.put(
                    left,
                    egui::TextEdit::singleline(min)
                        .id(id.with("min"))
                        .hint_text("最小")
                        .char_limit(24),
                )
                .on_hover_text("Minimum amount, inclusive; up to 2 decimals");
                ui.painter().text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    "-",
                    egui::FontId::proportional(14.0),
                    ui.visuals().text_color(),
                );
                ui.put(
                    right,
                    egui::TextEdit::singleline(max)
                        .id(id.with("max"))
                        .hint_text("最大")
                        .char_limit(24),
                )
                .on_hover_text("Maximum amount, inclusive; up to 2 decimals");
            }

            (FilterDraft::DateTime { start, end }, FilterKind::DateTime) => {
                ui.menu_button(
                    if start.is_some() || end.is_some() {
                        "日期时间 *"
                    } else {
                        "日期时间"
                    },
                    |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(420.0)
                            .show(ui, |ui| {
                                ui.push_id("start", |ui| date_picker(ui, "Start", start));
                                ui.separator();
                                ui.push_id("end", |ui| date_picker(ui, "End", end));
                                if ui.button("Clear dates").clicked() {
                                    *start = None;
                                    *end = None;
                                }
                                if let Err(error) = (FilterDraft::DateTime {
                                    start: *start,
                                    end: *end,
                                })
                                .compile(&FilterKind::DateTime)
                                {
                                    ui.colored_label(egui::Color32::YELLOW, error);
                                }
                            });
                    },
                );
            }
            (FilterDraft::Enum(selected), FilterKind::Enum(options)) => {
                ui.menu_button(
                    if selected.is_empty() {
                        "全部".to_owned()
                    } else {
                        format!("已选 {}", selected.len())
                    },
                    |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Select all").clicked() {
                                for option in options {
                                    if selected.len() >= 100 {
                                        break;
                                    }
                                    selected.insert(option.clone());
                                }
                            }
                            if ui.button("Clear").clicked() {
                                selected.clear();
                            }
                        });
                        egui::ScrollArea::vertical()
                            .max_height(230.0)
                            .show(ui, |ui| {
                                for option in options {
                                    let mut checked = selected.contains(option);
                                    if ui
                                        .add_enabled(
                                            checked || selected.len() < 100,
                                            egui::Checkbox::new(
                                                &mut checked,
                                                if option.is_empty() { "(Empty)" } else { option },
                                            ),
                                        )
                                        .changed()
                                    {
                                        if checked {
                                            selected.insert(option.clone());
                                        } else {
                                            selected.remove(option);
                                        }
                                    }
                                }
                            });
                    },
                );
            }
            _ => {
                ui.label("Schema changed");
            }
        });
    });
    *draft != before
}

fn date_picker(ui: &mut Ui, label: &str, value: &mut Option<DateTimeParts>) {
    let mut enabled = value.is_some();
    if ui.checkbox(&mut enabled, label).changed() {
        *value = enabled.then(DateTimeParts::default);
    }
    let Some(date) = value else {
        return;
    };
    calendar(ui, date, true);
}

pub(crate) fn calendar(ui: &mut Ui, date: &mut DateTimeParts, with_time: bool) {
    ui.horizontal(|ui| {
        ui.add(
            egui::DragValue::new(&mut date.year)
                .range(1900..=2100)
                .prefix("Year "),
        );
        ui.add(
            egui::DragValue::new(&mut date.month)
                .range(1..=12)
                .prefix("Month "),
        );
    });
    date.year = date.year.clamp(1900, 2100);
    date.month = date.month.clamp(1, 12);
    let days = DateTimeParts::days_in_month(date.year, date.month);
    date.day = date.day.clamp(1, days.max(1));
    let year = date.year - i32::from(date.month < 3);
    let month_offset = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4][date.month as usize - 1];
    let first_weekday = (year + year / 4 - year / 100 + year / 400 + month_offset + 1) % 7;
    ui.horizontal(|ui| {
        for name in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] {
            ui.add_sized([25.0, 20.0], egui::Label::new(name));
        }
    });
    for week in 0..6 {
        ui.horizontal(|ui| {
            for weekday in 0..7 {
                let day = week * 7 + weekday + 1 - first_weekday;
                if day > 0 && day <= days as i32 {
                    if ui
                        .add_sized(
                            [25.0, 20.0],
                            egui::Button::new(format!("{day:02}")).selected(date.day == day as u32),
                        )
                        .clicked()
                    {
                        date.day = day as u32;
                    }
                } else {
                    ui.allocate_space(egui::vec2(25.0, 20.0));
                }
            }
        });
    }
    if !with_time {
        return;
    }
    ui.horizontal(|ui| {
        ui.add(
            egui::DragValue::new(&mut date.hour)
                .range(0..=23)
                .suffix(" h"),
        );
        ui.add(
            egui::DragValue::new(&mut date.minute)
                .range(0..=59)
                .suffix(" m"),
        );
        ui.add(
            egui::DragValue::new(&mut date.second)
                .range(0..=59)
                .suffix(" s"),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        ctx: &egui::Context,
        column: &Column,
        draft: &mut FilterDraft,
        events: Vec<egui::Event>,
    ) -> Vec<(String, egui::Pos2)> {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                show(
                    ui,
                    Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(240.0, 26.0)),
                    Id::new("filter-test"),
                    column,
                    draft,
                );
            },
        );
        fn collect(shape: &egui::epaint::Shape, texts: &mut Vec<(String, egui::Pos2)>) {
            match shape {
                egui::epaint::Shape::Text(text) => {
                    texts.push((text.galley.text().into(), text.pos + egui::vec2(3.0, 4.0)))
                }
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, texts);
                    }
                }
                _ => {}
            }
        }
        let mut texts = Vec::new();
        for shape in &output.shapes {
            collect(&shape.shape, &mut texts);
        }
        output.textures_delta.clear();
        texts
    }
    fn click(
        ctx: &egui::Context,
        column: &Column,
        draft: &mut FilterDraft,
        pos: egui::Pos2,
    ) -> Vec<(String, egui::Pos2)> {
        frame(ctx, column, draft, vec![egui::Event::PointerMoved(pos)]);
        frame(
            ctx,
            column,
            draft,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
        );
        frame(
            ctx,
            column,
            draft,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        frame(ctx, column, draft, vec![])
    }

    #[test]
    fn money_can_be_typed_directly_in_both_inline_inputs() {
        let ctx = egui::Context::default();
        let column =
            Column::new(erp_grid_core::ColumnId(3), "Price", 240.0).with_filter(FilterKind::Money);
        let mut draft = FilterDraft::for_kind(&column.filter);
        frame(&ctx, &column, &mut draft, vec![]);
        click(&ctx, &column, &mut draft, egui::pos2(24.0, 22.0));
        frame(
            &ctx,
            &column,
            &mut draft,
            vec![egui::Event::Text("12.34".into())],
        );
        click(&ctx, &column, &mut draft, egui::pos2(160.0, 22.0));
        frame(
            &ctx,
            &column,
            &mut draft,
            vec![egui::Event::Text("56.78".into())],
        );
        assert_eq!(
            draft,
            FilterDraft::Money {
                min: "12.34".into(),
                max: "56.78".into()
            }
        );
    }

    #[test]
    fn date_and_enum_popups_really_update_typed_drafts() {
        let ctx = egui::Context::default();
        let date = Column::new(erp_grid_core::ColumnId(1), "Date", 240.0)
            .with_filter(FilterKind::DateTime);
        let mut draft = FilterDraft::for_kind(&date.filter);
        frame(&ctx, &date, &mut draft, vec![]);
        let labels = click(&ctx, &date, &mut draft, egui::pos2(24.0, 22.0));
        let start = labels
            .iter()
            .find(|(label, _)| label == "Start")
            .expect("date popup must expose start picker")
            .1;
        click(&ctx, &date, &mut draft, start);
        assert!(matches!(
            draft,
            FilterDraft::DateTime { start: Some(_), .. }
        ));

        let ctx = egui::Context::default();
        let column = Column::new(erp_grid_core::ColumnId(2), "State", 240.0).with_filter(
            FilterKind::categorical(["Ready".into(), "Hold".into()], true),
        );
        let mut draft = FilterDraft::for_kind(&column.filter);
        frame(&ctx, &column, &mut draft, vec![]);
        let labels = click(&ctx, &column, &mut draft, egui::pos2(24.0, 22.0));
        let ready = labels
            .iter()
            .find(|(label, _)| label == "Ready")
            .expect("enum popup must expose category values")
            .1;
        click(&ctx, &column, &mut draft, ready);
        assert_eq!(draft, FilterDraft::Enum(["Ready".into()].into()));
    }

    #[test]
    fn none_draws_no_control_and_never_changes_a_draft() {
        let ctx = egui::Context::default();
        let column = Column::new(erp_grid_core::ColumnId(9), "No filter", 240.0)
            .with_filter(FilterKind::None);
        let mut draft = FilterDraft::Text("stale".into());
        let before = draft.clone();
        let labels = frame(
            &ctx,
            &column,
            &mut draft,
            vec![egui::Event::Text("x".into())],
        );
        assert!(labels.is_empty());
        assert_eq!(draft, before);
    }
}
