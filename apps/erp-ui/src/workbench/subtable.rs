use eframe::egui;

#[derive(Default)]
pub struct SubtableTabs {
    selected: Option<String>,
}
impl SubtableTabs {
    pub fn show<'a>(&mut self, ui: &mut egui::Ui, entries: &'a [(&str, &str)]) -> Option<&'a str> {
        if !entries
            .iter()
            .any(|(id, _)| Some(*id) == self.selected.as_deref())
        {
            self.selected = entries.first().map(|(id, _)| (*id).into());
        }
        ui.horizontal(|ui| {
            for (id, title) in entries {
                if ui
                    .selectable_label(self.selected.as_deref() == Some(*id), *title)
                    .clicked()
                {
                    self.selected = Some((*id).into());
                }
            }
        });
        entries
            .iter()
            .find(|(id, _)| Some(*id) == self.selected.as_deref())
            .map(|(id, _)| *id)
    }
}
