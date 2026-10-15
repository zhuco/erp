use egui::{Id, Rect, Ui};
use erp_grid_core::{CellEdit, Column, ColumnId, RowId};
pub(crate) struct Editor {
    pub picker: crate::entity_picker::Picker,
    pub ticket: u64,
    pub row: RowId,
    pub column: ColumnId,
    pub before: String,
    pub draft: String,
    pub first: bool,
    pub composing: bool,
    pub focused: bool,
    pub awaiting: bool,
    pub retry_required: bool,
    pub external_conflict: bool,
}
#[derive(Default)]
pub(crate) struct Outcome {
    pub finished: bool,
    pub edit: Option<CellEdit>,
    pub error: Option<String>,
    pub navigation: Option<super::EditNavigation>,
}
pub(crate) fn show(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    column: &Column,
    editor: &mut Editor,
) -> Outcome {
    if editor.awaiting {
        ui.put(rect, egui::Label::new(&editor.draft))
            .on_hover_text("Waiting for save result");
        return Outcome::default();
    }
    ui.input(|input| {
        for event in &input.events {
            if let egui::Event::Ime(event) = event {
                match event {
                    egui::ImeEvent::Preedit { text, .. } => editor.composing = !text.is_empty(),
                    egui::ImeEvent::Commit(_) => editor.composing = false,
                    _ => {}
                }
            }
        }
    });
    if !editor.composing
        && (editor.focused || ui.memory(|memory| memory.has_focus(id)))
        && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        ui.memory_mut(|memory| memory.surrender_focus(id));
        return Outcome {
            finished: true,
            ..Default::default()
        };
    }
    // Consume before TextEdit moves focus to an unrelated widget.
    let navigation = if !editor.composing && ui.memory(|m| m.has_focus(id)) {
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab) {
                Some(true)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Tab) {
                Some(false)
            } else {
                None
            }
        })
    } else {
        None
    };
    let mut enter = !editor.composing
        && ui.memory(|memory| memory.has_focus(id))
        && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    let entity = matches!(column.edit_kind, erp_grid_core::EditKind::Entity);
    let has_picker = entity
        || matches!(
            column.edit_kind,
            erp_grid_core::EditKind::Choice(_)
                | erp_grid_core::EditKind::Date
                | erp_grid_core::EditKind::DateTime
        );
    let text_rect = if has_picker {
        Rect::from_min_max(
            rect.min,
            egui::pos2((rect.right() - 25.0).max(rect.left()), rect.bottom()),
        )
    } else {
        rect
    };
    let was_first = editor.first;
    let response = ui.put(
        text_rect.shrink(1.0),
        egui::TextEdit::singleline(if entity {
            &mut editor.picker.query
        } else {
            &mut editor.draft
        })
        .id(id)
        .char_limit(4096),
    );
    if editor.first {
        response.request_focus();
        let mut text_state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        text_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(if entity {
                    editor.picker.query.chars().count()
                } else {
                    editor.draft.chars().count()
                }),
            )));
        text_state.store(ui.ctx(), id);
        editor.first = false;
    }
    if !editor.composing
        && response.has_focus()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        return Outcome {
            finished: true,
            ..Default::default()
        };
    }
    let mut picker_open = false;
    if has_picker {
        let now = ui.input(|input| input.time);
        if entity {
            if response.changed() {
                editor.picker.invalidate(now);
            }
            editor.picker.poll(
                crate::EntitySearch {
                    instance: id,
                    ticket: editor.ticket,
                    row: editor.row,
                    column: editor.column,
                    revision: 0,
                    search: String::new(),
                },
                now,
                editor.composing,
            );
            if !editor.composing && response.has_focus() && !editor.picker.options.is_empty() {
                ui.input_mut(|input| {
                    if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                        editor.picker.cursor =
                            (editor.picker.cursor + 1).min(editor.picker.options.len() - 1);
                    }
                    if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                        editor.picker.cursor = editor.picker.cursor.saturating_sub(1);
                    }
                });
                if enter && !editor.picker.selected {
                    if let Some(value) = editor.picker.choose(editor.picker.cursor) {
                        editor.draft = value;
                    }
                    enter = false;
                }
            }
        }
        let selector = ui.put(
            Rect::from_min_max(egui::pos2(text_rect.right(), rect.top()), rect.max),
            egui::Button::new("▾"),
        );
        let popup_id = id.with("picker");
        if entity && (was_first || response.changed()) {
            egui::Popup::open_id(ui.ctx(), popup_id);
        }
        let popup = egui::Popup::menu(&selector)
            .id(popup_id)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        picker_open = popup.is_open() || selector.clicked();
        popup.show(|ui| {
            ui.set_min_width(220.0);
            match &column.edit_kind {
                erp_grid_core::EditKind::Choice(options) => {
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            for value in options.iter().take(100) {
                                if ui.selectable_label(editor.draft == *value, value).clicked() {
                                    editor.draft.clone_from(value);
                                    response.request_focus();
                                    ui.close();
                                }
                            }
                            if options.len() > 100 {
                                ui.label("最多显示100项；更多选项请使用对象搜索");
                            }
                        });
                }
                erp_grid_core::EditKind::Date | erp_grid_core::EditKind::DateTime => {
                    let mut date =
                        erp_grid_core::DateTimeParts::parse(&editor.draft).unwrap_or_default();
                    let before = date;
                    let with_time = matches!(column.edit_kind, erp_grid_core::EditKind::DateTime);
                    let _ = crate::quick_search::calendar(ui, &mut date, with_time);
                    let confirm = ui.button("选择").clicked();
                    if date != before || confirm {
                        editor.draft = if with_time {
                            date.label()
                        } else {
                            date.label()[..10].into()
                        };
                    }
                    if confirm {
                        response.request_focus();
                        ui.close();
                    }
                }
                erp_grid_core::EditKind::Entity => {
                    if let Some(error) = &editor.picker.error {
                        ui.label(error);
                    }
                    if editor.picker.loading {
                        ui.label("正在查询");
                    } else if editor.picker.options.is_empty() {
                        ui.label("无候选项；输入关键词搜索");
                    }
                    let mut selected = None;
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            for (index, option) in editor.picker.options.iter().enumerate() {
                                if ui
                                    .selectable_label(editor.picker.cursor == index, &option.label)
                                    .clicked()
                                {
                                    selected = Some(index);
                                }
                            }
                        });
                    if let Some(index) = selected
                        && let Some(value) = editor.picker.choose(index)
                    {
                        editor.draft = value;
                        response.request_focus();
                        ui.close();
                    }
                    if ui.button("重试查询").clicked() {
                        editor.picker.invalidate(now);
                    }
                }
                _ => {}
            }
        });
        if entity && editor.picker.debounce_pending() && !editor.composing {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
    let commit = navigation.is_some()
        || enter
        || (!entity && !picker_open && !editor.retry_required && response.lost_focus());
    editor.focused = response.has_focus();
    if commit && !editor.composing {
        if entity && !editor.picker.selected {
            response.request_focus();
            return Outcome {
                error: Some("Choose an object before saving".into()),
                ..Default::default()
            };
        }
        if editor.external_conflict {
            response.request_focus();
            return Outcome {
                error: Some("Row changed; review the latest version before saving".into()),
                ..Default::default()
            };
        }
        if let Err(error) = column.edit_kind.validate(&editor.draft) {
            response.request_focus();
            editor.focused = true;
            return Outcome {
                error: Some(error.into()),
                ..Default::default()
            };
        }
        let edit = (editor.before != editor.draft).then(|| CellEdit {
            row: editor.row,
            column: editor.column,
            before: editor.before.clone(),
            after: editor.draft.clone(),
        });
        editor.awaiting = edit.is_some();
        return Outcome {
            finished: edit.is_none(),
            edit,
            error: None,
            navigation: navigation.map(|backwards| super::EditNavigation {
                row: editor.row,
                column: editor.column,
                backwards,
            }),
        };
    }
    Outcome::default()
}
